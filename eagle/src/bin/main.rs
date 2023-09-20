use std::iter::zip;

use minifb::{Window, WindowOptions};

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
    let dstream = device.depth_stream().unwrap();
    let _vstream = device.video_stream().unwrap();
    // Start the main-loop-thread
    ctx.spawn_process_thread().unwrap();
    // Fetch the video and depth frames

    let mut window = Window::new("a", 640, 480, WindowOptions::default()).unwrap();
    let mut buffer = vec![0; 640 * 480];

    while window.is_open() {
        if let Ok((data, _)) = dstream.receiver.try_recv() {
            // ... handle depth data
            dbg!(data.len());
            for (dst, src) in zip(&mut buffer, data) {
                *dst = *src as u32 / 2;
            }
            window.update_with_buffer(&buffer, 640, 480).unwrap();
        }
    }

    ctx.stop_process_thread().unwrap();
}
