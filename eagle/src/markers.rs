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
use rerun::external::glam::Vec2;
pub struct Detector {
    detector: ArucoDetector,
    markers: HashMap<i32, Marker>,
}

pub struct Marker {
    confidence: u8,
    points: Vec<Vec2>,
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

    pub fn detect(&mut self, color_data: &[u8], width: usize, height: usize) -> Option<[Vec2; 4]> {
        let mut corners: Vector<Vector<Point2f>> = Vector::new();
        let mut ids: Vector<i32> = Vector::new();
        let mut rejected = no_array();

        let mut color_vec = Vec::new();
        for chunk in color_data.chunks(3) {
            let gray: i32 = chunk.iter().map(|&n| n as i32).sum();
            color_vec.push((gray / 3) as u8);
        }
        let mat = Mat::new_rows_cols_with_data(height as i32, width as i32, &color_vec).unwrap();

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
                .map(|point| Vec2::new(point.x, point.y))
                .collect();
            marker.confidence = (marker.confidence + 5).min(20);
        }
        for marker in self.markers.values_mut() {
            marker.confidence -= 1;
        }
        self.markers.retain(|_, marker| marker.confidence > 0);

        let markers: Vec<_> = self
            .markers
            .iter()
            .filter(|(_, marker)| marker.confidence > 10)
            .map(|(&id, marker)| (id, marker.points.clone()))
            .collect();

        Self::order_points(&markers)
    }

    fn order_points(markers: &[(i32, Vec<Vec2>)]) -> Option<[Vec2; 4]> {
        if markers.len() != 4 {
            return None;
        }

        let mid_points: Vec<_> = markers.iter().map(|(_id, pts)| mean(pts)).collect();
        let mid_point = mean(&mid_points);

        let mut top_left = None;
        let mut top_right = None;
        let mut bottom_left = None;
        let mut bottom_right = None;

        for pt in mid_points {
            match (pt.x > mid_point.x, pt.y > mid_point.y) {
                (true, false) => top_right = Some(pt),
                (true, true) => bottom_right = Some(pt),
                (false, false) => top_left = Some(pt),
                (false, true) => bottom_left = Some(pt),
            }
        }

        Some([top_right?, top_left?, bottom_left?, bottom_right?])
    }
}

fn mean(pts: &[Vec2]) -> Vec2 {
    let sumx: f32 = pts.iter().map(|p| p.x).sum();
    let sumy: f32 = pts.iter().map(|p| p.y).sum();
    let meanx = sumx / pts.len() as f32;
    let meany = sumy / pts.len() as f32;
    Vec2::new(meanx, meany)
}
