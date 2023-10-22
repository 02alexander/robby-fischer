use std::iter::zip;

use glam::{Mat3, Vec2, Vec3};
use minifb::{Window, WindowOptions};
use rerun::{
    components::Point3D,
    default_server_addr,
    external::image::{ImageBuffer, Rgb, GrayImage, Pixel},
};

use opencv::{prelude::{CLAHETrait, MatTraitConst, DataType}, imgproc::{canny, canny_def, sobel_def, sobel, gaussian_blur_def, median_blur}, core::{BORDER_DEFAULT, convert_scale_abs_def, CV_16S}};
use opencv::{core as cvvec, calib3d::project_points_def, imgproc::create_clahe};

const KINECT_WIDTH: usize = 640;
const KINECT_HEIGHT: usize = 480;

fn mat_to_image<P>(inp_arr: &cvvec::Mat) -> ImageBuffer<P, Vec<P::Subpixel>> 
where P: Pixel,
      P::Subpixel: DataType {
    let data: Vec<_> = inp_arr.iter().unwrap().map(|(_,v)| v).collect();
    ImageBuffer::from_vec(inp_arr.cols() as u32, inp_arr.rows() as u32, data).unwrap()
}

fn main() {
    use freenectrs::freenect;
    // Init with video functionality
    let ctx = freenect::FreenectContext::init_with_video().unwrap();
    // let ctx = freenect::FreenectContext::init_with_video_motor().unwrap(); // If we want to use the motor too
    // Open first device

    let rec_stream = rerun::RecordingStreamBuilder::new("my_app")
        .connect(default_server_addr())
        .unwrap();

    let device = ctx.open_device(0).unwrap();
    // Setup mode for this device
    device
        .set_depth_mode(
            freenect::FreenectResolution::Medium,
            freenect::FreenectDepthFormat::MM,
        )
        .unwrap();
    device
        .set_video_mode(
            freenect::FreenectResolution::Medium,
            freenect::FreenectVideoFormat::Rgb,
        )
        .unwrap();
    // Get rgb and depth stream
    let dstream = device.depth_stream().unwrap();
    let vstream = device.video_stream().unwrap();
    // Start the main-loop-thread
    ctx.spawn_process_thread().unwrap();
    // Fetch the video and depth frames

    // let mut window =
    //     Window::new("a", KINECT_WIDTH, KINECT_HEIGHT, WindowOptions::default()).unwrap();
    let mut buffer = vec![0; KINECT_WIDTH * KINECT_HEIGHT];

    // let intrinsic_param = array![
    //     [480., 0., 240.,],
    //     [0., 480., 240.],
    //     [0., 0., 1.]
    // ];

    let color_param = Mat3::from_cols(
        Vec3::new(521.04658096, 0.0, 0.0),
        Vec3::new(0., 520.19390147, 0.0),
        Vec3::new(316.77552846, 258.14152348, 1.),
    );
    let color_dist_coeffs = vec![0.24082551, -0.67781624, 0.00130271, 0.00447125, 0.60102011];

    // [[521.04658096   0.         316.77552846]
    // [  0.         520.19390147 258.14152348]
    // [  0.           0.           1.        ]]
    // [[ 0.24082551 -0.67781624  0.00130271  0.00447125  0.60102011]]

    let depth_param = Mat3::from_cols(
        Vec3::new(440., 0., 0.),
        Vec3::new(0., 440., 0.),
        Vec3::new(240., 320., 1.),
    );
    let kinv = depth_param.inverse();

    let mut detector = eagle::Detector::new().unwrap();

    loop {
        let Ok((depth_data, _)) = dstream.receiver.try_recv() else {
            continue;
        };
        // ... handle depth data

        let Ok((data, _)) = vstream.receiver.try_recv() else {
            continue;
        };

        let color_img: ImageBuffer<Rgb<_>, _> =
            rerun::external::image::ImageBuffer::from_vec(KINECT_WIDTH as u32, KINECT_HEIGHT as u32, data.to_vec()).unwrap();
        let gray_img = rerun::external::image::imageops::colorops::grayscale(&color_img);

        let mut clahe = create_clahe(4.0, (8,8).into()).unwrap();

        let inp_arr = cvvec::Mat::from_slice_rows_cols(&gray_img.to_vec(), KINECT_HEIGHT, KINECT_WIDTH).unwrap(); 
        let mut clahed_mat = inp_arr.clone();
        clahe.apply(&inp_arr, &mut clahed_mat).unwrap();

        let mut blurred_clahed_mat = cvvec::Mat::default();
        gaussian_blur_def(&clahed_mat, &mut blurred_clahed_mat, (3,3).into(), 4.0).unwrap();

        let mut sobel_mat: cvvec::Mat = cvvec::Mat::default();

        // sobel_def(&out_arr, &mut edges, 4, , dy)
        sobel(&blurred_clahed_mat, &mut sobel_mat, CV_16S, 0, 1, 1, 1., 0., BORDER_DEFAULT).unwrap();
        // canny_def(&out_arr, &mut edges, 200.0, 250.0).unwrap();

        let mut scaled_sobel = cvvec::Mat::default();
        convert_scale_abs_def(&sobel_mat, &mut scaled_sobel).unwrap();

        // let mut median_blurred_sobel_mat = cvvec::Mat::default();
        // median_blur(&scaled_sobel, &mut median_blurred_sobel_mat, 3).unwrap();

        let threshold = 20.0;
        let threshed_mat = cvvec::Mat::from_slice_rows_cols(
            &(scaled_sobel.iter().unwrap().map(|(_,t): (_, u8)| if t as f32 > threshold {255} else {0} ).collect::<Vec<u8>>()),
            scaled_sobel.rows() as usize,
            scaled_sobel.cols() as usize,
        ).unwrap();
        let threshed_img: GrayImage = mat_to_image(&threshed_mat);
        let edges_img: GrayImage = mat_to_image(&scaled_sobel);
        let blurred_clahed_img: GrayImage = mat_to_image(&blurred_clahed_mat);

        rerun::MsgSender::new("threshed image")
            .with_component(&[rerun::components::Tensor::from_image(threshed_img).unwrap()])
            .unwrap()
            .send(&rec_stream)
            .unwrap();

        rerun::MsgSender::new("blurred clahed")
            .with_component(&[rerun::components::Tensor::from_image(blurred_clahed_img).unwrap()])
            .unwrap()
            .send(&rec_stream)
            .unwrap();

        rerun::MsgSender::new("edges")
            .with_component(&[rerun::components::Tensor::from_image(edges_img).unwrap()])
            .unwrap()
            .send(&rec_stream)
            .unwrap();

        rerun::MsgSender::new("image")
            .with_component(&[rerun::components::Tensor::from_image(color_img).unwrap()])
            .unwrap()
            .send(&rec_stream)
            .unwrap();

        let Some(marks) = detector.detect(&data, 640, 480) else {
            continue;
        };
        // let p = solve_p(marks[0], marks[1], marks[2], marks[3], Vec2::new(240.0, 240.0), 0.000001, 0.0);
        // println!("{:?}", p);
        // let p = solve_p(marks[0], marks[1], marks[2], marks[3], Vec2::new(240.0, 240.0));

        let object_points: cvvec::Vector<cvvec::Point3d> = vec![
            (8.4, 8.4, 0.2).into(),
            (-0.4, 8.4, 0.2).into(),
            (-0.4, -0.4, 0.2).into(),
            (8.4, -0.4, 0.2).into(),
        ]
        .into();

        let image_points: cvvec::Vector<cvvec::Point2d> = marks
            .into_iter()
            .map(|p| (p.x as f64, p.y as f64).into())
            .collect();

        let camera_matrix: cvvec::Mat = cvvec::Mat::from_slice_2d(&[
            &color_param.row(0).to_array(),
            &color_param.row(1).to_array(),
            &color_param.row(2).to_array(),
        ])
        .unwrap();

        let dist_coeffs: cvvec::Vector<f32> = color_dist_coeffs.clone().into();

        let mut tvec: cvvec::Vector<f32> = cvvec::Vector::new();
        let mut rvec: cvvec::Vector<f32> = cvvec::Vector::new();

        if !opencv::calib3d::solve_pnp_def(
            &object_points,
            &image_points,
            &camera_matrix,
            &dist_coeffs,
            &mut rvec,
            &mut tvec,
        )
        .unwrap()
        {
            continue;
        }


        // for file in 0..8 {
        //     for rank in 0..8 {
        //         let x = file as f32+0.5;
        //         let y = rank as f32+0.5;
        //         let output = cvvec::Vector::<cvvec::Point2d>::new();
        //         let object_points: cvvec::Vector<cvvec::Point3d> = vec![
        //             (8.4, 8.4, 0.2).into(),
        //             (-0.4, 8.4, 0.2).into(),
        //             (-0.4, -0.4, 0.2).into(),
        //             (8.4, -0.4, 0.2).into(),
        //         ]
        //         .into();
        //         project_points_def(, rvec, tvec, camera_matrix, dist_coeffs, image_points)
        //     }
        // }

        println!("{:?}", tvec);
        println!("{:?}", rvec);

        let mut points = Vec::new();
        for (i, depth) in depth_data.iter().enumerate() {
            let x = i % KINECT_WIDTH;
            let y = i / KINECT_WIDTH;
            // if inside_convex_polygon(Vec2::new(x as f32, y as f32), &marks) {
            if true {
                let img_cord = Vec3::new(x as f32, y as f32, 1.);
                let real_world_cord = (kinv * img_cord) * *depth as f32;
                points.push(real_world_cord);
            }
        }

        rerun::MsgSender::new("points")
            .with_component(
                &points
                    .iter()
                    .map(|p| Point3D::new(p.x, p.y, p.z))
                    .collect::<Vec<_>>(),
            )
            .unwrap()
            .send(&rec_stream)
            .unwrap();

        // window.update_with_buffer(&buffer, 640, 480).unwrap();
    }

    ctx.stop_process_thread().unwrap();
}

