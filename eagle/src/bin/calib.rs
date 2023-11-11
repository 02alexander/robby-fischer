use image::{ImageBuffer, Rgb};

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

    // let rec_stream = rerun::RecordingStreamBuilder::new("my_app").connect(default_server_addr(), default_flush_timeout()).unwrap();
    // let rec_stream = rerun::RecordingStreamBuilder::new("my_app")
    //     .connect(default_server_addr())
    //     .unwrap();

    let mut detector = eagle::Detector::new().unwrap();

    loop {
        if let Ok((data, _)) = vstream.receiver.try_recv() {
            let _marks = detector.detect(data, 640, 480);

            let _grayscale: ImageBuffer<Rgb<_>, _> =
                ImageBuffer::from_vec(640, 480, data.to_vec()).unwrap();
            // let rr_points: Vec<_> = marks
            //     .into_iter()
            //     .flat_map(|(corners)| corners)
            //     .map(|pt| Point2D { x: pt.x, y: pt.y })
            //     .collect();

            // rerun::MsgSender::new("image/points")
            //     .with_component(&rr_points)
            //     .unwrap()
            //     .send(&rec_stream)
            //     .unwrap();

            // rerun::MsgSender::new("image")
            //     .with_component(&[rerun::components::Tensor::from_image(grayscale).unwrap()])
            //     .unwrap()
            //     .send(&rec_stream)
            //     .unwrap();
        }
    }

    // let a =
}
