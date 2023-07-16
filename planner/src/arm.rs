use core::panic;
use std::{
    f64::consts::PI,
    io::{BufRead, BufReader, Error, Write},
    ops,
    time::Duration,
};

use nalgebra::{Rotation2, Vector2, Vector3};
use robby_fischer::{Command, Response};

use crate::termdev::TerminalDevice;

const BOTTOM_ARM_LENGTH: f64 = 0.29;
const TOP_ARM_LENGTH: f64 = 0.29;

pub struct Arm {
    pub claw_pos: Vector3<f64>,

    pub bottom_angle_offset: f64,
    pub top_angle_offset: f64,
    pub translation_offset: Vector3<f64>,

    /// (0,0,0) is in the middle of the H1 square
    writer: crate::termdev::TerminalWriter,
    reader: BufReader<crate::termdev::TerminalReader>,
}

impl Arm {
    pub fn new(td: TerminalDevice) -> Self {
        let (reader, writer) = td.split();
        let reader = BufReader::new(reader);

        Arm {
            claw_pos: Vector3::new(0.0, 0.0, 0.0),
            bottom_angle_offset: 0.0,
            top_angle_offset: 0.0,
            translation_offset: Vector3::new(0.0, 0.0, 0.0),
            reader,
            writer,
        }
    }

