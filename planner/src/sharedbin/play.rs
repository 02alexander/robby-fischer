use eagle::Vision;
use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{
    arm::Arm,
    board::Board,
    chess::{Color, Piece, Role, Square},
    termdev::TerminalDevice,
};
use robby_fischer::Command;
use std::{
    io::empty,
    sync::mpsc::{channel, Receiver},
    time::Duration,
};

struct Button {
    recv: Receiver<()>,
}
impl Button {
    pub fn new() -> Self {
        let (send, recv) = channel();

        std::thread::spawn(move || {
            for _ in std::io::stdin().lines() {
                if send.send(()).is_err() {
                    break;
                }
            }
        });

        Button { recv }
    }
    pub fn reset(&self) {
        while self.recv.try_recv().is_ok() {}
    }
    pub fn has_pressed(&self) -> bool {
        self.recv.try_recv().is_ok()
    }
}

fn main() -> anyhow::Result<()> {
    // let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
    // td.configure(BaudRate::B115200)?;
    // td.set_timeout(1)?;
    // let mut arm = Arm::new(td);

    // arm.translation_offset = Vector3::new(
    //     -0.128352028627157198,
    //     -0.05799125474691391,
    //     -0.012553090130407229,
    // );

    // arm.calib();
    // println!("DONE CALIBRATING");

    // arm.release();
    // arm.sync_pos()?;
    // arm.move_claw_to(Vector3::new(0.0, 0.45, 0.2));

    let mut board = Board::default();
    board.position[0][1] = Some(Piece::new(Color::White, Role::Pawn));
    board.position[1][1] = Some(Piece::new(Color::White, Role::Pawn));
    board.position[0][6] = Some(Piece::new(Color::Black, Role::Pawn));
    board.position[1][6] = Some(Piece::new(Color::Black, Role::Pawn));

    let button = Button::new();
    let mut vision = Vision::new();

    println!("waiting for button...");
    loop {
        let Some(pieces) = vision.pieces() else {
            println!("no piece found");
            continue;
        };

        if !button.has_pressed() {
            continue;
        }
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
        if board.new_colors(empty_board).is_ok() {
            println!("Okay board!");
        } else {
            eprintln!("Invalid new board");
        }
        println!("{}", board);
    }

    Ok(())
}
