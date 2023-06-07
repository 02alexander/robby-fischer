#![no_std]
use burktelefon::Burk;
extern crate alloc;

#[derive(Burk, Clone, Copy, Debug, PartialEq)]
pub enum Response {
    #[burk(name = "iscal")]
    IsCalibrated(bool),
    #[burk(name = "qs")]
    QueueSize(u32, u32), // in queue, max queue size
    #[burk(name = "pos")]
    Position(f32, f32, f32),
}

#[derive(Burk, Clone, Copy, Debug, PartialEq)]
pub enum Command {
    #[burk(name = "pos")]
    Position,
    #[burk(name = "grip")]
    Grip,
    #[burk(name = "rel")]
    Release,
    #[burk(name = "iscal")]
    IsCalibrated,
    #[burk(name = "cal")]
    Calibrate,
    #[burk(name = "mvs")]
    MoveSideways(f32),
    #[burk(name = "mvt")]
    MoveTopArm(f32),
    #[burk(name = "mvb")]
    MoveBottomArm(f32),
    #[burk(name = "q")]
    Queue(f32, f32, f32, f32), // sideways, top arm, bottom arm, sideways. speed scaling.
    #[burk(name = "qs")]
    QueueSize,
}