    pub fn check_calib(&mut self) {
        loop {
            std::thread::sleep(Duration::from_millis(100));
            self.send_command(Command::IsCalibrated).unwrap();
            let res = self.get_response();
            if let Err(e) = &res {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    self.send_command(Command::IsCalibrated).unwrap();
                    continue;
                }
            }
            let response = res.unwrap();
            dbg!(response);
            if response != Response::IsCalibrated(true) {
                self.send_command(Command::Calibrate).unwrap();
            } else {
                break;
            }
        }
    }

    pub fn sync_pos(&mut self) -> std::io::Result<()> {
        loop {
            self.send_command(Command::Position)?;
            let response = self.get_response()?;
            if let Response::Position(a1, a2, sd) = response {
                let a1 = a1 as f64;
                let a2 = a2 as f64;
                let sd = sd as f64;
                let ta = a2 - a1 / 3.0 + self.top_angle_offset;
                let ba = a1 + self.bottom_angle_offset;
                let cord2d = Arm::position_from_angles(ba, ta);
                self.claw_pos = Vector3::new(cord2d[0], sd, cord2d[1]) + self.translation_offset;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub fn move_claw(&mut self, change: Vector3<f64>) {
        self.move_claw_to(self.claw_pos + change);
    }

    pub fn move_claw_to(&mut self, position: Vector3<f64>) {
        self.claw_pos = position;
        let (a1, a2, sd) = self.angles(position);
        self.send_command(Command::MoveSideways(sd as f32)).unwrap();
        self.send_command(Command::MoveBottomArm(a1 as f32))
            .unwrap();
        self.send_command(Command::MoveTopArm(a2 as f32)).unwrap();
    }

    fn angles(&self, pos: Vector3<f64>) -> (f64, f64, f64) {
        let (ba, ta) = Arm::arm_2d_angles(pos - self.translation_offset);
        // eprintln!("{ba} {ta}");
        let a1 = ba * 180.0 / PI - self.bottom_angle_offset;
        let a2 = ta * 180.0 / PI - self.top_angle_offset + a1 / 3.0;

        (a1, a2, (pos - self.translation_offset).y)
    }

    pub fn send_command(&mut self, command: Command) -> std::io::Result<()> {
        let mut buf: Vec<_> = command.to_string().bytes().collect();
        // eprintln!("sent: '{}'", String::from_utf8_lossy(&buf));
        buf.push(b'\n');
        self.writer.write_all(&buf)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn get_response(&mut self) -> std::io::Result<Response> {
        let mut buf = Vec::new();
        let res = self.reader.read_until(b'\n', &mut buf);
        match res {
            Ok(_n) => {
                let s = String::from_utf8_lossy(&buf);
                let trimmed = s.trim_end();
                // eprintln!("recv: {:?}", trimmed.as_bytes());
                if trimmed.is_empty() {
                    let e = Error::new(std::io::ErrorKind::WouldBlock, "reading timed out");
                    return Err(e);
                }
                Ok(trimmed.parse().unwrap())
            }
            Err(e) => Err(e),
        }
    }

    pub fn arm_2d_angles(position: Vector3<f64>) -> (f64, f64) {
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

    pub fn smooth_move_z(&mut self, z: f64) {
        let mut pos = self.claw_pos;
        pos.z = z;
        self.smooth_move_claw_to(pos);
    }

    /// Computes the coordinates to move to compensate for inaccuracies when moving on the opposite end of the board.
    pub fn practical_real_world_coordinate(mut pos: Vector3<f64>) -> Vector3<f64> {
        if pos.x >= 0.2 {
            pos.x += (pos.x - 0.175) / 0.175 * 0.001;
            pos.z += (pos.x - 0.175) / 0.175 * 0.005;
        }
        pos
    }

    pub fn smooth_move_claw_to(&mut self, pos: Vector3<f64>) {
        let target_pos = Self::practical_real_world_coordinate(pos);
        const N_POINTS_CM: f64 = 3.0;
        let npoints = (self.claw_pos - target_pos).norm() * 100.0 * N_POINTS_CM;
        for chunk in linspace(self.claw_pos, target_pos, npoints as u32)
            .skip(0)
            .map(|e| (e, Arm::speed_factor(self.claw_pos, e, target_pos)))
            .collect::<Vec<_>>()
            .chunks(20)
        {
            for &(cur_point, scale) in chunk {
                // dbg!(p);
                let (a1, a2, sd) = self.angles(cur_point);
                // dbg!(a1, a2, sd);
                // dbg!(self.claw_pos);
                self.send_command(Command::Queue(
                    a1 as f32,
                    a2 as f32,
                    sd as f32,
                    scale as f32,
                ))
                .unwrap();
            }
            while self.queue_size() >= 15 {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        while self.queue_size() != 0 {
            std::thread::sleep(Duration::from_millis(50));
        }
        self.claw_pos = target_pos;
    }

    pub fn speed_factor(start: Vector3<f64>, cur: Vector3<f64>, dst: Vector3<f64>) -> f64 {
        let min_dist = (start - cur).norm().min((cur - dst).norm());
        if min_dist < 0.05 {
            min_dist / 0.05 * 0.8 + 0.2
        } else {
            1.0
        }
    }

    fn queue_size(&mut self) -> u32 {
        loop {
            self.send_command(Command::QueueSize).unwrap();
            let res = self.get_response();
            match res {
                Ok(response) => {
                    if let Response::QueueSize(in_queue, _max) = response {
                        return in_queue;
                    } else {
                        panic!("expected QueueSize, got '{:?}'", response);
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        continue;
                    } else {
                        panic!("error when reading queue size: {:?}", e);
                    }
                }
            }
        }
    }

    pub fn grip(&mut self) {
        std::thread::sleep(Duration::from_millis(400));
        self.send_command(Command::Grip).unwrap();
        std::thread::sleep(Duration::from_millis(400));
    }

    pub fn release(&mut self) {
        std::thread::sleep(Duration::from_millis(400));
        self.send_command(Command::Release).unwrap();
        std::thread::sleep(Duration::from_millis(600));
    }
}

fn linspace<T>(start: T, end: T, n: u32) -> impl Iterator<Item = T>
where
    T: Copy
        + ops::Sub<Output = T>
        + ops::Add<Output = T>
        + ops::Div<f64, Output = T>
        + ops::Mul<f64, Output = T>,
{
    let n = n.max(2);
    let step_size = (end - start) / (n - 1) as f64;
    (0..=n - 1).map(move |i| start + step_size * i as f64)
}
