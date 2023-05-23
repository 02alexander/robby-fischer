use std::{
    f64::consts::PI,
    io::{BufRead, BufReader, Write},
};

use nalgebra::{Rotation2, Vector2, Vector3};
use robby_fischer::{Command, Response};

use crate::termdev::TerminalDevice;

pub const SQUARE_SIZE: f64 = 0.05;

const BOTTOM_ANGLE_OFFSET: f64 =  0.0; //51.2655; // 43.0;
const TOP_ANGLE_OFFSET: f64 = 0.0; //26.641663; // 43.0;
const TRANSLATION_OFFSET: Vector3<f64> =  Vector3::new(0.0, 0.0, 0.0);// Vector3::new(-1.5939192e-01, 1.1850257e-02, 9.9999997e-05);

const BOTTOM_ARM_LENGTH: f64 = 0.29;
const TOP_ARM_LENGTH: f64 = 0.29;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    Duck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub is_low: bool,
    pub file: u8,
    pub rank: u8,
    pub gripping: bool,
}

impl State {}

pub struct Arm {
    pub claw_pos: Vector3<f64>,
    /// (0,0,0) is in the middle of the H1 square
    writer: crate::termdev::TerminalWriter,
    reader: BufReader<crate::termdev::TerminalReader>,
    pub state: State,
}

impl Arm {
    pub fn new(td: TerminalDevice) -> Self {
        let (reader, writer) = td.split();
        let reader = BufReader::new(reader);

        let arm = Arm {
            claw_pos: Vector3::new(0.0, 0.0, 0.0),
            reader,
            writer,
            state: State {
                is_low: true,
                file: 0,
                rank: 0,
                gripping: false,
            },
        };

        arm
    }

    pub fn move_claw(&mut self, change: Vector3<f64>) {
        self.move_claw_to(self.claw_pos + change);
    }

    pub fn move_claw_to(&mut self, position: Vector3<f64>) {
        self.claw_pos = position;
        let (ba, ta) = Arm::angles(self.claw_pos - TRANSLATION_OFFSET);
        // eprintln!("{ba} {ta}");
        let a1 = ba * 180.0 / PI - BOTTOM_ANGLE_OFFSET;
        let a2 = ta * 180.0 / PI - TOP_ANGLE_OFFSET + a1 / 3.0;
        self.send_command(Command::MoveSideways(position.y as f32)).unwrap();
        self.send_command(Command::MoveBottomArm(a1 as f32))
            .unwrap();
        self.send_command(Command::MoveTopArm(a2 as f32)).unwrap();
    }

    pub fn send_command(&mut self, command: Command) -> std::io::Result<()> {
        let mut buf: Vec<_> = command.to_string().bytes().collect();
        buf.push('\n' as u8);
        // eprintln!("{}", String::from_utf8_lossy(&buf));
        self.writer.write_all(&mut buf)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn get_response(&mut self) -> std::io::Result<Response> {
        let mut buf = Vec::new();
        while let Err(e) = self.reader.read_until('\n' as u8, &mut buf) {
            if e.kind() != std::io::ErrorKind::WouldBlock {
                return Err(e);
            }
        }
        let s = String::from_utf8_lossy(&mut buf);
        eprintln!("'{}'", s.trim_end());

        Ok(s.trim_end().parse().unwrap())
    }

    pub fn raw_move(&mut self, new_state: State) -> std::io::Result<()> {
        self.state = new_state;
        self.send_command(Command::MoveSideways(5.0 * self.state.file as f32))?;
        Ok(())
    }

    pub fn angles(position: Vector3<f64>) -> (f64, f64) {
        let theta = (position.z).atan2(position.x);
        let d = Vector2::new(position.x, position.z).norm();
        let q2 = -((d.powi(2) - BOTTOM_ARM_LENGTH.powi(2) - TOP_ARM_LENGTH.powi(2))
            / (2.0 * BOTTOM_ARM_LENGTH * TOP_ARM_LENGTH))
            .acos();
        let thetak =
            (TOP_ARM_LENGTH * q2.sin()).atan2(BOTTOM_ARM_LENGTH + TOP_ARM_LENGTH * q2.cos());
        let q1 = theta - thetak;

        (PI - q1, -q2)
    }

    /// Calculates the claw position from the angles given in degrees.
    pub fn position_from_angles(theta1: f64, theta2: f64) -> Vector2<f64> {
        let bottom_arm = Vector2::new(-0.29, 0.0);
        let top_arm = Vector2::new(-0.29, 0.0);
        let rot1 = Rotation2::new(-theta1 * PI / 180.0);
        let rot2 = Rotation2::new(-theta2 * PI / 180.0);
        rot1 * (bottom_arm + rot2 * top_arm)
    }
}
