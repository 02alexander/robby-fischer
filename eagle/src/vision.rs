
#[cfg(feature = "vis")]
use std::sync::Mutex;

use freenectrs::freenect::{
    FreenectContext, FreenectDepthFormat, FreenectDepthStream, FreenectResolution,
    FreenectVideoFormat, FreenectVideoStream,
};
use glam::{Mat3, Vec2, Vec3};
use image::{GrayImage, ImageBuffer, Luma, Pixel, Rgb, RgbImage};
#[cfg(feature = "vis")]
use once_cell::sync::Lazy;
use opencv::{
    calib3d::project_points_def,
    core as cvvec,
    core::{convert_scale_abs_def, BORDER_DEFAULT, CV_16S},
    imgproc::create_clahe,
    imgproc::{gaussian_blur_def, sobel},
    prelude::{CLAHETrait, DataType, MatTraitConst},
};
#[cfg(feature = "vis")]
use rerun::{components::Position2D, Points2D, RecordingStream};

use crate::Detector;

const KINECT_WIDTH: usize = 640;
const KINECT_HEIGHT: usize = 480;

#[cfg(feature = "vis")]
static REC: Lazy<Mutex<RecordingStream>> = Lazy::new(|| {
    Mutex::new(
        rerun::RecordingStreamBuilder::new("rerun_example_app")
            .connect()
            .unwrap(),
    )
});

fn mat_to_image<P>(inp_arr: &cvvec::Mat) -> ImageBuffer<P, Vec<P::Subpixel>>
where
    P: Pixel,
    P::Subpixel: DataType,
{
    let data: Vec<_> = inp_arr.iter().unwrap().map(|(_, v)| v).collect();
    ImageBuffer::from_vec(inp_arr.cols() as u32, inp_arr.rows() as u32, data).unwrap()
}

fn highlight_edges(color_img: &RgbImage) -> (RgbImage, RgbImage) {
    let mut clahe = create_clahe(4.0, (8, 8).into()).unwrap();

    let mut thresh_img = color_img.clone();
    let mut clahe_img = color_img.clone();

    for channel in 0..2 {
        let mut image = GrayImage::new(color_img.width(), color_img.height());
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            pixel.0[0] = color_img[(x, y)].0[channel];
        }

        let inp_arr =
            cvvec::Mat::from_slice_rows_cols(&image, KINECT_HEIGHT, KINECT_WIDTH).unwrap();
        let mut clahed_mat = inp_arr.clone();
        clahe.apply(&inp_arr, &mut clahed_mat).unwrap();

        let clahe_grayimg = mat_to_image::<Luma<u8>>(&clahed_mat);

        #[cfg(feature = "vis")]
        REC.lock()
            .unwrap()
            .log(
                format!("images/clahed{channel}"),
                &rerun::Image::try_from(clahe_grayimg.clone()).unwrap(),
            )
            .unwrap();

        let mut blurred_clahed_mat = cvvec::Mat::default();
        gaussian_blur_def(&clahed_mat, &mut blurred_clahed_mat, (3, 3).into(), 4.0).unwrap();

        let mut sobel_mat: cvvec::Mat = cvvec::Mat::default();

        // sobel_def(&out_arr, &mut edges, 4, , dy)
        sobel(
            &blurred_clahed_mat,
            &mut sobel_mat,
            CV_16S,
            0,
            1,
            1,
            1.,
            0.,
            BORDER_DEFAULT,
        )
        .unwrap();
        // canny_def(&out_arr, &mut edges, 200.0, 250.0).unwrap();

        let mut scaled_sobel = cvvec::Mat::default();
        convert_scale_abs_def(&sobel_mat, &mut scaled_sobel).unwrap();

        let threshold = [15.0, 15.0, 50.0][channel];
        let threshed_mat = cvvec::Mat::from_slice_rows_cols(
            &(scaled_sobel
                .iter()
                .unwrap()
                .map(|(_, t): (_, u8)| if t as f32 > threshold { 255 } else { 0 })
                .collect::<Vec<u8>>()),
            scaled_sobel.rows() as usize,
            scaled_sobel.cols() as usize,
        )
        .unwrap();
        let threshed_img: GrayImage = mat_to_image(&threshed_mat);

        for (x, y, pixel) in thresh_img.enumerate_pixels_mut() {
            pixel.0[channel] = threshed_img[(x, y)].0[0];
        }
        for (x, y, pixel) in clahe_img.enumerate_pixels_mut() {
            pixel.0[channel] = clahe_grayimg[(x, y)].0[0];
        }
    }
    for img in [&mut thresh_img, &mut clahe_img] {
        for pixel in img.pixels_mut() {
            pixel.0[2] = 0;
        }
    }

    (thresh_img, clahe_img)
}

