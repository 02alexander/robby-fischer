use glam::Vec3;
use nix::sys::termios::BaudRate;
use planner::{arm::Arm, board::chess_pos_to_board, chess::Square, termdev::TerminalDevice};

#[cfg(feature = "vis")]
use planner::visualizer::{
    arm_vis::{init_arm_vis, log_robot_state},
    BOARD_VISUALIZER,
};

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);
    arm.translation_offset =
        -Vec3::new(0.1411907894023803, 0.07200000000000005, 0.0243057524245006);
    arm.calib()?;

    #[cfg(feature = "vis")]
    {
        use rerun::RecordingStream;
        use shakmaty::Chess;
        let rec = rerun::RecordingStreamBuilder::new("RobbyFischer")
            .connect()
            .unwrap();
        RecordingStream::set_thread_local(rerun::StoreKind::Recording, Some(rec.clone()));

        let chess_board = Chess::default();

        let mut board = chess_pos_to_board(chess_board.clone()).unwrap();

        #[cfg(feature = "vis")]
        BOARD_VISUALIZER.log_piece_positions(&board);

        #[cfg(feature = "vis")]
        init_arm_vis(&RecordingStream::thread_local(rerun::StoreKind::Recording).unwrap());

        #[cfg(feature = "vis")]
        log_robot_state(0.0, 90.0, 90.0, arm.grabbed_piece);

        arm.practical_smooth_move_claw_to(Vec3::new(0.0, 0.0, 0.2))?;
        board.move_piece(&mut arm, Square::new(2, 6), Square::new(2, 5))?;
    }

    loop {}
}
