use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{arm::Arm, board::{Board, Pieceholder}, chess::{Square, Position}, termdev::TerminalDevice};

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-Raspberry_Pi_Pico_1234-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);

    arm.check_calib();

    arm.bottom_angle_offset = 49.84891891479492;
    arm.top_angle_offset = 38.3333625793457;
    arm.translation_offset = Vector3::new(-0.13726842403411865, -0.076, 0.0026969648897647858);

    arm.sync_pos()?;
    println!("{}", arm.claw_pos);
    if arm.claw_pos.z < 0.4 && arm.claw_pos.z > 0.0 && arm.claw_pos.x > 0.0 && arm.claw_pos.x < 0.8 {
        arm.smooth_move_z(0.12);
    } else {
        arm.move_claw_to(Vector3::new(0.10, 0.0, 0.29));        
    }

    let mut board = Board::default();
    let target_chess_board = Position::default();
    board.pieceholder = Pieceholder::full();
    for file in 0..8 {
        for rank in 0..8 {
            board.position.board[file][rank] = None;
        }
    }
    
    for rank in 0..8 {
        for file in 0..8 {
            if let Some(piece) = target_chess_board.board[file][rank] {
                board.add_piece(&mut arm, Square::new(file as u8, rank as u8), piece);        
            }
        }
    }

    // board.move_piece(&mut arm, Square::new(4, 0), Square::new(4, 2));
    // board.move_piece(&mut arm, Square::new(4, 7), Square::new(4, 4));
    // board.remove_piece(&mut arm, Square::new(7, 4));

    Ok(())
}
