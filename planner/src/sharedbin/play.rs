use eagle::Vision;
use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{
    arm::Arm,
    board::Board,
    chess::{Color, Piece, Square},
    moves::PieceMove,
    termdev::TerminalDevice,
};
use robby_fischer::{Command, Response};
use shakmaty::{Chess, Position, uci::Uci};
use uci::Engine;
use std::{
    sync::mpsc::sync_channel,
    time::Duration,
};

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
            role:_,
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

    arm.translation_offset = Vector3::new(-0.1383520286271571, -0.015, -0.015553090130407);

    let engine = Engine::new("stockfish").unwrap().movetime(4000);
    let mut played_uci_moves = Vec::new();

    arm.calib();
    println!("DONE CALIBRATING");

    arm.release();
    arm.sync_pos()?;
    // arm.move_claw_to(Vector3::new(0.0, 0.45, 0.2));

    let (vision_sender, vision_recv) = sync_channel(0);

    let _vision_handle = std::thread::spawn(move || {
        let mut vision = Vision::new();
        loop {
            let _ = vision_sender.try_send(vision.pieces());
        }
    });

    let mut chess_board = Chess::default();
    let mut board = chess_pos_to_board(chess_board.clone()).unwrap();

    arm.smooth_move_claw_to(Vector3::new(0.1, 0.48, 0.15));

    println!("waiting for button...");

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
        let Some(lm) = chess_board.legal_moves().into_iter().find(|lm| chess_move_to_move(lm.clone()) == Some(mv)) else {
            println!("illegal moves");
            continue;
        };
        board = new_board;
        chess_board = chess_board.play(&lm).unwrap();
        played_uci_moves.push(lm.to_uci(shakmaty::CastlingMode::Standard).to_string());
        let target = chess_pos_to_board(chess_board.clone()).unwrap();
        for (src, dst) in board.diff(&target) {
            board.move_piece(&mut arm, src, dst);
        }
        println!("{}", board);

        engine.make_moves(&played_uci_moves).unwrap();
        let engine_move = engine.bestmove().unwrap();
        println!("{}", engine_move);
        let mv = Uci::from_ascii(engine_move.as_bytes()).unwrap().to_move(&chess_board).unwrap();
        chess_board = chess_board.play(&mv).unwrap();
        played_uci_moves.push(mv.to_uci(shakmaty::CastlingMode::Standard).to_string());

        let target = chess_pos_to_board(chess_board.clone()).unwrap();
        for (src, dst) in board.diff(&target) {
            board.move_piece(&mut arm, src, dst);
        }
        println!("{}", board);

        arm.smooth_move_claw_to(Vector3::new(0.1, 0.48, 0.15));
    }
}
