use eagle::Vision;
use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use ordered_float::OrderedFloat;
use planner::{
    arm::{Arm, CHAIN, URDF_PATH},
    board::Board,
    chess::{Color, Piece, Role, Square},
    mesh_conversion::{load_gltf, log_node, GltfNode},
    moves::PieceMove,
    termdev::TerminalDevice,
    uci::Engine,
};
use rerun::{
    datatypes::UVec3D, external::glam::Vec3, Angle, Mesh3D, RecordingStream, Rotation3D, RotationAxisAngle, Scale3D, Vec3D
};
use robby_fischer::{Command, Response};
use shakmaty::{uci::Uci, Chess, Position};
use std::{
    collections::HashMap,
    fs::{read_dir, File},
    path::Path,
    sync::mpsc::sync_channel,
    time::Duration,
};
use stl_io::IndexedMesh;

fn chess_pos_to_board(pos: Chess) -> Option<Board> {
    let mut board = Board::default();
    'outer: for (sq, piece) in pos.board().clone() {
        let piece = {
            let role = piece.role.into();
            let color = match piece.color {
                shakmaty::Color::Black => Color::Black,
                shakmaty::Color::White => Color::White,
            };
            Piece::new(color, role)
        };
        for file in (8..14).rev() {
            for rank in 0..8 {
                if board.position[file][rank] == Some(piece) {
                    board.position[file][rank] = None;
                    board.position[sq.file() as usize][sq.rank() as usize] = Some(piece);
                    continue 'outer;
                }
            }
        }
        return None;
    }
    Some(board)
}

fn chess_move_to_move(mv: shakmaty::Move) -> Option<PieceMove> {
    match mv {
        shakmaty::Move::Normal {
            role: _,
            from,
            capture,
            to,
            promotion,
        } => Some(PieceMove::Normal {
            from: from.into(),
            to: to.into(),
            cap: capture.map(|_| to.into()),
            promote: promotion.map(|role| role.into()),
        }),
        shakmaty::Move::EnPassant { from, to } => Some(PieceMove::Normal {
            from: from.into(),
            to: to.into(),
            cap: Some(Square::new(to.file() as usize, from.rank() as usize)),
            promote: None,
        }),
        shakmaty::Move::Castle { king, rook } => {
            let (kf, rf) = if king.file() >= rook.file() {
                (2, 3)
            } else {
                (6, 5)
            };
            Some(PieceMove::Castle {
                king_src: king.into(),
                rook_src: rook.into(),
                king_dst: Square::new(kf, king.rank() as usize),
                rook_dst: Square::new(rf, rook.rank() as usize),
            })
        }
        shakmaty::Move::Put { .. } => None,
    }
}

#[derive(Clone)]
struct PieceModelInfo {
    pub model: rerun::Mesh3D,
    bounding_box: BoundingBox, // (x, y, w, h)
}

struct BoardVisualizer {
    // pub all_pieces: HashMap<Piece, Vec<i32>>,
    // pub available_pieces: HashMap<Piece, Vec<i32>>,
    pub board_scene: GltfNode,
    pub piece_meshes: HashMap<Piece, PieceModelInfo>,
    pub board_offset: Vector3<f64>,
}

#[derive(Clone, Copy, Debug)]
struct BoundingBox {
    center: Vec3,
    half_size: Vec3,
}

impl PieceModelInfo {
    pub fn log(&self, rec: &rerun::RecordingStream, entity_path: &str) {
        self.bounding_box
            .log(rec, &format!("{entity_path}/bounding_box"));
        rec.log(format!("{entity_path}/mesh"), &self.model).unwrap();
    }
}

impl BoundingBox {
    pub fn from_mesh(mesh: &rerun::Mesh3D) -> Self {
        let x_min = mesh
            .vertex_positions
            .iter()
            .map(|pos| OrderedFloat(pos.x()))
            .min()
            .unwrap()
            .0;
        let x_max = mesh
            .vertex_positions
            .iter()
            .map(|pos| OrderedFloat(pos.x()))
            .max()
            .unwrap()
            .0;

        let y_min = mesh
            .vertex_positions
            .iter()
            .map(|pos| OrderedFloat(pos.y()))
            .min()
            .unwrap()
            .0;
        let y_max = mesh
            .vertex_positions
            .iter()
            .map(|pos| OrderedFloat(pos.y()))
            .max()
            .unwrap()
            .0;

        let z_min = mesh
            .vertex_positions
            .iter()
            .map(|pos| OrderedFloat(pos.z()))
            .min()
            .unwrap()
            .0;
        let z_max = mesh
            .vertex_positions
            .iter()
            .map(|pos| OrderedFloat(pos.z()))
            .max()
            .unwrap()
            .0;

        let center = Vec3::new(
            (x_max + x_min) / 2.0,
            (y_max + y_min) / 2.0,
            (z_max + z_min) / 2.0,
        );
        let half_size = Vec3::new(
            (x_max - x_min) / 2.0,
            (y_max - y_min) / 2.0,
            (z_max - z_min) / 2.0,
        );

        println!("{} {}", x_max, x_min);
        println!("{} {}", y_max, y_min);
        println!("{} {}", z_max, z_min);
        println!("center = {center}");
        println!("");

        BoundingBox { center, half_size }
    }

