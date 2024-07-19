// use std::time::Duration;

// use nalgebra::Vector3;
// use nix::sys::termios::BaudRate;
// use parrot::stalk_game;
// use planner::{arm::Arm, board::Board, chess::Position, termdev::TerminalDevice};
// use robby_fischer::Command;

// fn main() -> anyhow::Result<()> {
//     let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
//     td.configure(BaudRate::B115200)?;
//     td.set_timeout(1)?;
//     let mut arm = Arm::new(td);

//     println!("1");
//     println!("2");

//     arm.translation_offset = Vector3::new(
//         -0.1283520286271571,
//         -0.0579912547469139,
//         -0.0125530901304072,
//     );
//     // arm.translation_offset = Vector3::new(0.0, 0.0, 0.0);
//     println!("3");
//     arm.release();
//     println!("4");
//     arm.sync_pos()?;
//     println!("5");
//     println!("{}", arm.claw_pos);
//     // if arm.claw_pos.z < 0.4 && arm.claw_pos.z > 0.0 && arm.claw_pos.x > 0.0 && arm.claw_pos.x < 0.8
//     // {
//     //     arm.smooth_move_z(0.12);
//     // } else {
//     //     arm.smooth_move_claw_to(Vector3::new(0.15, 0.0, 0.29));
//     // }

//     // arm.smooth_move_claw_to(Vector3::new(0.0, 0.00, 0.02) );
//     // std::thread::sleep(Duration::from_millis(1500));
//     // arm.smooth_move_claw_to(Vector3::new(0.35, 0.00, 0.02) );

//     arm.calib();

//     // let mut board = Board::default();

//     // let id = parrot::tv_games()?["Rapid"].to_owned();
//     let id = "myJE5y7K";
//     dbg!(&id);

//     // let recv = watch_game(id)?;
//     // let mut actions_since_calib = 0;
//     // let recv = stalk_game(id)?;
//     // while let Ok(result) = recv.recv() {
//     //     let mut fen = result?;
//     //     while let Ok(result) = recv.try_recv() {
//     //         fen = result?;
//     //     }

//     //     let position = Position::from_partial_fen(fen.split_ascii_whitespace().next().unwrap());
//     //     let actions = board.position.diff(position);
//     //     for action in actions {
//     //         println!("{action:?}");
//     //         actions_since_calib += 1;
//     //         match action {
//     //             planner::chess::Action::Move(src, dst) => {
//     //                 board.move_piece(&mut arm, src, dst);
//     //             }
//     //             planner::chess::Action::Add(sq, piece) => board.add_piece(&mut arm, sq, piece),
//     //             planner::chess::Action::Remove(sq, _piece) => board.remove_piece(&mut arm, sq),
//     //         }
//     //         if actions_since_calib > 10 {
//     //             arm.calib();
//     //             actions_since_calib = 0;
//     //         }
//     //     }
//     //     board.position = position;
//     // }

//     Ok(())
// }

fn main() {}
