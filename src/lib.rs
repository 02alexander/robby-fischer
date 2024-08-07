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
    #[burk(name = "chessbtn")]
    ChessButtonStatus(bool), // Checks if the chess button has been pressed since this command wast last sent.
    #[burk(name = "magnets")]
    Magnets(f32, f32), // Checks if the chess button has been pressed since this command wast last sent.
}

#[derive(Burk, Clone, Copy, Debug, PartialEq)]
pub enum Command {
    #[burk(name = "mag")]
    Magnets,
    #[burk(name = "pos")]
    Position,
    #[burk(name = "grip")]
    Grip,
    #[burk(name = "rel")]
    Release,
    #[burk(name = "iscal")]
    IsCalibrated,
    #[burk(name = "calsid")]
    CalibrateSideways,
    #[burk(name = "calarm")]
    CalibrateArm,
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
    #[burk(name = "boot")]
    RestartToBoot,
    #[burk(name = "chessbtn")]
    ChessButton, // Checks if the chess button has been pressed since this command wast last sent.
}
