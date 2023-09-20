use opencv::{objdetect::{PredefinedDictionaryType, get_predefined_dictionary, DetectorParameters, RefineParameters, ArucoDetector}, prelude::{ArucoDetectorTraitConst, Mat}, core::{Vector, Vec3f, VecN, Point2f, no_array}};
use rerun::{external::image::{DynamicImage, ImageBuffer, Luma}, default_server_addr, components::Point2D};


fn main() {
    
    use freenectrs::freenect;
    // Init with video functionality
    let ctx = freenect::FreenectContext::init_with_video().unwrap();
    // let ctx = freenect::FreenectContext::init_with_video_motor().unwrap(); // If we want to use the motor too
    // Open first device
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
    let _dstream = device.depth_stream().unwrap();
    let vstream = device.video_stream().unwrap();
    // Start the main-loop-thread
    ctx.spawn_process_thread().unwrap();
    
    
    let aruco_dict = get_predefined_dictionary(PredefinedDictionaryType::DICT_4X4_50).unwrap();
    let detec_params = DetectorParameters::default().unwrap();
    let detector = ArucoDetector::new(&aruco_dict, &detec_params, RefineParameters::new(10., 6., true).unwrap()).unwrap();

    // let rec_stream = rerun::RecordingStreamBuilder::new("my_app").connect(default_server_addr(), default_flush_timeout()).unwrap();
    let rec_stream = rerun::RecordingStreamBuilder::new("my_app").connect(default_server_addr()).unwrap();
    

    loop {
        let mut corners: Vector<Vector<Point2f>> = Vector::new();
        let mut ids: Vector<i32> = Vector::new();
        let mut _rejected_img_points = no_array();
    
    
        if let Ok((data, _)) = vstream.receiver.try_recv() {
            // let image = to_opencv_matrix_color(&data, 640, 480);
            let mut color_vec =  Vec::new();
            for chunk in data.chunks(3) {
                let gray: i32 = chunk.iter().map(|&n| n as i32).sum();
                color_vec.push((gray / 3) as u8);
            }
            let mat = Mat::from_slice_rows_cols(&color_vec, 480, 640).unwrap();
            // let image = Mat::from_slice_rows_cols::<>()
            detector.detect_markers(&mat, &mut corners, &mut ids, &mut _rejected_img_points).unwrap();

            let mut grayscale: ImageBuffer<Luma<_>, _> = rerun::external::image::ImageBuffer::from_vec(640, 480, color_vec).unwrap();
            let points: Vec<_> = corners.into_iter().flatten().map(|pt| Point2D { x: pt.x, y: pt.y}).collect();

            rerun::MsgSender::new("image/points")
            .with_component(&points).unwrap()
            .send(&rec_stream).unwrap();
                
            rerun::MsgSender::new("image")
                .with_component(&[rerun::components::Tensor::from_image(grayscale).unwrap()]).unwrap()
                .send(&rec_stream).unwrap();

   
        }            
    }

    

    // let a = 
}