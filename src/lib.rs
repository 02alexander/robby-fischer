#![no_std]
use burktelefon::{Burk};

#[derive(Burk, Clone, Copy, Debug)]
pub enum Response {
    QueueSizeResponse(u32, u32), // in queue, max queue size
    Position(i32, i32, i32),
}

#[derive(Burk, Clone, Copy, Debug)]
pub enum Command {
    Position,
    Calibrate,
    MoveSideways(f32),
    MoveTopArm(f32),
    MoveBottomArm(f32),    
    Queue(f32, f32, f32), // sideways, top arm, bottom arm, speed.
    QueueSize,
}