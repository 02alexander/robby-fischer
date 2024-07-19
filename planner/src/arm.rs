use core::panic;
use std::{
    f64::consts::PI, io::{BufRead, BufReader, Error, ErrorKind, Write}, ops, sync::Mutex, time::Duration
};

use k::Node;
use nalgebra::{Rotation2, Vector2, Vector3};
use once_cell::sync::Lazy;
use rerun::{Quaternion, RecordingStream, Rotation3D, Vec3D};
use robby_fischer::{Command, Response};
use super::utils::MyIntersperseExt;

use crate::termdev::TerminalDevice;

const BOTTOM_ARM_LENGTH: f64 = 0.29;
const TOP_ARM_LENGTH: f64 = 0.29;

pub const URDF_PATH: &str = "arm.urdf";

pub static CHAIN: Lazy<Mutex<k::Chain<f32>>> = Lazy::new(|| {
    let chain = k::Chain::<f32>::from_urdf_file(URDF_PATH).unwrap();
    Mutex::new(chain)
});

// pub static REC: Lazy<Mutex<RecordingStream>> = Lazy::new(|| {
//     Mutex::new(
//         rerun::RecordingStreamBuilder::new("RobbyFischer")
//             .connect()
//             .unwrap(),
//     )
// });

pub const CLAW_CHANGE_DELAY: u64 = 600;

fn get_entity_path(link: &Node<f32>) -> String {
    let mut ancestors: Vec<_> = link
        .iter_ancestors()
        .map(|node| node.link().as_ref().unwrap().name.clone())
        .collect();
    ancestors.push(String::from(URDF_PATH));
    ancestors
        .into_iter()
        .rev()
        .my_intersperse(String::from("/"))
        .collect()
}

pub fn log_robot_state(sideways_m: f32, bottom_deg: f32, top_deg: f32, grip_closed: bool) -> Option<()> {
    // let rec = REC.lock().unwrap();
    let rec = RecordingStream::thread_local(rerun::StoreKind::Recording)?;
    let chain = CHAIN.lock().unwrap();
    let bottom = bottom_deg.to_radians();
    let top = top_deg.to_radians();

    let mut positions = chain.joint_positions();
    positions[0] = -(sideways_m-0.2);
    positions[1] = -(bottom - std::f32::consts::PI/2.0);
    positions[2] = -(top - std::f32::consts::PI/2.0);
    positions[3] = -(bottom + top - std::f32::consts::PI);
    if grip_closed {
        positions[4] = -0.02;
        positions[5] = -0.02;
    } else {
        positions[4] = 0.0;
        positions[5] = 0.0;
    }
    chain.set_joint_positions(&positions).unwrap();

    chain.update_transforms();
    chain.update_link_transforms();

    for link_name in chain.iter_links().map(|link| link.name.clone()) {
        let link = chain.find_link(&link_name).unwrap();
        let entity_path = get_entity_path(&link);
        let link_to_world = link.world_transform().unwrap();
        let link_to_parent = if link_name != "base_link" {
            let parent = link.parent().unwrap();
            parent.world_transform().unwrap().inv_mul(&link_to_world)
        } else {
            link_to_world

        };
        let link_to_parent_mat = link_to_parent.to_matrix();


        let trans = link_to_parent_mat.column(3);
        let trans = trans.as_slice();
        let quat = link_to_parent.rotation.quaternion();
        let rot = Rotation3D::Quaternion(Quaternion(quat.coords.as_slice().try_into().unwrap()));
        rec.log(
            entity_path,
            &rerun::Transform3D::from_translation_rotation(
                Vec3D::new(trans[0], trans[1], trans[2]),
                Rotation3D::Quaternion(Quaternion(quat.coords.as_slice().try_into().unwrap())),
            ),
        )
        .unwrap();
    }
    Some(())
}