    pub fn log(&self, rec: &rerun::RecordingStream, base_path: &str) {
        let center: Vec3D = self.center.into();
        let half_size: Vec3D = self.half_size.into();
        println!("center = {center}");
        rec.log(
            base_path,
            &rerun::Boxes3D::from_centers_and_half_sizes(&[center], &[half_size]),
        )
        .unwrap();
    }
}

fn stl_to_mesh3d(mesh: &IndexedMesh, color: impl Into<rerun::Color> + Clone) -> Mesh3D {
    let vertices: Vec<_> = mesh
        .vertices
        .iter()
        .map(|v| rerun::Position3D::new(v[0], v[1], v[2]))
        .collect();
    let mut normals = vec![Vec3D::new(0.0, 0.0, 0.0); vertices.len()];
    for face in &mesh.faces {
        for idx in face.vertices {
            // normals[idx] = Vec3D::new(face.normal[0], face.normal.into(), z);
            let v: [f32; 3] = face.normal.into();
            normals[idx] = v.into();
        }
    }

    rerun::Mesh3D::new(vertices)
        .with_triangle_indices(mesh.faces.iter().map(|face| {
            rerun::TriangleIndices(UVec3D::new(
                face.vertices[0] as u32,
                face.vertices[1] as u32,
                face.vertices[2] as u32,
            ))
        }))
        .with_vertex_colors(std::iter::repeat(color).take(mesh.vertices.len()))
        .with_vertex_normals(normals)
}

impl BoardVisualizer {
    pub fn new(mesh_folder_path: impl AsRef<Path>, board_offset: Vector3<f64>) -> Self {
        let mut board_scene = None;
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
        BoardVisualizer {
            // all_pieces,
            board_scene: board_scene.unwrap(),
            board_offset,
            piece_meshes,
        }
    }