fn inside_convex_polygon(pt: Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let side = (pt - polygon[0]).perp_dot(polygon[1] - polygon[0]) > 0.0;
    for i in 1..polygon.len() {
        let cur = polygon[i];
        let next = polygon[(i + 1) % polygon.len()];
        let segment = next - cur;
        let v = pt - cur;
        if (v.perp_dot(segment) > 0.0) != side {
            return false;
        }
    }
    true
}

fn solve_p(a: Vec2, b: Vec2, c: Vec2, d: Vec2, p: Vec2) -> Vec2 {
    let r = p - c;
    let s = b - c;
    let t = d - c;
    let u = a + c - b - d;
    let a = s.perp_dot(u);
    let b = r.perp_dot(u) + s.perp_dot(t);
    let c = r.perp_dot(t);

    let y1 = (-b + (b * b - 4.0 * a * c).sqrt()) / (2.0 * a);
    let y2 = (-b - (b * b - 4.0 * a * c).sqrt()) / (2.0 * a);
    let x1 = (r.x + s.x * y1) / (t.x + u.x * y1);
    let x2 = (r.x + s.x * y1) / (t.x + u.x * y1);

    dbg!((x1, y1), (x2, y2));
    // let solutions = Vec::new();
    Vec2::new(0.0, 0.0)
}

// fn solve_p(A: Vec2, B: Vec2, C: Vec2, D: Vec2, P: Vec2, alpha: f32, beta: f32) -> Vec2 {
//     let err = |p: Vec2| {
//         (P-C-(D-C)*p.x - (D-C)*p.y - (A+C-B-D)*p.x*p.y).length_squared()
//     };
//     let grad = |cur: Vec2| {
//         let fx = (D-C)+(A+C-B-D)*cur.y;
//         let fy = (B-C)+(A+C-B-D)*cur.x;
//         let e = (P-C-(D-C)*cur.x - (D-C)*cur.y - (A+C-B-D)*cur.x*cur.y);

//         -2.0*(e.x*fx + (e.y*fy))
//     };

//     let mut cur_point = Vec2::new(0.5, 0.5);
//     let mut moment = Vec2::new(0., 0.);
//     for _i in 0..20 {
//         moment = moment*beta-alpha*grad(cur_point);
//         cur_point -= moment;
//     }
//     cur_point
// }
