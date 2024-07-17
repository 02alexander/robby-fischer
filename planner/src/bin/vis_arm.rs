use std::{collections::HashMap, hash::Hash, sync::{Arc, Mutex}, time::Duration};

use k::urdf;
use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{arm::Arm, termdev::TerminalDevice};
use rerun::{EntityPath, RecordingStream};
use robby_fischer::{Command, Response};
use once_cell::sync::Lazy;


const URDF_PATH: &str = "arm.urdf";

static CHAIN: Lazy<Mutex<k::Chain<f32>>> = Lazy::new(|| {
    let chain = k::Chain::<f32>::from_urdf_file(URDF_PATH).unwrap();
    Mutex::new(chain)
});

static REC: Lazy<Mutex<RecordingStream>> = Lazy::new(|| {
    Mutex::new(
        rerun::RecordingStreamBuilder::new("trajectory")
            .connect()
            .unwrap(),
    )
});

fn log_robot_state(sideways: f32, bottom: f32, top: f32, grip_closed: bool) {
    let rec = REC.lock().unwrap();
    let chain = CHAIN.lock().unwrap();
    // println!("{:?}", );
}

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-alebe_herla_robby_fischer_1972-if00")?;
    td.configure(BaudRate::B115200)?;
    td.set_timeout(1)?;
    let mut arm = Arm::new(td);

    arm.translation_offset = Vector3::new(-0.1383520286271571, -0.015, -0.015553090130407);

    // let mut engine = Engine::new("stockfish", &[])?;
    // let mut played_uci_moves = Vec::new();

    // arm.calib().unwrap();
    // println!("DONE CALIBRATING");

    // arm.release().unwrap();
    // arm.sync_pos().unwrap();

    // let robot_model = RobotModel::new("arm.urdf", HashMap::new(), &[]).unwrap();

    {
        REC.lock().unwrap().log_file_from_path(URDF_PATH, Some("arm".into()), false).unwrap();
    }
    log_robot_state(0.0, 0.0, 0.0, false);

    
    
    loop {
        std::thread::sleep(Duration::from_millis(200));

        arm.send_command(Command::Magnets)?;
        match arm.get_response() {
            Err(e) => {
                println!("{:?}", e);
            },
            Ok(response) => {
                if let Response::Magnets(bottom, top) = response {
                    println!("{:?} {:?}", bottom, top);
                    
                } else {
                    println!("{:?}", response);
                }
            }
        }
    }
    
    Ok(())
}