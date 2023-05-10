#![no_std]
use burktelefon::{Burk};

#[derive(Burk, Clone, Copy, Debug, PartialEq)]
pub enum Response {
    #[burk(name="ISCAL")]
    IsCalibrated(bool),
    #[burk(name="QS")]
    QueueSize(u32, u32), // in queue, max queue size
    #[burk(name="POS")]
    Position(f32, f32, f32),
}

#[derive(Burk, Clone, Copy, Debug, PartialEq)]
pub enum Command {
    #[burk(name="POS")]
    Position,
    #[burk(name="ISCAL")]
    IsCalibrated,
    #[burk(name="CAL")]
    Calibrate,
    #[burk(name="MVS")]
    MoveSideways(f32),
    #[burk(name="MVT")]
    MoveTopArm(f32),
    #[burk(name="MVB")]
    MoveBottomArm(f32),    
    #[burk(name="Q")]
    Queue(f32, f32, f32), // sideways, top arm, bottom arm, speed.
    #[burk(name="QS")]
    QueueSize,
}