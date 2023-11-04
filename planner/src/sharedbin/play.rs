use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{arm::Arm, termdev::TerminalDevice, board::Board, chess::{Piece, Role, Color, Square}};
use robby_fischer::Command;
use std::{sync::mpsc::{Receiver, channel}, time::Duration};
use eagle::Vision;

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
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
    let mut vision = Vision::new();
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);

    arm.translation_offset = Vector3::new(
        -0.128352028627157198, -0.05799125474691391, -0.012553090130407229
    );

    arm.calib();
    println!("DONE CALIBRATING");
    
    arm.release();
    arm.sync_pos()?;

    let mut board = Board::default();

    let button = Button::new();

    arm.move_claw_to(Vector3::new(0.0, 0.45, 0.2));

    println!("waiting for button...");
    loop {
        let Some(pieces) = vision.pieces() else {
            println!("no piece found");
            continue;
        };

        if !button.has_pressed() {
            continue;
        }
        if pieces[0].is_some() {
            // board.add_piece(&mut, dst, piece)
            board.position.board[0][0] = Some(Piece::new(Color::White, Role::Pawn));
            board.move_piece(&mut arm, Square::new(0,0), Square::new(0,1));
            arm.move_claw_to(Vector3::new(0.2, 0.5, 0.2));
            button.reset();
        } else {
            println!("no piece in correct position")
        }
    }

    Ok(())
}