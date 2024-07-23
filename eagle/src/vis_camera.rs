use opencv::videoio::{self, VideoCaptureTrait};

use rerun::external::image::{ImageBuffer, RgbImage};

use opencv::prelude::MatTraitConst;
use opencv::prelude::MatTraitConstManual;

pub fn vis_camera(entity_path: &str, device_id: u32) {
    let rec = rerun::RecordingStream::thread_local(rerun::StoreKind::Recording).unwrap();
    let mut cam = videoio::VideoCapture::new(device_id as i32, videoio::CAP_ANY)
        .expect("Failed to get video capture");
    let mut frame = opencv::core::Mat::default();
    loop {
        cam.read(&mut frame).unwrap();
        let data: Vec<_> = frame
            .iter::<opencv::core::VecN<u8, 3>>()
            .unwrap()
            .map(|(_, v)| v.0.into_iter().rev()) // .rev() to go from BGR to RGB.
            .flatten()
            .collect();

        let image: RgbImage =
            ImageBuffer::from_vec(frame.cols() as u32, frame.rows() as u32, data).unwrap();
        rec.log(entity_path, &rerun::Image::try_from(image).unwrap())
            .unwrap()
    }
}