struct CoordConverter {
    camera_matrix: cvvec::Mat,
    dist_coeffs: cvvec::Vector<f32>,
    rvec: cvvec::Vector<f32>,
    tvec: cvvec::Vector<f32>,
}

impl CoordConverter {
    pub fn from_markers(markers: [Vec2; 4]) -> Option<Self> {
        let color_param: Mat3 = Mat3::from_cols(
            Vec3::new(521.0466, 0.0, 0.0),
            Vec3::new(0., 520.1939, 0.0),
            Vec3::new(316.77554, 258.1415, 1.),
        );

        let camera_matrix: cvvec::Mat = cvvec::Mat::from_slice_2d(&[
            &color_param.row(0).to_array(),
            &color_param.row(1).to_array(),
            &color_param.row(2).to_array(),
        ])
        .unwrap();

        let color_dist_coeffs: Vec<f32> =
            vec![0.2408255, -0.6778162, 0.00130271, 0.00447125, 0.6010201];
        let dist_coeffs: cvvec::Vector<f32> = color_dist_coeffs.clone().into();

        let object_points: cvvec::Vector<cvvec::Point3d> = vec![
            (-0.4, -0.4, 0.2).into(),
            (8.4, -0.4, 0.2).into(),
            (8.4, 8.4, 0.2).into(),
            (-0.4, 8.4, 0.2).into(),
        ]
        .into();

        let image_points: cvvec::Vector<cvvec::Point2d> = markers
            .into_iter()
            .map(|p| (p.x as f64, p.y as f64).into())
            .collect();

        let mut rvec: cvvec::Vector<f32> = cvvec::Vector::new();
        let mut tvec: cvvec::Vector<f32> = cvvec::Vector::new();
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
            return None;
        }

        Some(CoordConverter {
            camera_matrix,
            dist_coeffs,
            rvec,
            tvec,
        })
    }

    pub fn project_points(&self, points: &[Vec3]) -> Vec<Vec2> {
        let object_points: cvvec::Vector<cvvec::Point3f> = points
            .iter()
            .map(|point| (point.x, point.y, point.z).into())
            .collect();
        let mut image_points = cvvec::Vector::<cvvec::Point2f>::new();
        project_points_def(
            &object_points,
            &self.rvec,
            &self.tvec,
            &self.camera_matrix,
            &self.dist_coeffs,
            &mut image_points,
        )
        .unwrap();

        image_points
            .into_iter()
            .map(|point| (point.x, point.y).into())
            .collect()
    }
}

struct Kinect<'a, 'b> {
    dstream: FreenectDepthStream<'a, 'b>,
    vstream: FreenectVideoStream<'a, 'b>,
}

