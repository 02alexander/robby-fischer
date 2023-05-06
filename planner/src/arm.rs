use std::io::{BufRead, BufReader, Write};

use nalgebra::Vector2;
use robby_fischer::{Response, Command};

use crate::termdev::TerminalDevice;

const SQUARE_SIZE: f64 = 0.05;

const BOTTOM_ANGLE_OFFSET: f64 = 43.0;
const TOP_ANGLE_OFFSET: f64 = 43.0;

const H1POS: Vector2<f64> = Vector2::new(0.0, 0.0);
// const H1POS: Vector2<f64> = Vector2::new(0.0, 0.0);

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
    // sideways_pos: f64,
    // bottom_angle: f64,
    // top_angle: f64,
    writer: crate::termdev::TerminalWriter,
    reader: BufReader<crate::termdev::TerminalReader>,
    pub state: State,
}

impl Arm {
    pub fn new(td: TerminalDevice) -> Self {
        let (reader, writer) = td.split();
        let reader = BufReader::new(reader);
        let arm = Arm {
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

    fn send_command(&mut self, command: Command) -> std::io::Result<()> {
        let mut buf: Vec<_> = command.to_string().bytes().collect();
        buf.push('\n' as u8);
        eprintln!("{}", String::from_utf8_lossy(&buf));
        self.writer.write_all(&mut buf)?;
        self.writer.flush()?;
        Ok(())
    }

    fn get_response(&mut self) -> std::io::Result<Response> {
        let mut buf = Vec::new();
        self.reader.read_until('\n' as u8, &mut buf)?;
        let s = String::from_utf8_lossy(&mut buf);
        Ok(s.parse().unwrap())
    }

    pub fn raw_move(&mut self, new_state: State) -> std::io::Result<()> {
        self.state = new_state;
        self.send_command(Command::MoveSideways(5.0*self.state.file as f32))?;
        Ok(())
    }
}
