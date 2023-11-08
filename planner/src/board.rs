use std::time::Duration;

use crate::{
    arm::Arm,
    chess::{Color, Piece, Position, Role, Square},
};
use lazy_static::lazy_static;
use nalgebra::Vector3;

lazy_static! {
    static ref HOLDER_POSISIONS: [[Option<Piece>; 8]; 14] = {
        let s = [
            "...KNBRQ", "...NNBRQ", "...knbrq", "..dnnbrq", "PPPPPPPP", "pppppppp",
        ];
        let mut pieces = [[None; 8]; 14];
        for (file, file_chars) in s.iter().enumerate() {
            for (rank, chr) in file_chars.chars().enumerate() {
                pieces[file+8][rank] = Piece::from_fen_char(chr);
            }
        }
        pieces
    };
}

pub struct Pieceholder {
    pub occupied: [[bool; 8]; 6],
}

/// Represents the physical board including the Pieceholder for the captured pieces.
pub struct Board {
    // pub position: Position,
    // pub pieceholder: Pieceholder,
    pub position: [[Option<Piece>; 8]; 14],
}

impl Default for Board {
    fn default() -> Self {
        Board {
            position: *HOLDER_POSISIONS,
        }
    }
}

impl Board {
    pub const SQUARE_SIZE: f64 = 0.05;

    pub fn new_colors(&mut self, new_colors: [[Option<Color>; 8]; 9]) -> Result<(), ()> {
        let mut added_white = Vec::new();
        let mut added_black = Vec::new();
        let mut removed_white = Vec::new();
        let mut removed_black = Vec::new();

        for file in 0..9 {
            for rank in 0..8 {
                let old_color = self.position[file][rank].map(|piece| match piece.role {
                    Role::Duck => Color::White,
                    _ => piece.color,
                });
                let new_color = new_colors[file][rank];

                if new_color != old_color {
                    match new_color {
                        Some(Color::White) => added_white.push((file, rank)),
                        Some(Color::Black) => added_black.push((file, rank)),
                        None => {}
                    }
                    match old_color {
                        Some(Color::White) => removed_white.push((file, rank)),
                        Some(Color::Black) => removed_black.push((file, rank)),
                        None => {}
                    }
                }
            }
        }

        // Piece appeared or was removed
        if added_white.len() != removed_white.len() || added_black.len() != removed_black.len() {
            return Err(());
        }

        match (added_white.len(), added_black.len()) {
            (1, 0) => {
                let (file1, rank1) = removed_white[0];
                let (file2, rank2) = added_white[0];

                if self.position[file2][rank2].is_some() {
                    return Err(());
                }

                let piece = self.position[file1][rank1].take();
                self.position[file2][rank2] = piece;
                Ok(())
            }
            (1, 1) => {
                let (file1, rank1) = removed_white[0];
                let (file2, rank2) = added_white[0];
                let (file3, rank3) = removed_black[0];
                let (file4, rank4) = added_black[0];

                if file4 != 8 {
                    return Err(());
                }

                let piece1 = self.position[file1][rank1].take();
                let piece2 = self.position[file3][rank3].take();
                self.position[file2][rank2] = piece1;
                self.position[file4][rank4] = piece2;
                Ok(())
            }
            _ => Err(()),
        }
    }

    pub fn real_world_coordinate(file: u32, rank: u32) -> Vector3<f64> {
        if file >= 8 {
            let x = (7.0 - rank as f64 + 1.3) * Self::SQUARE_SIZE;
            let y = if file >= 4 {
                (7.0 - file as f64) * Self::SQUARE_SIZE - 0.01
            } else {
                (7.0 - file as f64) * Self::SQUARE_SIZE + 0.01
            };
            Vector3::new(x, y, 0.0)
        } else {
            Vector3::new(
                (7.0 - rank as f64) * Self::SQUARE_SIZE,
                (7.0 - file as f64) * Self::SQUARE_SIZE,
                0.0,
            )
        }
    }

    pub fn move_piece(&mut self, arm: &mut Arm, start: (usize, usize), end: (usize, usize)) {
        assert!(start.0 < 14);
        assert!(start.1 < 8);
        assert!(end.0 < 14);
        assert!(end.1 < 8);
        if let Some(piece) = self.position[start.0][start.1] {
            let role = piece.role;

            arm.smooth_move_z(Role::MAX_ROLE_HEIGHT + 0.01);
            let dz = Vector3::new(0.0, 0.0, arm.claw_pos.z);
            arm.smooth_move_claw_to(
                Self::real_world_coordinate(start.0 as u32, start.1 as u32) + dz,
            );
            arm.smooth_move_z(role.grip_height());
            arm.grip();

            // Moves to end and releases the piece
            arm.smooth_move_z(role.height() + Role::MAX_ROLE_HEIGHT + 0.01);
            let dz = Vector3::new(0.0, 0.0, arm.claw_pos.z);
            arm.smooth_move_claw_to(Self::real_world_coordinate(end.0 as u32, end.1 as u32) + dz);
            arm.smooth_move_z(role.grip_height());
            arm.release();

            // Move claw up so it isn't in the way.
            arm.smooth_move_z(Role::MAX_ROLE_HEIGHT + 0.01);
        }
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rank in (0..8).rev() {
            for file in 0..14 {
                if let Some(piece) = self.position[file][rank] {
                    write!(f, "{} ", piece.fen_char())?;
                } else {
                    write!(f, ". ")?;

                }
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

// impl Pieceholder {
//     const SQUARE_SIZE: f64 = 0.05;
//     const MID_MARIGIN: f64 = 0.01;
//     const BOARD_OFFSET: Vector3<f64> = Vector3::new(0.0, 8.0 * Board::SQUARE_SIZE + 0.045, 0.0);

//     pub fn empty() -> Pieceholder {
//         Pieceholder {
//             occupied: [[false; 8]; 6],
//         }
//     }

//     pub fn full() -> Pieceholder {
//         Pieceholder {
//             occupied: [[true; 8]; 6],
//         }
//     }

//     pub fn pop(&mut self, piece: Piece) -> Option<(usize, usize)> {
//         for file in 0..self.occupied.len() {
//             for rank in 0..self.occupied[0].len() {
//                 if self.occupied[file][rank] && HOLDER_POSISIONS[file][rank] == piece {
//                     self.occupied[file][rank] = false;
//                     return Some((file, rank));
//                 }
//             }
//         }
//         None
//     }

//     pub fn push(&mut self, piece: Piece) -> Option<(usize, usize)> {
//         for file in 0..self.occupied.len() {
//             for rank in 0..self.occupied[0].len() {
//                 if !self.occupied[file][rank] && HOLDER_POSISIONS[file][rank] == piece {
//                     self.occupied[file][rank] = true;
//                     return Some((file, rank));
//                 }
//             }
//         }
//         None
//     }

//     pub fn real_world_coordinate(idx: (usize, usize)) -> Vector3<f64> {
//         let (file, rank) = idx;
//         let mut x = rank as f64 * Self::SQUARE_SIZE;
//         if rank >= 4 {
//             x += Self::MID_MARIGIN;
//         }
//         let mut y = file as f64 * Self::SQUARE_SIZE;
//         if file >= 4 {
//             y += Self::MID_MARIGIN;
//         }

//         Vector3::new(x, y, 0.0) + Self::BOARD_OFFSET
//     }
// }
