use crate::{
    arm::Arm,
    chess::{Color, Piece, Role, Square},
    moves::{
        bishop_moves, king_moves, knight_moves, pawn_moves, queen_moves, rook_moves, PieceMove,
    },
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
                pieces[file + 8][rank] = Piece::from_fen_char(chr);
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

    pub fn new_colors(&self, new_colors: [[Option<Color>; 8]; 9]) -> Option<(Board, PieceMove)> {
        let old_colors = self.position.map(|file| {
            file.map(|square| {
                square.map(|piece| match piece.role {
                    Role::Duck => Color::White,
                    _ => piece.color,
                })
            })
        });

        let mut added_white = 0;
        let mut added_black = 0;
        let mut removed_white = 0;
        let mut removed_black = 0;

        for file in 0..9 {
            for rank in 0..8 {
                let old_color = old_colors[file][rank];
                let new_color = new_colors[file][rank];

                if new_color != old_color {
                    match new_color {
                        Some(Color::White) => added_white += 1,
                        Some(Color::Black) => added_black += 1,
                        None => {}
                    }
                    match old_color {
                        Some(Color::White) => removed_white += 1,
                        Some(Color::Black) => removed_black += 1,
                        None => {}
                    }
                }
            }
        }

        // Piece appeared or was removed
        if added_white != removed_white || added_black != removed_black {
            return None;
        }
        let moved_white = added_white;
        let moved_black = added_black;

        for mv in self.all_moves() {
            let mut position = self.position;

            match mv {
                PieceMove::Normal {
                    from,
                    to,
                    cap,
                    promote,
                } => {
                    let from = (from.file as usize, from.rank as usize);
                    let to = (to.file as usize, to.rank as usize);
                    let cap = cap.map(|sq| (sq.file as usize, sq.rank as usize));

                    if let Some(cap) = cap {
                        if moved_black != 1 {
                            continue;
                        }
                        let Some(rank) = (0..8).find(|&rank| {
                            position[8][rank].is_none() && new_colors[8][rank] == Some(Color::Black)
                        }) else {
                            continue;
                        };
                        let dst = (8, rank);

                        if old_colors[cap.0][cap.1] != Some(Color::Black)
                            || new_colors[cap.0][cap.1] == Some(Color::Black)
                            || new_colors[dst.0][dst.1] != Some(Color::Black)
                        {
                            continue;
                        }
                        position[dst.0][dst.1] = position[cap.0][cap.1].take();
                    } else if moved_black != 0 {
                        continue;
                    }

                    if new_colors[from.0][from.1].is_some()
                        || position[to.0][to.1].is_some()
                        || new_colors[to.0][to.1] != Some(Color::White)
                    {
                        continue;
                    }
                    position[to.0][to.1] = position[from.0][from.1].take();

                    if let Some(new_role) = promote {
                        if moved_white != 2 {
                            continue;
                        }

                        let Some(rank) = (0..8).find(|&rank| {
                            position[8][rank] == Some(Piece::new(Color::White, new_role))
                        }) else {
                            continue;
                        };
                        let src = (8, rank);

                        let Some(rank) = (0..8).find(|&rank| {
                            position[8][rank].is_none() && new_colors[8][rank] == Some(Color::White)
                        }) else {
                            continue;
                        };
                        let dst = (8, rank);

                        position[dst.0][dst.1] = position[to.0][to.1].take();
                        position[to.0][to.1] = position[src.0][src.1].take();
                    } else if moved_white != 1 {
                        continue;
                    }

                    return Some((Board { position }, mv));
                }
                PieceMove::Castle {
                    king_src,
                    rook_src,
                    king_dst,
                    rook_dst,
                } => {
                    let ks = (king_src.file as usize, king_src.rank as usize);
                    let rs = (rook_src.file as usize, rook_src.rank as usize);
                    let kd = (king_dst.file as usize, king_dst.rank as usize);
                    let rd = (rook_dst.file as usize, rook_dst.rank as usize);

                    let mut moved = 0;
                    if ks != kd && ks != rd {
                        moved += 1;
                        if new_colors[ks.0][ks.1].is_some() {
                            continue;
                        }
                    }
                    if rs != kd && rs != rd {
                        moved += 1;
                        if new_colors[rs.0][rs.1].is_some() {
                            continue;
                        }
                    }
                    if moved_black != 0
                        || moved_white != moved
                        || new_colors[kd.0][kd.1] != Some(Color::White)
                        || new_colors[rd.0][rd.1] != Some(Color::White)
                    {
                        continue;
                    }

                    return Some((Board { position }, mv));
                }
            }
        }

        None
    }

    pub fn all_moves(&self) -> Vec<PieceMove> {
        let mut moves = Vec::new();
        for rank in 0..8 {
            for file in 0..8 {
                if let Some(Piece {
                    color: Color::White,
                    role,
                }) = self.position[file][rank]
                {
                    let square = Square::new(file as u8, rank as u8);
                    match role {
                        Role::Pawn => pawn_moves(self, square, &mut moves),
                        Role::Knight => knight_moves(self, square, &mut moves),
                        Role::Bishop => bishop_moves(self, square, &mut moves),
                        Role::Rook => rook_moves(self, square, &mut moves),
                        Role::Queen => queen_moves(self, square, &mut moves),
                        Role::King => king_moves(self, square, &mut moves),
                        Role::Duck => {}
                    }
                }
            }
        }
        moves.retain(|piece_move| match piece_move {
            PieceMove::Normal { .. } => true,
            PieceMove::Castle {
                king_src,
                rook_src,
                king_dst,
                rook_dst,
            } => {
                if king_dst != king_src
                    && king_dst != rook_src
                    && self.position[king_dst.file as usize][king_dst.rank as usize].is_some()
                {
                    return false;
                }
                if rook_dst != king_src
                    && rook_dst != rook_src
                    && self.position[rook_dst.file as usize][rook_dst.rank as usize].is_some()
                {
                    return false;
                }
                true
            }
        });
        moves
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
            writeln!(f)?;
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
