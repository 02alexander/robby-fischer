use std::time::Duration;

use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{
    arm::Arm,
    board::chess_pos_to_board,
    chess::Square,
    pathfinding::find_path,
    termdev::TerminalDevice,
    visualizer::{
        arm_vis::{init_arm_vis, log_robot_state, URDF_PATH},
        board_to_real_cord, BoardVisualizer, BoundingBox,
    },
};
use rerun::{external::glam::Vec3, RecordingStream, Vec3D};
use robby_fischer::{Command, Response};
use shakmaty::Chess;

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);
    arm.translation_offset =
        -Vector3::new(0.1411907894023803, 0.07200000000000005, 0.0243057524245006);
    arm.calib()?;


    let rec = rerun::RecordingStreamBuilder::new("RobbyFischer")
        .connect()
        .unwrap();
    RecordingStream::set_thread_local(rerun::StoreKind::Recording, Some(rec.clone()));

    let mut board_visualizer = BoardVisualizer::new("pieces", arm.translation_offset);
    board_visualizer.init_logging(&rec);

    let chess_board = Chess::default();

    let mut board = chess_pos_to_board(chess_board.clone()).unwrap();
    board_visualizer.log_piece_positions(&rec, &board);

    // let claw_bb = BoundingBox {
    //     center: Vec3::new(0.0, 0.0, -0.05),
    //     half_size: Vec3::new(0.02, 0.02, 0.1),
    // };
    // let path = find_path(
    //     board_to_real_cord(Square::new(0, 7)) + Vec3::new(0.0, 0.0, 0.02),
    //     board_to_real_cord(Square::new(6, 2)) + Vec3::new(0.0, 0.0, 0.02),
    //     claw_bb,
    //     &board,
    //     &board_visualizer.piece_meshes,
    // ).unwrap();
    
    // let path: Vec<_> = path.iter().map(|v| Vec3D::new(v[0], v[1], v[2])).collect();
    // println!("{:?}", path);
    // rec.log("a8origin/traj", &rerun::LineStrips3D::new(&[path])).unwrap();
    
    init_arm_vis(&RecordingStream::thread_local(rerun::StoreKind::Recording).unwrap());

    log_robot_state(0.0, 90.0, 90.0, arm.grabbed_piece);

    // arm.practical_smooth_move_claw_to(Vector3::new(0.0, 0.0, 0.2))?;
    board.move_piece(&mut arm, Square::new(2, 6), Square::new(2, 5), &mut board_visualizer)?;
    
    loop {
        // std::thread::sleep(Duration::from_millis(200));

        // arm.send_command(Command::Position)?;
        // match arm.get_response() {
        //     Err(e) => {
        //         println!("{:?}", e);
        //     }
        //     Ok(response) => {
        //         if let Response::Position(bottom, top, sideways) = response {
        //             log_robot_state(sideways, bottom, top, false);
        //             println!("{:?} {:?} {:?}", bottom, top, sideways);
        //         } else {
        //             println!("{:?}", response);
        //         }
        //     }
        // }
    }
    Ok(())
}
