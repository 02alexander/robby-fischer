use core::panic;
use std::{
    f32::consts::PI,
    io::{BufRead, BufReader, Error, ErrorKind, Write},
    ops,
    time::Duration,
};

use glam::{Affine2, Vec2, Vec3};
use robby_fischer::{Command, Response};

use crate::{chess::Piece, termdev::TerminalDevice};

#[cfg(feature = "vis")]
use crate::visualizer::arm_vis::log_robot_state;

pub const BOARD_TO_ARM_TRANSFORM: Vec3 =
    Vec3::new(0.1411907894023803, 0.02200000000000005, 0.0243057524245006);
const BOTTOM_ARM_LENGTH: f32 = 0.29;
const TOP_ARM_LENGTH: f32 = 0.29;

// pub static REC: Lazy<Mutex<RecordingStream>> = Lazy::new(|| {
//     Mutex::new(
//         rerun::RecordingStreamBuilder::new("RobbyFischer")
//             .connect()
//             .unwrap(),
//     )
// });

pub const CLAW_CHANGE_DELAY: u64 = 700;

pub struct Arm {
    pub claw_pos: Vec3,

    pub translation_offset: Vec3,
    writer: crate::termdev::TerminalWriter,
    reader: BufReader<crate::termdev::TerminalReader>,
    pub grabbed_piece: Option<Piece>,
}

impl Arm {
    pub fn new(td: TerminalDevice) -> Self {
        let (reader, writer) = td.split();
        let reader = BufReader::new(reader);

        Arm {
            claw_pos: Vec3::new(0.0, 0.0, 0.0),
            translation_offset: Vec3::new(0.0, 0.0, 0.0),
            reader,
            writer,
            grabbed_piece: None,
        }
    }

