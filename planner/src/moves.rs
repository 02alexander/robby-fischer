use arrayvec::ArrayVec;

use crate::{
    board::Board,
    chess::{Color, Piece, Role, Square},
};

/// A piece move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceMove {
    Normal {
        role: Role,
        from: Square,
        to: Square,
        cap: Option<(Role, Square)>,
        new_role: Role,
        en_passant: Option<Square>,
    },
    Castle {
        king_src: Square,
        rook_src: Square,
        king_dst: Square,
        rook_dst: Square,
    },
}

impl PieceMove {
    /// True if this move wins the game immediately.
    pub fn wins(self) -> bool {
        matches!(
            self,
            PieceMove::Normal {
                cap: Some((Role::King, _)),
                ..
            }
        )
    }

    /// The pieces removed by `self`.
    pub fn removed(self, turn: Color) -> ArrayVec<(Piece, Square), 2> {
        let mut squares = ArrayVec::new();

        match self {
            PieceMove::Normal {
                role, from, cap, ..
            } => {
                squares.push((Piece::new(turn, role), from));
                if let Some((role, square)) = cap {
                    squares.push((Piece::new(!turn, role), square));
                }
            }
            PieceMove::Castle {
                king_src, rook_src, ..
            } => {
                squares.push((Piece::new(turn, Role::King), king_src));
                squares.push((Piece::new(turn, Role::Rook), rook_src));
            }
        }

        squares
    }

    /// The pieces added by `self`.
    pub fn added(self, turn: Color) -> ArrayVec<(Piece, Square), 2> {
        let mut squares = ArrayVec::new();

        match self {
            PieceMove::Normal { to, new_role, .. } => {
                squares.push((Piece::new(turn, new_role), to));
            }
            PieceMove::Castle {
                king_dst, rook_dst, ..
            } => {
                squares.push((Piece::new(turn, Role::King), king_dst));
                squares.push((Piece::new(turn, Role::Rook), rook_dst));
            }
        }

        squares
    }
}

fn get_role(board: &Board, square: Square) -> Option<Role> {
    board.position[square.file as usize][square.rank as usize].map(|piece| piece.role)
}

/// The pawn moves for the current player from the square.
pub fn pawn_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let mut move_to = |to: Square, cap: Square, skipped: Option<Square>| {
        let new_roles: &[_] = match to.rank {
            7 => &[Role::Queen, Role::Knight, Role::Rook, Role::Bishop],
            _ => &[Role::Pawn],
        };

        for &new_role in new_roles {
            let mov = PieceMove::Normal {
                role: Role::Pawn,
                from,
                to,
                cap: get_role(pos, cap).map(|r| (r, cap)),
                new_role,
                en_passant: skipped,
            };
            buf.push(mov);
        }
    };

    let to = from.translate(0, 1).unwrap();

    move_to(to, to, None);

    if from.rank == 1 {
        let to2 = to.translate(0, 1).unwrap();
        move_to(to2, to2, Some(to));
    }

    for dx in [-1, 1] {
        if let Some(to) = to.translate(dx, 0) {
            move_to(to, to, None);

            let cap = to.translate(0, -1).unwrap();
            move_to(to, cap, None);
        }
    }
}

/// The moves of pieces other than pawns.
fn straight_moves(
    pos: &Board,
    role: Role,
    from: Square,
    delta: &[(i8, i8)],
    sliding: bool,
    buf: &mut Vec<PieceMove>,
) {
    for &(dx, dy) in delta {
        let mut to = from;

        while let Some(to2) = to.translate(dx, dy) {
            to = to2;

            if let Some(captured) = get_role(pos, to) {
                let mov = PieceMove::Normal {
                    role,
                    from,
                    to,
                    cap: Some((captured, to)),
                    new_role: role,
                    en_passant: None,
                };
                buf.push(mov);
                break;
            }

            let mov = PieceMove::Normal {
                role,
                from,
                to,
                cap: None,
                new_role: role,
                en_passant: None,
            };
            buf.push(mov);

            if !sliding {
                break;
            }
        }
    }
}

/// The valid knight moves.
pub fn knight_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let delta = &[
        (-1, -2),
        (1, -2),
        (-2, -1),
        (2, -1),
        (-2, 1),
        (2, 1),
        (-1, 2),
        (1, 2),
    ];
    straight_moves(pos, Role::Knight, from, delta, false, buf);
}

/// The valid bishop moves.
pub fn bishop_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let delta = &[(-1, -1), (1, -1), (-1, 1), (1, 1)];
    straight_moves(pos, Role::Bishop, from, delta, true, buf);
}

/// The valid rook moves.
pub fn rook_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let delta = &[(0, -1), (-1, 0), (1, 0), (0, 1)];
    straight_moves(pos, Role::Rook, from, delta, true, buf);
}

/// The valid queen moves.
pub fn queen_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let delta = &[
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
        (0, -1),
        (-1, 0),
        (1, 0),
        (0, 1),
    ];
    straight_moves(pos, Role::Queen, from, delta, true, buf);
}

/// The valid king moves.
pub fn king_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let delta = &[
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
        (0, -1),
        (-1, 0),
        (1, 0),
        (0, 1),
    ];
    straight_moves(pos, Role::King, from, delta, false, buf);

    let king_src = from;
    for file in 0..8 {
        let rook_src = Square::new(file, 0);
        if get_role(pos, rook_src) != Some(Role::Rook) {
            continue;
        }

        let (king_file, rook_file) = if king_src.file > rook_src.file {
            (2, 3)
        } else {
            (6, 5)
        };
        let king_dst = Square::new(king_file, 0);
        let rook_dst = Square::new(rook_file, 0);

        let mov = PieceMove::Castle {
            king_src,
            rook_src,
            king_dst,
            rook_dst,
        };
        buf.push(mov);
    }
}

/// A summary of the move, for the engine protocol.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MoveSummary {
    pub from: Option<Square>,
    pub to: Square,
    pub promotion: Option<Role>,
}

impl From<PieceMove> for MoveSummary {
    fn from(mov: PieceMove) -> Self {
        match mov {
            PieceMove::Normal {
                role,
                from,
                to,
                new_role,
                ..
            } => MoveSummary {
                from: Some(from),
                to,
                promotion: (role != new_role).then_some(new_role),
            },
            PieceMove::Castle {
                king_src, rook_src, ..
            } => MoveSummary {
                from: Some(king_src),
                to: rook_src,
                promotion: None,
            },
        }
    }
}
