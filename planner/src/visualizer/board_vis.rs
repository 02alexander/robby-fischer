use std::{
    collections::HashMap,
    fs::{read_dir, File},
    path::Path,
};

use once_cell::sync::Lazy;
use ordered_float::OrderedFloat;
use rerun::{datatypes::UVec3D, external::glam::Vec3, Mesh3D, Radius, Scale3D, Vec3D};
use stl_io::IndexedMesh;

use crate::{
    arm::BOARD_TO_ARM_TRANSFORM,
    board::Board,
    chess::{Color, Piece, Role, Square},
};

use super::gltf_logging::{load_gltf, log_node, GltfNode};

#[derive(Clone)]
pub struct PieceModelInfo {
    pub model: rerun::Mesh3D,
    pub bounding_box: BoundingBox, // (x, y, w, h)
}

pub struct BoardVisualizer {
    pub board_scene: GltfNode,
    pub holder_mesh: rerun::Mesh3D,
    pub piece_meshes: HashMap<Piece, PieceModelInfo>,
    pub board_offset: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub center: Vec3,
    pub half_size: Vec3,
}

impl PieceModelInfo {
    pub fn log(&self, rec: &rerun::RecordingStream, entity_path: &str) {
        self.bounding_box
            .log(rec, &format!("{entity_path}/bounding_box"));
        rec.log(format!("{entity_path}/mesh"), &self.model).unwrap();
    }
}

pub static BOARD_VISUALIZER: Lazy<BoardVisualizer> =
    Lazy::new(|| BoardVisualizer::new("pieces", -BOARD_TO_ARM_TRANSFORM));

