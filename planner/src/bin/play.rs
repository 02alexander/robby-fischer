use std::io::{self, BufRead};

use nalgebra::Vector3;
use nix::sys::termios::BaudRate;
use planner::{
    arm::{Arm, SQUARE_SIZE},
    termdev::TerminalDevice,
};
use robby_fischer::Command;

fn main() -> anyhow::Result<()> {
    let mut td = TerminalDevice::new("/dev/serial/by-id/usb-Raspberry_Pi_Pico_1234-if00")?;
    td.configure(BaudRate::B115200)?;
    let mut arm = Arm::new(td);

    arm.check_calib();

    arm.bottom_angle_offset = 54.01273727416992;
    arm.top_angle_offset = 34.218055725097656;
    arm.translation_offset = Vector3::new(-0.14979085326194763, -0.28, 0.009541633538901806);

    let stdin = io::stdin().lock();
    let mut height = 0.01;
    for line in stdin.lines() {
        let line = line.unwrap();
        if line.starts_with("h") {
            if let Ok(d) = line[1..].parse::<f64>() {
                height = d * 0.01 + 0.01;
            }
        }
        match line.trim() {
            "grip" => {
                arm.send_command(Command::Grip).unwrap();
            }
            "rel" => {
                arm.send_command(Command::Release).unwrap();
            }
            _ => {}
        }
        match line.parse::<usize>() {
            Ok(v) => {
                let pos = Vector3::new(v as f64 * SQUARE_SIZE, 0.0, height);
                arm.move_claw_to(pos);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }

    // let stdin = io::stdin().lock();
    // for line in stdin.lines() {
    //     let line = line.unwrap();

    // }

    Ok(())
}
