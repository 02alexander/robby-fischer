use std::collections::HashMap;

use anyhow::Result;
use opencv::{
    core::{no_array, Point2f, Vector},
    objdetect::{
        get_predefined_dictionary, ArucoDetector, DetectorParameters, PredefinedDictionaryType,
        RefineParameters,
    },
    prelude::{ArucoDetectorTraitConst, Mat},
};

pub struct Detector {
    detector: ArucoDetector,
    markers: HashMap<i32, Marker>,
}

pub struct Marker {
    confidence: u8,
    points: Vec<(f32, f32)>,
}

impl Detector {
    pub fn new() -> Result<Self> {
        let aruco_dict = get_predefined_dictionary(PredefinedDictionaryType::DICT_4X4_50).unwrap();
        let detec_params = DetectorParameters::default().unwrap();
        let detector = ArucoDetector::new(
            &aruco_dict,
            &detec_params,
            RefineParameters::new(10., 6., true).unwrap(),
        )
        .unwrap();
        Ok(Self {
            detector,
            markers: HashMap::new(),
        })
    }

    pub fn detect(
        &mut self,
        color_data: &[u8],
        width: usize,
        height: usize,
    ) -> Vec<(i32, Vec<(f32, f32)>)> {
        let mut corners: Vector<Vector<Point2f>> = Vector::new();
        let mut ids: Vector<i32> = Vector::new();
        let mut rejected = no_array();

        let mut color_vec = Vec::new();
        for chunk in color_data.chunks(3) {
            let gray: i32 = chunk.iter().map(|&n| n as i32).sum();
            color_vec.push((gray / 3) as u8);
        }
        let mat = Mat::from_slice_rows_cols(&color_vec, height, width).unwrap();

        self.detector
            .detect_markers(&mat, &mut corners, &mut ids, &mut rejected)
            .unwrap();

        for (id, corners) in ids.into_iter().zip(corners) {
            let marker = self.markers.entry(id).or_insert_with(|| Marker {
                confidence: 0,
                points: Vec::new(),
            });

            marker.points = corners
                .into_iter()
                .map(|point| (point.x, point.y))
                .collect();
            marker.confidence = (marker.confidence + 5).min(20);
        }
        for marker in self.markers.values_mut() {
            marker.confidence -= 1;
        }
        self.markers.retain(|_, marker| marker.confidence > 0);

        self.markers
            .iter()
            .filter(|(_, marker)| marker.confidence > 10)
            .map(|(&id, marker)| (id, marker.points.clone()))
            .collect()
    }
}