pub fn board_to_real_cord(position: Square) -> Vec3 {
    let v = Board::real_world_coordinate(position.file as u32, position.rank as u32);
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

impl BoundingBox {
    pub fn from_mesh(mesh: &rerun::Mesh3D) -> Self {
        let mut center = Vec3::ZERO;
        let mut half_size = Vec3::ZERO;
        for i in 0..3 {
            let mn = mesh
                .vertex_positions
                .iter()
                .map(|pos| OrderedFloat(pos[i]))
                .min()
                .unwrap()
                .0;
            let mx = mesh
                .vertex_positions
                .iter()
                .map(|pos| OrderedFloat(pos[i]))
                .max()
                .unwrap()
                .0;
            center[i] = (mx + mn) / 2.0;
            half_size[i] = (mx - mn) / 2.0;
        }

        BoundingBox { center, half_size }
    }

    pub fn log(&self, rec: &rerun::RecordingStream, base_path: &str) {
        let center: Vec3D = self.center.into();
        let half_size: Vec3D = self.half_size.into();
        rec.log(
            base_path,
            &rerun::Boxes3D::from_centers_and_half_sizes(&[center], &[half_size])
                .with_colors(std::iter::once(rerun::components::Color::from_rgb(
                    150, 150, 150,
                )))
                .with_radii([Radius::new_scene_units(0.2)]),
        )
        .unwrap();
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        for i in 0..3 {
            let lhs_range = (
                OrderedFloat(self.center[i] - self.half_size[i]),
                OrderedFloat(self.center[i] + self.half_size[i]),
            );
            let rhs_range = (
                OrderedFloat(other.center[i] - other.half_size[i]),
                OrderedFloat(other.center[i] + other.half_size[i]),
            );
            let mut ranges = [lhs_range, rhs_range];
            ranges.sort();

            let low_range = ranges[0];
            let high_range = ranges[1];

            if !(low_range.1 > high_range.0) {
                return false;
            }
        }
        true
    }
}

fn stl_to_mesh3d(mesh: &IndexedMesh, color: impl Into<rerun::Color> + Clone) -> Mesh3D {
    let vertices: Vec<_> = mesh
        .vertices
        .iter()
        .map(|v| rerun::Position3D::new(v[0], v[1], v[2]))
        .collect();
    let mut normals = vec![Vec::<Vec3>::new(); vertices.len()];
    for face in &mesh.faces {
        for idx in face.vertices {
            let v: [f32; 3] = face.normal.into();
            normals[idx].push(v.into());
        }
    }
    let avg_normals = normals.iter().map(|normals| {
        (normals.iter().fold(Vec3::ZERO, |acc, v| acc + *v) / normals.len() as f32).normalize()
    });

    rerun::Mesh3D::new(vertices)
        .with_triangle_indices(mesh.faces.iter().map(|face| {
            rerun::TriangleIndices(UVec3D::new(
                face.vertices[0] as u32,
                face.vertices[1] as u32,
                face.vertices[2] as u32,
            ))
        }))
        .with_vertex_colors(std::iter::repeat(color).take(mesh.vertices.len()))
        .with_vertex_normals(avg_normals)
}

impl BoardVisualizer {
    pub fn new(mesh_folder_path: impl AsRef<Path>, board_offset: Vec3) -> Self {
        let mut board_scene = None;
        let mut holder_mesh = None;
        let mut piece_meshes = HashMap::new();
        let name_to_role = HashMap::from([
            ("queen", Role::Queen),
            ("king", Role::King),
            ("pawn", Role::Pawn),
            ("knight", Role::Knight),
            ("bishop", Role::Bishop),
            ("rook", Role::Rook),
        ]);
        for entry in read_dir(mesh_folder_path).unwrap() {
            let entry = entry.unwrap();
            let file_name = entry.file_name().clone();
            let file_name_without_ext = file_name.to_str().unwrap().split('.').next().unwrap();

            if file_name_without_ext == "board" {
                let (doc, buffers, _) =
                    gltf::import_slice(bytes::Bytes::from(std::fs::read(entry.path()).unwrap()))
                        .unwrap();
                let mut nodes = load_gltf(&doc, &buffers);
                board_scene = Some(nodes.next().unwrap());
            } else if file_name_without_ext == "platta" {
                let stl_mesh = stl_io::read_stl(&mut File::open(entry.path()).unwrap()).unwrap();
                let platta = stl_to_mesh3d(&stl_mesh, 0xFFFFFFFF);
                holder_mesh = Some(platta);
            } else if let Some(role) = name_to_role.get(file_name_without_ext) {
                let stl_mesh = stl_io::read_stl(&mut File::open(entry.path()).unwrap()).unwrap();
                let white_mesh = stl_to_mesh3d(&stl_mesh, 0xFFFFFFFF);
                let black_mesh = stl_to_mesh3d(&stl_mesh, 0x202020FF);
                let bounding_box = BoundingBox::from_mesh(&white_mesh);

                piece_meshes.insert(
                    Piece::new(Color::White, *role),
                    PieceModelInfo {
                        model: white_mesh,
                        bounding_box,
                    },
                );

                piece_meshes.insert(
                    Piece::new(Color::Black, *role),
                    PieceModelInfo {
                        model: black_mesh,
                        bounding_box,
                    },
                );
            } else {
                eprintln!("Unknown piece file: {:?}", entry.path());
            }
        }
        let mut board_visulizer = BoardVisualizer {
            // all_pieces,
            holder_mesh: holder_mesh.unwrap(),
            board_scene: board_scene.unwrap(),
            board_offset,
            piece_meshes,
        };
        board_visulizer.init_logging(
            &rerun::RecordingStream::thread_local(rerun::StoreKind::Recording).unwrap(),
        );
        board_visulizer
    }

    pub fn init_logging(&mut self, rec: &rerun::RecordingStream) {
        // relative to middle of A8 square.
        let board_center = Vec3::new(0.175, 0.175, -0.035);
        let board_center_to_holder_center = Vec3::new(0.0, 7.0 * 0.05 + 0.041, 0.02);

        rec.log(
            "a8origin/board",
            &rerun::Transform3D::from_translation(Into::<[f32; 3]>::into(board_center)),
        )
        .unwrap();
        log_node(rec, "a8origin/board", self.board_scene.clone()).unwrap();
        rec.log(
            "a8origin/holder",
            &rerun::Transform3D::from_translation(Into::<[f32; 3]>::into(
                board_center + board_center_to_holder_center,
            )),
        )
        .unwrap();
        rec.log("a8origin/holder/mesh", &self.holder_mesh).unwrap();
        for file in 0..14 {
            for rank in 0..8 {
                let cord = board_to_real_cord(Square::new(file, rank));
                rec.log(
                    format!("a8origin/pieces/{file}/{rank}/"),
                    &rerun::Transform3D::from_translation_rotation_scale(
                        cord,
                        rerun::Rotation3D::IDENTITY,
                        Scale3D::Uniform(0.001),
                    ),
                )
                .unwrap();
            }
        }
    }

    pub fn log_piece_positions(&self, board: &Board) {
        let rec = rerun::RecordingStream::thread_local(rerun::StoreKind::Recording).unwrap();
        for file in 0..14 {
            for rank in 0..8 {
                if let Some(piece) = board.position[file][rank] {
                    if piece.role != Role::Duck {
                        let piece_model_info = self.piece_meshes.get(&piece).unwrap();
                        piece_model_info.log(&rec, &format!("a8origin/pieces/{file}/{rank}"));
                    }
                } else {
                    rec.log(
                        format!("a8origin/pieces/{file}/{rank}/mesh"),
                        &rerun::Clear::flat(),
                    )
                    .unwrap();
                    rec.log(
                        format!("a8origin/pieces/{file}/{rank}/bounding_box"),
                        &rerun::Clear::flat(),
                    )
                    .unwrap();
                }
            }
        }
    }
}
