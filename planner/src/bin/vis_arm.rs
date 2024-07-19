use std::time::Duration;

use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{
    arm::{log_robot_state, Arm, URDF_PATH}, termdev::TerminalDevice
};
use rerun::RecordingStream;
use robby_fischer::{Command, Response};

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);

    arm.translation_offset = Vector3::new(-0.1383520286271571, -0.015, -0.015553090130407);

    RecordingStream::set_thread_local(
        rerun::StoreKind::Recording,
        Some(
            rerun::RecordingStreamBuilder::new("RobbyFischer")
                .connect()
                .unwrap(),
        ),
    );

    {
        let rec = RecordingStream::thread_local(rerun::StoreKind::Recording).unwrap();
        rec.log_file_from_path(URDF_PATH, None, false)?;
        // let mesh = load_gltf_as_mesh3d("pieces/king.gltf");
        // rec.log("board/board", &mesh)?;
    }
    std::thread::sleep(Duration::from_millis(500));
    log_robot_state(0.0, 90.0, 90.0, false);

    
    loop {
        std::thread::sleep(Duration::from_millis(200));

        arm.send_command(Command::Position)?;
        match arm.get_response() {
            Err(e) => {
                println!("{:?}", e);
            }
            Ok(response) => {
                if let Response::Position(bottom, top, sideways) = response {
                    log_robot_state(sideways, bottom, top, false);
                    println!("{:?} {:?} {:?}", bottom, top, sideways);
                } else {
                    println!("{:?}", response);
                }
            }
        }
    }
}
