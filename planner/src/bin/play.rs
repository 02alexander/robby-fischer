use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use parrot::stalk_game;
use planner::{arm::Arm, board::Board, chess::Position, termdev::TerminalDevice};

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-Raspberry_Pi_Pico_1234-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);

    arm.check_calib();

    arm.bottom_angle_offset = 49.84891891479492;
    arm.top_angle_offset = 38.3333625793457;
    arm.translation_offset = Vector3::new(-0.13726842403411865, -0.0736, 0.0026969648897647858);

    arm.release();
    arm.sync_pos()?;
    println!("{}", arm.claw_pos);
    if arm.claw_pos.z < 0.4 && arm.claw_pos.z > 0.0 && arm.claw_pos.x > 0.0 && arm.claw_pos.x < 0.8
    {
        arm.smooth_move_z(0.12);
    } else {
        arm.move_claw_to(Vector3::new(0.10, 0.0, 0.29));
    }

    let mut board = Board::default();

    let id = parrot::tv_games()?["Rapid"].to_owned();
    // let id = "uktvwtZI";
    dbg!(&id);

    // let recv = watch_game(id)?;
    let recv = stalk_game(id)?;
    while let Ok(result) = recv.recv() {
        let mut fen = result?;
        while let Ok(result) = recv.try_recv() {
            fen = result?;
        }

        let position = Position::from_partial_fen(fen.split_ascii_whitespace().next().unwrap());
        let actions = board.position.diff(position);
        for action in actions {
            println!("{action:?}");
            match action {
                planner::chess::Action::Move(src, dst) => {
                    board.move_piece(&mut arm, src, dst);
                }
                planner::chess::Action::Add(sq, piece) => board.add_piece(&mut arm, sq, piece),
                planner::chess::Action::Remove(sq, _piece) => board.remove_piece(&mut arm, sq),
            }
        }
        board.position = position;
    }

    Ok(())
}
