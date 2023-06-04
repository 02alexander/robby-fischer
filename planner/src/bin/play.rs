use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{arm::Arm, board::Board, chess::Square, termdev::TerminalDevice};
fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-Raspberry_Pi_Pico_1234-if00")?;
    td.configure(BaudRate::B115200)?;
    let mut arm = Arm::new(td);

    arm.check_calib();

    arm.bottom_angle_offset = 49.84891891479492;
    arm.top_angle_offset = 38.3333625793457;
    arm.translation_offset = Vector3::new(-0.13726842403411865, -0.076, 0.0026969648897647858);

    arm.move_claw_to(Vector3::new(0.10, 0.0, 0.29));

    let mut board = Board::default();
    // let p = board.pieceholder.push(planner::chess::Piece::from_fen_char('p').unwrap()).unwrap();
    // println!("{:?}", planner::board::Pieceholder::real_world_coordinate(p));
    // arm.smooth_move_claw_to(Vector3::new(0.0, 0.2, 0.01));
    // arm.smooth_move_claw_to(Vector3::new(0.0, 0.59, 0.127));
    board.move_piece(&mut arm, Square::new(7, 1), Square::new(7, 2));
    board.move_piece(&mut arm, Square::new(7, 6), Square::new(7, 7));
    // board.remove_piece(&mut arm, Square::new(7, 4));
    // let stdin = io::stdin().lock();
    // for line in stdin.lines() {
    //     let line = line.unwrap();

    // }

    Ok(())
}