impl<'a, 'b> Kinect<'a, 'b> {
    pub fn new(context: &'a FreenectContext) -> Kinect<'a, 'b>
    where
        'a: 'b,
    {
        let device = context.open_device(0).unwrap();
        // Setup mode for this device
        device
            .set_depth_mode(FreenectResolution::Medium, FreenectDepthFormat::MM)
            .unwrap();
        device
            .set_video_mode(FreenectResolution::Medium, FreenectVideoFormat::Rgb)
            .unwrap();

        let device = Box::leak(Box::new(device));

        // Get rgb and depth stream
        let dstream = device.depth_stream().unwrap();
        let vstream = device.video_stream().unwrap();
        // Start the main-loop-thread

        Kinect { dstream, vstream }
    }

    pub fn receive(&mut self) -> (Vec<u16>, Vec<u8>) {
        (
            self.dstream.receiver.recv().unwrap().0.to_vec(),
            self.vstream.receiver.recv().unwrap().0.to_vec(),
        )
    }
}

fn count_in_frustum(
    point: Vec3,
    inside_board: bool,
    coord_converter: &CoordConverter,
    threshed_img: &RgbImage,
    clahed_img: &RgbImage,
    mask_img: &mut GrayImage,
) -> (u32, Vec2, Vec<u8>) {
    let bottom_xoffset = 0.1;
    let bottom_yoffset = 0.2;
    let top_xoffset = -0.2;
    let top_yoffset = -0.2;
    let top_left_xoffset = if inside_board {
        top_xoffset
    } else {
        top_xoffset - 0.8
    };
    let object_points: Vec<Vec3> = vec![
        point + Vec3::from((-0.5 + bottom_xoffset, -0.5 + bottom_yoffset, 0.0)),
        point + Vec3::from((-0.5 + bottom_xoffset, 0.5 - bottom_yoffset, 0.0)),
        point + Vec3::from((0.5 - bottom_xoffset, 0.5 - bottom_yoffset, 0.0)),
        point + Vec3::from((0.5 - bottom_xoffset, -0.5 + bottom_yoffset, 0.0)),
        point + Vec3::from((-0.5 + top_left_xoffset, -0.5 + top_yoffset, 1.6)),
        point + Vec3::from((-0.5 + top_left_xoffset, 0.5 - top_yoffset, 1.6)),
        point + Vec3::from((0.5 - top_xoffset, 0.5 - top_yoffset, 1.6)),
        point + Vec3::from((0.5 - top_xoffset, -0.5 + top_yoffset, 1.6)),
        point + Vec3::from((0.0, 0.0, 0.0)),
    ];

    let image_points = coord_converter.project_points(&object_points);

    let bounding_box = bounding_box(&image_points);

    let mut cnt = 0;
    let mut intensities = Vec::new();

    for x in bounding_box[0]..bounding_box[1] {
        for y in bounding_box[2]..bounding_box[3] {
            if inside_convex_polygon((x as f32, y as f32).into(), &image_points[0..4])
                && inside_convex_polygon((x as f32, y as f32).into(), &image_points[4..8])
            {
                mask_img[(x, y)] = [255].into();
                for channel in 0..3 {
                    if threshed_img[(x, y)].0[channel] == 255 {
                        cnt += 1
                    }
                }
                intensities.push(clahed_img[(x, y)].0[1]);
            }
        }
    }
    let mid_point = image_points[8];
    intensities.sort_unstable();

    (cnt, mid_point, intensities)
}

fn bounding_box(polygon: &[Vec2]) -> [u32; 4] {
    let minx = polygon.iter().map(|p| p.x as u32).min().unwrap();
    let maxx = polygon.iter().map(|p| p.x as u32).max().unwrap() + 1;
    let miny = polygon.iter().map(|p| p.y as u32).min().unwrap();
    let maxy = polygon.iter().map(|p| p.y as u32).max().unwrap() + 1;
    [minx, maxx, miny, maxy]
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
pub struct Vision {
    kinect: Kinect<'static, 'static>,
    detector: Detector,
    count_avg: Vec<f32>,
}

impl Vision {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let freenect = FreenectContext::init_with_video().unwrap();
        let freenect = Box::leak(Box::new(freenect));
        freenect.spawn_process_thread().unwrap();

        let kinect = Kinect::new(freenect);
        let detector = Detector::new().unwrap();

        Vision {
            kinect,
            detector,
            count_avg: vec![0.0; 72],
        }
    }

    pub fn pieces(&mut self) -> Option<Vec<Option<bool>>> {
        let (_, data) = self.kinect.receive();

        let color_img: ImageBuffer<Rgb<_>, _> =
            ImageBuffer::from_vec(KINECT_WIDTH as u32, KINECT_HEIGHT as u32, data.to_vec())
                .unwrap();
        #[cfg(feature = "vis")]
        REC.lock()
            .unwrap()
            .log(
                "images/image",
                &rerun::Image::try_from(color_img.clone()).unwrap(),
            )
            .unwrap();

        let marks = self.detector.detect(&data, 640, 480)?;

        let (threshed_img, clahed_img) = highlight_edges(&color_img);

        let coord_converter = CoordConverter::from_markers(marks).unwrap();

        let mut mask = GrayImage::from_fn(KINECT_HEIGHT as u32, KINECT_HEIGHT as u32, |_, _| {
            [255].into()
        });
        mask.iter_mut().for_each(|p| *p = 0);

        #[cfg(feature = "vis")]
        let mut square_mid_points: Vec<Position2D> = Vec::new();

        let mut square_intensities = Vec::new();
        for rank in 0..8 {
            for file in 0..8 {
                let (count, mid_point, intensities) = count_in_frustum(
                    Vec3::new(file as f32 + 0.5, rank as f32 + 0.5, 0.0),
                    true,
                    &coord_converter,
                    &threshed_img,
                    &clahed_img,
                    &mut mask,
                );
                self.count_avg[file + rank * 8] =
                    0.9 * self.count_avg[file + rank * 8] + 0.1 * count as f32;

                let _ = &mid_point;
                #[cfg(feature = "vis")]
                square_mid_points.push(Position2D::new(mid_point.x, mid_point.y));

                square_intensities.push(intensities);
            }
        }
        for rank in 0..8 {
            let fudge_factor = if rank >= 4 { 0.1 } else { -0.1 };
            let (count, mid_point, intensities) = count_in_frustum(
                Vec3::new(9.3, rank as f32 + 0.5 + fudge_factor, -0.1),
                false,
                &coord_converter,
                &threshed_img,
                &clahed_img,
                &mut mask,
            );
            self.count_avg[64 + rank] = 0.9 * self.count_avg[64 + rank] + 0.1 * count as f32;

            let _ = &mid_point;
            #[cfg(feature = "vis")]
            square_mid_points.push(Position2D::new(mid_point.x, mid_point.y));

            square_intensities.push(intensities);
        }

        let with_pieces: Vec<_> = self.count_avg.iter().map(|&count| count > 70.0).collect();

        let is_white: Vec<_> = square_intensities
            .iter()
            .enumerate()
            .map(|(i, values)| {
                let white_square = if i < 64 {
                    let rank = i / 8;
                    let file = i % 8;
                    (rank + file) % 2 == 1
                } else {
                    true
                };
                let i1 = values.len() / 10;
                let i2 = values.len() * 9 / 10;
                let wrong_color = values[i2] - values[i1] > 140;
                white_square ^ wrong_color
            })
            .collect();

        #[cfg(feature = "vis")]
        {
            let rec = REC.lock().unwrap();
            // for rank in 0..8 {
            //     for file in 0..8 {
            //         if with_pieces[rank*8+file] {
            //             rec.log(format!("plot{rank}_{file}"), &BarChart::new(square_intensities[rank*8+file].as_slice())).unwrap();
            //         }
            //     }
            // }

            rec.log("images/mask", &rerun::Image::try_from(mask).unwrap())
                .unwrap();
            rec.log(
                "images/thrseshed image",
                &rerun::Image::try_from(threshed_img).unwrap(),
            )
            .unwrap();
            rec.log("images/image", &rerun::Image::try_from(color_img).unwrap())
                .unwrap();
            rec.log(
                "images/points",
                &Points2D::new(square_mid_points)
                    .with_labels(self.count_avg.iter().map(|cnt| cnt.to_string()))
                    .with_radii(with_pieces.iter().map(|b| if *b { 10.0 } else { 2.0 }))
                    .with_colors(is_white.iter().map(|&w| if w { [220; 3] } else { [50; 3] })),
            )
            .unwrap();
        }
        Some(
            is_white
                .iter()
                .zip(with_pieces)
                .map(|(&w, p)| p.then_some(w))
                .collect(),
        )
    }
}