pub struct Arm {
    pub claw_pos: Vector3<f64>,

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
            translation_offset: Vector3::new(0.0, 0.0, 0.0),
            reader,
            writer,
        }
    }

    pub fn calib(&mut self) -> std::io::Result<()> {
        self.calib_all_except_sideways()?;
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
        Ok(())
    }

    pub fn calib_all_except_sideways(&mut self) -> std::io::Result<()> {
        self.send_command(Command::CalibrateArm)?;
        self.move_claw_to(Vector3::new(0.0, 0.0, 0.15))?;
        std::thread::sleep(Duration::from_millis(100));
        self.send_command(Command::CalibrateArm)?;
        self.move_claw_to(Vector3::new(0.0, 0.00, 0.15))?;
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
                log_robot_state(sd, a1, a2, false);
                let a1 = a1 as f64;
                let a2 = a2 as f64;
                let sd = sd as f64;
                let cord2d = Arm::position_from_angles(a1, a2);
                println!("{a1} {a2}");
                self.claw_pos = Vector3::new(cord2d[0], sd, cord2d[1]) + self.translation_offset;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub fn move_claw(&mut self, change: Vector3<f64>) -> std::io::Result<()> {
        self.move_claw_to(self.claw_pos + change)
    }

    pub fn move_claw_to(&mut self, position: Vector3<f64>) -> std::io::Result<()> {
        self.claw_pos = position;
        let (a1, a2, sd) = dbg!(self.angles(position));
        self.send_command(Command::Queue(a1 as f32, a2 as f32, sd as f32, 1.0))?;
        Ok(())
    }

    fn angles(&self, pos: Vector3<f64>) -> (f64, f64, f64) {
        let (a1, a2) = Arm::arm_2d_angles(pos - self.translation_offset);
        let a1 = a1 * 180.0 / core::f64::consts::PI;
        let a2 = a2 * 180.0 / core::f64::consts::PI;
        // eprintln!("{ba} {ta}");

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
                    return Err(e.into());
                }
                trimmed.parse().map_err(|e| Error::new(ErrorKind::InvalidData, e))
            }
            Err(e) => Err(e.into()),
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
        let bottom_arm = Vector2::new(-BOTTOM_ARM_LENGTH, 0.0);
        let top_arm = Vector2::new(-TOP_ARM_LENGTH, 0.0);
        let rot1 = Rotation2::new(-theta1 * PI / 180.0);
        let rot2 = Rotation2::new(-theta2 * PI / 180.0);
        rot1 * (bottom_arm + rot2 * top_arm)
    }

    pub fn smooth_move_z(&mut self, z: f64) -> std::io::Result<()> {
        let mut pos = self.claw_pos;
        pos.z = z;
        self.practical_smooth_move_claw_to(pos)
    }

    /// Computes the coordinates to move to compensate for inaccuracies when moving on the opposite end of the board.
    pub fn practical_real_world_coordinate(mut pos: Vector3<f64>) -> Vector3<f64> {
        let threshold = 0.075;
        if pos.x >= threshold {
            pos.x += (pos.x - threshold) / (0.35 - threshold) * 0.001;
            pos.z += (pos.x - threshold) / (0.35 - threshold) * 0.002;
        }
        pos
    }

    pub fn practical_smooth_move_claw_to(&mut self, pos: Vector3<f64>) -> std::io::Result<()> {
        let target_pos = Self::practical_real_world_coordinate(pos);
        // let target_pos = pos;
        const N_POINTS_CM: f64 = 3.0;
        let npoints = (self.claw_pos - target_pos).norm() * 100.0 * N_POINTS_CM;
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

    pub fn speed_factor(start: Vector3<f64>, cur: Vector3<f64>, dst: Vector3<f64>) -> f64 {
        let min_dist = (start - cur).norm().min((cur - dst).norm());
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
        std::thread::sleep(Duration::from_millis(CLAW_CHANGE_DELAY));
        self.send_command(Command::Grip)?;
        std::thread::sleep(Duration::from_millis(CLAW_CHANGE_DELAY));
        Ok(())
    }

    pub fn release(&mut self) -> std::io::Result<()> {
        std::thread::sleep(Duration::from_millis(CLAW_CHANGE_DELAY));
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
        + ops::Div<f64, Output = T>
        + ops::Mul<f64, Output = T>,
{
    let n = n.max(2);
    let step_size = (end - start) / (n - 1) as f64;
    (0..=n - 1).map(move |i| start + step_size * i as f64)
}