    pub fn calib(&mut self) -> std::io::Result<()> {
        loop {
            std::thread::sleep(Duration::from_millis(100));
            self.send_command(Command::IsCalibrated)?;
            let res = self.get_response();
            if let Err(e) = &res {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    self.send_command(Command::IsCalibrated)?;
                    continue;
                }
            }
            let response = res?;
            dbg!(response);
            if response != Response::IsCalibrated(true) {
                self.send_command(Command::CalibrateSideways)?;
            } else {
                break;
            }
            self.send_command(Command::CalibrateArm)?;
        }
        println!("calibrated sideways!");
        self.sync_pos()?;
        self.calib_all_except_sideways()?;
        Ok(())
    }

    pub fn calib_all_except_sideways(&mut self) -> std::io::Result<()> {
        self.send_command(Command::CalibrateArm)?;
        let cur_y = self.claw_pos.y;
        self.move_claw_to(Vec3::new(0.0, cur_y, 0.15))?;
        std::thread::sleep(Duration::from_millis(100));
        self.send_command(Command::CalibrateArm)?;
        self.move_claw_to(Vec3::new(0.0, cur_y, 0.15))?;
        std::thread::sleep(Duration::from_millis(100));
        self.send_command(Command::CalibrateArm)?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    pub fn sync_pos(&mut self) -> std::io::Result<()> {
        loop {
            self.send_command(Command::Position)?;
            let response = self.get_response()?;
            if let Response::Position(a1, a2, sd) = response {
                #[cfg(feature = "vis")]
                log_robot_state(sd, a1, a2, self.grabbed_piece);

                let cord2d = Arm::position_from_angles(a1, a2);
                self.claw_pos = Vec3::new(cord2d[0], sd, cord2d[1]) + self.translation_offset;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub fn move_claw(&mut self, change: Vec3) -> std::io::Result<()> {
        self.move_claw_to(self.claw_pos + change)
    }

    pub fn move_claw_to(&mut self, position: Vec3) -> std::io::Result<()> {
        self.claw_pos = position;
        let (a1, a2, sd) = dbg!(self.angles(position));
        self.send_command(Command::Queue(a1 as f32, a2 as f32, sd as f32, 1.0))?;
        Ok(())
    }

    fn angles(&self, pos: Vec3) -> (f32, f32, f32) {
        let (a1, a2) = Arm::arm_2d_angles(pos - self.translation_offset);
        let a1 = a1 * 180.0 / core::f32::consts::PI;
        let a2 = a2 * 180.0 / core::f32::consts::PI;
        (a1, a2, (pos - self.translation_offset).y)
    }

    pub fn send_command(&mut self, command: Command) -> std::io::Result<()> {
        let mut buf: Vec<_> = command.to_string().bytes().collect();
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
                    return Err(e.into());
                }
                trimmed
                    .parse()
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn arm_2d_angles(position: Vec3) -> (f32, f32) {
        let theta = (position.z).atan2(position.x);
        let d = Vec2::new(position.x, position.z).length();
        let q2 = -((d.powi(2) - BOTTOM_ARM_LENGTH.powi(2) - TOP_ARM_LENGTH.powi(2))
            / (2.0 * BOTTOM_ARM_LENGTH * TOP_ARM_LENGTH))
            .acos();
        let thetak =
            (TOP_ARM_LENGTH * q2.sin()).atan2(BOTTOM_ARM_LENGTH + TOP_ARM_LENGTH * q2.cos());
        let q1 = theta - thetak;

        (PI - q1, -q2)
    }

    /// Calculates the claw position from the angles given in degrees.
    pub fn position_from_angles(theta1: f32, theta2: f32) -> Vec2 {
        let bottom_arm = Vec2::new(-BOTTOM_ARM_LENGTH, 0.0);
        let top_arm = Vec2::new(-TOP_ARM_LENGTH, 0.0);
        let rot1 = Affine2::from_angle(-theta1 * PI / 180.0);
        let rot2 = Affine2::from_angle(-theta2 * PI / 180.0);
        rot1.transform_point2(bottom_arm + rot2.transform_point2(top_arm))
    }

    pub fn smooth_move_z(&mut self, z: f32) -> std::io::Result<()> {
        let mut pos = self.claw_pos;
        pos.z = z;
        self.practical_smooth_move_claw_to(pos)
    }

    /// Computes the coordinates to move to compensate for inaccuracies when moving on the opposite end of the board.
    pub fn practical_real_world_coordinate(mut pos: Vec3) -> Vec3 {
        let threshold = 0.075;
        if pos.x >= threshold {
            pos.x += (pos.x - threshold) / (0.35 - threshold) * 0.001;
            pos.z += (pos.x - threshold) / (0.35 - threshold) * 0.002;
        }
        pos
    }

    pub fn practical_smooth_move_claw_to(&mut self, pos: Vec3) -> std::io::Result<()> {
        let target_pos = Self::practical_real_world_coordinate(pos);
        // let target_pos = pos;
        const N_POINTS_CM: f32 = 3.0;
        let npoints = (self.claw_pos - target_pos).length() * 100.0 * N_POINTS_CM;
        for chunk in linspace(self.claw_pos, target_pos, npoints as u32)
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
                ))?;
            }
            while self.queue_size()? >= 15 {
                std::thread::sleep(Duration::from_millis(100));
                self.sync_pos().unwrap();
            }
        }
        while self.queue_size()? != 0 {
            std::thread::sleep(Duration::from_millis(100));
            self.sync_pos().unwrap();
        }
        std::thread::sleep(Duration::from_millis(300));
        self.sync_pos().unwrap();
        self.claw_pos = target_pos;
        Ok(())
    }

    pub fn speed_factor(start: Vec3, cur: Vec3, dst: Vec3) -> f32 {
        let min_dist = (start - cur).length().min((cur - dst).length());
        if min_dist < 0.05 {
            min_dist / 0.05 * 0.8 + 0.2
        } else {
            1.0
        }
    }

    fn queue_size(&mut self) -> std::io::Result<u32> {
        loop {
            self.send_command(Command::QueueSize)?;
            std::thread::sleep(Duration::from_millis(10));
            let res = self.get_response();
            match res {
                Ok(response) => {
                    if let Response::QueueSize(in_queue, _max) = response {
                        return Ok(in_queue);
                    } else {
                        panic!("expected QueueSize, got '{:?}'", response);
                    }
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }

    pub fn grip(&mut self) -> std::io::Result<()> {
        std::thread::sleep(Duration::from_millis(200));
        self.send_command(Command::Grip)?;
        std::thread::sleep(Duration::from_millis(CLAW_CHANGE_DELAY));
        Ok(())
    }

    pub fn release(&mut self) -> std::io::Result<()> {
        std::thread::sleep(Duration::from_millis(200));
        self.send_command(Command::Release)?;
        std::thread::sleep(Duration::from_millis(CLAW_CHANGE_DELAY));
        Ok(())
    }
}

fn linspace<T>(start: T, end: T, n: u32) -> impl Iterator<Item = T>
where
    T: Copy
        + ops::Sub<Output = T>
        + ops::Add<Output = T>
        + ops::Div<f32, Output = T>
        + ops::Mul<f32, Output = T>,
{
    let n = n.max(2);
    let step_size = (end - start) / (n - 1) as f32;
    (0..=n - 1).map(move |i| start + step_size * i as f32)
}
