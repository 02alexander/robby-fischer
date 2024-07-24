#[cfg(feature = "vis")]
use eagle::vis_camera;

use eagle::Vision;
use glam::Vec3;
use nix::sys::termios::BaudRate;

use planner::{
    arm::Arm,
    board::chess_pos_to_board,
    chess::{Color, Square},
    moves::PieceMove,
    termdev::TerminalDevice,
    uci::Engine,
};

#[cfg(feature = "vis")]
use planner::visualizer::{arm_vis::init_arm_vis, BOARD_VISUALIZER};
#[cfg(feature = "vis")]
use rerun::RecordingStream;

use robby_fischer::{Command, Response};
use shakmaty::{uci::Uci, Chess, Position};
use std::{sync::mpsc::sync_channel, time::Duration};

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

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);

    // arm.translation_offset = Vec3::new(-0.1383520286271571, -0.015, -0.015553090130407);
    arm.translation_offset =
        -Vec3::new(0.1411907894023803, 0.02200000000000005, 0.0243057524245006);

    let app_id = "RobbyFischer";
    let rec_id = uuid::Uuid::new_v4().to_string();
    #[cfg(feature = "vis")]
    let rec = rerun::RecordingStreamBuilder::new(app_id)
        .recording_id(&rec_id)
        .connect()
        .unwrap();
    #[cfg(feature = "vis")]
    {
        RecordingStream::set_thread_local(rerun::StoreKind::Recording, Some(rec.clone()));

        // Creates and logs blueprint.
        match std::process::Command::new(format!("../../blueprint.py"))
            .arg("--recording-id")
            .arg(&rec_id)
            .arg("--application-id")
            .arg(&app_id)
            .spawn()
        {
            Err(e) => {
                eprintln!("Error creating blueprint {:?}", e);
            }
            _ => {}
        }
    }
    #[cfg(feature = "vis")]
    init_arm_vis(&rec);

    let mut engine = Engine::new("stockfish", &[])?;
    let mut played_uci_moves = Vec::new();

    arm.calib().unwrap();
    println!("DONE CALIBRATING");

    arm.release().unwrap();
    arm.sync_pos().unwrap();
    // arm.move_claw_to(Vec3::new(0.0, 0.45, 0.2));

    let (vision_sender, vision_recv) = sync_channel(0);

    #[cfg(feature = "vis")]
    let to_be_moved_rec = rec.clone();
    let _vision_handle = std::thread::spawn(move || {
        #[cfg(feature = "vis")]
        RecordingStream::set_thread_local(rerun::StoreKind::Recording, Some(to_be_moved_rec));

        let mut vision = Vision::new();
        loop {
            let _ = vision_sender.try_send(vision.pieces());
        }
    });

    #[cfg(feature = "vis")]
    {
        let to_be_moved_rec = rec.clone();
        let _cam_vis = std::thread::spawn(move || {
            RecordingStream::set_thread_local(rerun::StoreKind::Recording, Some(to_be_moved_rec));
            vis_camera("external_camera", 0);
        });
    }

    let mut chess_board = Chess::default();
    let mut board = chess_pos_to_board(chess_board.clone()).unwrap();

    #[cfg(feature = "vis")]
    BOARD_VISUALIZER.log_piece_positions(&board);

    arm.practical_smooth_move_claw_to(Vec3::new(0.1, 0.48, 0.15))?;

    println!("waiting for button...");

    arm.sync_pos().unwrap();

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

        #[cfg(feature = "vis")]
        BOARD_VISUALIZER.log_piece_positions(&new_board);

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

        #[cfg(feature = "vis")]
        BOARD_VISUALIZER.log_piece_positions(&board);

        if let Err(e) = arm.practical_smooth_move_claw_to(Vec3::new(0.1, 0.48, 0.15)) {
            dbg!(e);
            continue;
        }
        if moves_since_cailbration >= 10 {
            if let Err(e) = arm.calib_all_except_sideways() {
                dbg!(e);
                continue;
            }
            loop {
                if let Err(e) = arm.practical_smooth_move_claw_to(Vec3::new(0.1, 0.48, 0.15)) {
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
