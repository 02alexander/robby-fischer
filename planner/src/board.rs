use std::time::Duration;

use crate::{
    arm::Arm,
    chess::{Piece, Position, Role, Square},
};
use lazy_static::lazy_static;
use nalgebra::Vector3;

lazy_static! {
    static ref HOLDER_POSISIONS: [[Piece; 8]; 6] = {
        let s = [
            "PPPPPPPP", "RNBKQBNR", "pppppppp", "rnbkqbnr", "pppppppp", "pppppppp",
        ];
        let mut positions = [[Piece::from_fen_char('P').unwrap(); 8]; 6];
        for (fidx, file) in s.iter().enumerate() {
            for (cidx, ch) in file.chars().enumerate() {
                positions[fidx][cidx] = Piece::from_fen_char(ch).unwrap();
            }
        }

        positions
    };
}

pub struct Pieceholder {
    pub occupied: [[bool; 8]; 6],
}

/// Represents the physical board including the Pieceholder for the captured pieces.
pub struct Board {
    pub position: Position,
    pub pieceholder: Pieceholder,
}

impl Default for Board {
    fn default() -> Self {
        Board {
            position: Position::from_partial_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR"),
            pieceholder: Pieceholder::empty(),
        }
    }
}

impl Board {
    pub const SQUARE_SIZE: f64 = 0.05;

    pub fn real_world_coordinate(file: u32, rank: u32) -> Vector3<f64> {
        Vector3::new(
            rank as f64 * Self::SQUARE_SIZE,
            (7.0 - file as f64) * Self::SQUARE_SIZE,
            0.0,
        )
    }

    pub fn move_piece(&mut self, arm: &mut Arm, start: Square, end: Square) {
        let role = self.position[start].unwrap().role;

        // Moves to start and grips the piece.
        arm.smooth_move_z(Role::MAX_ROLE_HEIGHT + 0.01);
        let dz = Vector3::new(0.0, 0.0, arm.claw_pos.z);
        arm.smooth_move_claw_to(
            Self::real_world_coordinate(start.file as u32, start.rank as u32) + dz,
        );
        arm.smooth_move_z(role.grip_height());
        arm.grip();

        // Moves to end and releases the piece
        arm.smooth_move_z(role.height() + Role::MAX_ROLE_HEIGHT + 0.01);
        let dz = Vector3::new(0.0, 0.0, arm.claw_pos.z);
        arm.smooth_move_claw_to(Self::real_world_coordinate(end.file as u32, end.rank as u32) + dz);
        arm.smooth_move_z(role.grip_height());
        arm.release();

        // Move claw up so it isn't in the way.
        arm.smooth_move_z(Role::MAX_ROLE_HEIGHT + 0.01);

        self.position[end] = self.position[start];
        self.position[start] = None;
    }

    pub fn remove_piece(&mut self, arm: &mut Arm, sq: Square) {
        let piece = self.position[sq].unwrap();
        // Moves to start and grips the piece.
        arm.smooth_move_z(Role::MAX_ROLE_HEIGHT + 0.01);
        let dz = Vector3::new(0.0, 0.0, arm.claw_pos.z);
        arm.smooth_move_claw_to(Self::real_world_coordinate(sq.file as u32, sq.rank as u32) + dz);
        arm.smooth_move_z(piece.role.grip_height());
        arm.grip();

        // Moves to end and releases the piece
        arm.smooth_move_z(piece.role.height() + Role::MAX_ROLE_HEIGHT + 0.01);
        let dz = Vector3::new(0.0, 0.0, arm.claw_pos.z);
        let end = self.pieceholder.push(piece).unwrap();
        arm.smooth_move_claw_to(Pieceholder::real_world_coordinate(end) + dz);
        arm.smooth_move_z(piece.role.grip_height());
        arm.release();

        // Move claw up so it isn't in the way.
        arm.smooth_move_z(Role::MAX_ROLE_HEIGHT + 0.01);

        self.position[sq] = None;
    }

    pub fn add_piece(&mut self, arm: &mut Arm, dst: Square, piece: Piece) {
        // Moves to end and releases the piece
        arm.smooth_move_z(Role::MAX_ROLE_HEIGHT + 0.01);
        let dz = Vector3::new(0.0, 0.0, arm.claw_pos.z);
        let end = self.pieceholder.pop(piece).unwrap();
        arm.smooth_move_claw_to(Pieceholder::real_world_coordinate(end) + dz);
        arm.smooth_move_z(piece.role.grip_height());
        std::thread::sleep(Duration::from_millis(400));
        arm.grip();

        // Moves to end and releases the piece
        arm.smooth_move_z(piece.role.height() + Role::MAX_ROLE_HEIGHT + 0.01);
        let dz = Vector3::new(0.0, 0.0, arm.claw_pos.z);
        arm.smooth_move_claw_to(Self::real_world_coordinate(dst.file as u32, dst.rank as u32) + dz);
        arm.smooth_move_z(piece.role.grip_height());
        arm.release();

        // Move claw up so it isn't in the way.
        arm.smooth_move_z(Role::MAX_ROLE_HEIGHT + 0.01);

        self.position[dst] = Some(piece);
    }
}

impl Pieceholder {
    const SQUARE_SIZE: f64 = 0.05;
    const MID_MARIGIN: f64 = 0.01;
    const BOARD_OFFSET: Vector3<f64> = Vector3::new(0.0, 8.0 * Board::SQUARE_SIZE + 0.045, 0.0);

    pub fn empty() -> Pieceholder {
        Pieceholder {
            occupied: [[false; 8]; 6],
        }
    }

    pub fn full() -> Pieceholder {
        Pieceholder {
            occupied: [[true; 8]; 6],
        }
    }

    pub fn pop(&mut self, piece: Piece) -> Option<(usize, usize)> {
        for file in 0..self.occupied.len() {
            for rank in 0..self.occupied[0].len() {
                if self.occupied[file][rank] && HOLDER_POSISIONS[file][rank] == piece {
                    self.occupied[file][rank] = false;
                    return Some((file, rank));
                }
            }
        }
        None
    }

    pub fn push(&mut self, piece: Piece) -> Option<(usize, usize)> {
        for file in 0..self.occupied.len() {
            for rank in 0..self.occupied[0].len() {
                if !self.occupied[file][rank] && HOLDER_POSISIONS[file][rank] == piece {
                    self.occupied[file][rank] = true;
                    return Some((file, rank));
                }
            }
        }
        None
    }

    pub fn real_world_coordinate(idx: (usize, usize)) -> Vector3<f64> {
        let (file, rank) = idx;
        let mut x = rank as f64 * Self::SQUARE_SIZE;
        if rank >= 4 {
            x += Self::MID_MARIGIN;
        }
        let mut y = file as f64 * Self::SQUARE_SIZE;
        if file >= 4 {
            y += Self::MID_MARIGIN;
        }

        Vector3::new(x, y, 0.0) + Self::BOARD_OFFSET
    }
}