    pub fn init_logging(&mut self, rec: &rerun::RecordingStream) {
        // relative to middle of A8 square.
        let board_center: [f32; 3] = Vector3::new(0.175, 0.175, -0.035).into();

        rec.log(
            "a8origin/board",
            &rerun::Transform3D::from_translation(board_center),
        )
        .unwrap();
        log_node(rec, "a8origin/board", self.board_scene.clone()).unwrap();
        for file in 0..14 {
            for rank in 0..8 {
                let cord = BoardVisualizer::board_to_real_cord(Square::new(file, rank));
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

    pub fn log_piece_positions(&mut self, rec: &rerun::RecordingStream, board: &Board) {
        for file in 0..8 {
            for rank in 0..8 {
                if let Some(piece) = board.position[file][rank] {
                    let piece_model_info = self.piece_meshes.get(&piece).unwrap();
                    piece_model_info.log(rec, &format!("a8origin/pieces/{file}/{rank}"));
                } else {
                    
                }
            }
        }
    }

    fn board_to_real_cord(position: Square) -> Vec3 {
        Vec3::new((7-position.rank) as f32, position.file as f32, 0.0) * Board::SQUARE_SIZE as f32
    }
}

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);

    // arm.translation_offset = Vector3::new(-0.1383520286271571, -0.015, -0.015553090130407);
    arm.translation_offset = -Vector3::new(0.1411907894023803, 0.07200000000000005, 0.0243057524245006);

    let rec_id = uuid::Uuid::new_v4().to_string();
    RecordingStream::set_thread_local(
        rerun::StoreKind::Recording,
        Some(
            rerun::RecordingStreamBuilder::new("RobbyFischer")
                .recording_id(&rec_id)
                .connect()
                .unwrap(),
        ),
    );
    let rec = RecordingStream::thread_local(rerun::StoreKind::Recording).unwrap();
    rec.log_file_from_path(URDF_PATH, None, false).unwrap();

    let mut engine = Engine::new("stockfish", &[])?;
    let mut played_uci_moves = Vec::new();

    arm.calib().unwrap();
    println!("DONE CALIBRATING");

    arm.release().unwrap();
    arm.sync_pos().unwrap();
    // arm.move_claw_to(Vector3::new(0.0, 0.45, 0.2));

    let mut board_visualizer = BoardVisualizer::new("pieces", arm.translation_offset);
    board_visualizer.init_logging(&rec);

    let (vision_sender, vision_recv) = sync_channel(0);

    let _vision_handle = std::thread::spawn(move || {
        RecordingStream::set_thread_local(
            rerun::StoreKind::Recording,
            Some(
                rerun::RecordingStreamBuilder::new("RobbyFischer")
                    .recording_id(&rec_id)
                    .connect()
                    .unwrap(),
            ),
        );
        let mut vision = Vision::new();
        loop {
            let _ = vision_sender.try_send(vision.pieces());
        }
    });

    let mut chess_board = Chess::default();
    let mut board = chess_pos_to_board(chess_board.clone()).unwrap();
    board_visualizer.log_piece_positions(&rec, &board);

    arm.practical_smooth_move_claw_to(Vector3::new(0.1, 0.48, 0.15))?;
    // arm.practical_smooth_move_claw_to(Vector3::new(0.1, 0.0, 0.15))?;

    println!("waiting for button...");

    rec.log(
        "arm.urdf",
        &rerun::Transform3D::from_translation_rotation(
            [-0.185, 0.0, 0.04],
            Rotation3D::AxisAngle(RotationAxisAngle::new(
                [0., 0., 1.],
                Angle::Degrees(180.0),
            )),
        ),
    ).unwrap();
    arm.sync_pos().unwrap();
    {
        let chain = CHAIN.lock().unwrap();
        let end_link = chain.find_link("hake_1").unwrap();
        println!("{:?}", end_link.world_transform().unwrap().translation);
    }

    let mut moves_since_cailbration = 0;
    loop {
        std::thread::sleep(Duration::from_millis(10));
        arm.send_command(Command::ChessButton).unwrap();
        if let Ok(Response::ChessButtonStatus(pressed)) = arm.get_response() {
            if !pressed {
                continue;
            }
        } else {
            continue;
        }

        let Some(pieces) = vision_recv.recv().unwrap() else {
            println!("No pieces found!");
            continue;
        };

        println!("button pressed");
        let mut empty_board = [[None; 8]; 9];
        for rank in 0..8 {
            for file in 0..8 {
                if let Some(b) = pieces[file + rank * 8] {
                    empty_board[file][rank] = Some(if b { Color::White } else { Color::Black });
                }
            }
        }
        for rank in 0..8 {
            if let Some(b) = pieces[64 + rank] {
                empty_board[8][rank] = Some(if b { Color::White } else { Color::Black });
            }
        }
        println!("{:?}", pieces);
        println!("{}", board);
        let Some((new_board, mv)) = board.new_colors(empty_board) else {
            println!("bad colors");
            continue;
        };
        let Some(lm) = chess_board
            .legal_moves()
            .into_iter()
            .find(|lm| chess_move_to_move(lm.clone()) == Some(mv))
        else {
            println!("illegal moves");
            continue;
        };
        if let Err(e) = arm.smooth_move_z(0.2) {
            dbg!(e);
            continue;
        }
        board = new_board;
        chess_board = chess_board.play(&lm).unwrap();
        played_uci_moves.push(lm.to_uci(shakmaty::CastlingMode::Standard).to_string());
        engine.start_search(&played_uci_moves)?;
        let target = chess_pos_to_board(chess_board.clone()).unwrap();
        for (src, dst) in board.diff(&target) {
            if let Err(e) = board.move_piece(&mut arm, src, dst) {
                dbg!(e);
                continue;
            }
        }
        println!("{}", board);
        std::thread::sleep(Duration::from_millis(2000));
        let engine_move = engine.stop_search()?;

        println!("{}", engine_move);
        let mv = Uci::from_ascii(engine_move.as_bytes())
            .unwrap()
            .to_move(&chess_board)
            .unwrap();
        chess_board = chess_board.play(&mv).unwrap();
        played_uci_moves.push(mv.to_uci(shakmaty::CastlingMode::Standard).to_string());

        let target = chess_pos_to_board(chess_board.clone()).unwrap();
        for (src, dst) in board.diff(&target) {
            if let Err(e) = board.move_piece(&mut arm, src, dst) {
                dbg!(e);
                continue;
            }
        }
        println!("{}", board);
        board_visualizer.log_piece_positions(&rec, &board);

        if let Err(e) = arm.practical_smooth_move_claw_to(Vector3::new(0.1, 0.48, 0.15)) {
            dbg!(e);
            continue;
        }
        if moves_since_cailbration >= 10 {
            if let Err(e) = arm.calib_all_except_sideways() {
                dbg!(e);
                continue;
            }
            loop {
                if let Err(e) = arm.practical_smooth_move_claw_to(Vector3::new(0.1, 0.48, 0.15)) {
                    dbg!(e);
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                break;
            }
            moves_since_cailbration = 0;
        }

        moves_since_cailbration += 1;
    }
}
