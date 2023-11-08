use std::fmt::{self, Display, Formatter};

use arrayvec::ArrayVec;

use crate::{chess::{Role, Square, Piece, Color}, board::Board};

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

/// The pawn moves for the current player from the square.
pub fn pawn_moves(
    pos: &Board,
    from: Square,
    only_captures: bool,
    buf: &mut Vec<PieceMove>,
) {
    let mut move_to = |to: Square, cap: Square, skipped: Option<Square>| {
        let new_roles: &[_] = match to.y() {
            0 | 7 => &[Role::Queen, Role::Knight, Role::Rook, Role::Bishop],
            _ => &[Role::Pawn],
        };

        for &new_role in new_roles {
            let mov = PieceMove::Normal {
                role: Role::Pawn,
                from,
                to,
                cap: pos.roles[cap as usize].map(|r| (r, cap)),
                new_role,
                en_passant: skipped,
            };
            buf.push(mov);
        }
    };

    let blockers = pos.white | pos.black;
    let (capturable, fw) = match pos.turn {
        Color::White => (pos.black, 1),
        Color::Black => (pos.white, -1),
    };

    let to = from.translate(0, fw).unwrap();
    if !only_captures && !blockers.has(to) {
        move_to(to, to, None);

        if matches!(from.y(), 1 | 6) {
            if let Some(to2) = to.translate(0, fw) {
                if !blockers.has(to2) {
                    move_to(to2, to2, Some(to));
                }
            }
        }
    }

    for dx in [-1, 1] {
        if let Some(to) = to.translate(dx, 0) {
            if capturable.has(to) {
                move_to(to, to, None);
            } else if Some(to) == pos.en_passant {
                let cap = to.translate(0, -fw).unwrap();
                move_to(to, cap, None);
            }
        }
    }
}

/// The moves of pieces other than pawns.
fn straight_moves(
    pos: &Position,
    role: Role,
    from: Square,
    delta: &[(i8, i8)],
    sliding: bool,
    only_captures: bool,
    buf: &mut Vec<MoveWithBlockers>,
) {
    let friendly = match pos.turn {
        Color::White => pos.white,
        Color::Black => pos.black,
    };

    for &(dx, dy) in delta {
        let mut to = from;
        let mut blockers = BitBoard::EMPTY;

        while let Some(to2) = to.translate(dx, dy) {
            to = to2;

            blockers.add(to);

            if friendly.has(to) {
                break;
            }

            if let Some(captured) = pos.roles[to as usize] {
                let mov = PieceMove::Normal {
                    role,
                    from,
                    to,
                    cap: Some((captured, to)),
                    new_role: role,
                    en_passant: None,
                };
                buf.push(MoveWithBlockers { mov, blockers });
                break;
            }

            if !only_captures {
                let mov = PieceMove::Normal {
                    role,
                    from,
                    to,
                    cap: None,
                    new_role: role,
                    en_passant: None,
                };
                buf.push(MoveWithBlockers { mov, blockers });
            }

            if !sliding {
                break;
            }
        }
    }
}

/// The valid knight moves.
pub fn knight_moves(
    pos: &Position,
    from: Square,
    only_captures: bool,
    buf: &mut Vec<MoveWithBlockers>,
) {
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
    straight_moves(pos, Role::Knight, from, delta, false, only_captures, buf);
}

/// The valid bishop moves.
pub fn bishop_moves(
    pos: &Position,
    from: Square,
    only_captures: bool,
    buf: &mut Vec<MoveWithBlockers>,
) {
    let delta = &[(-1, -1), (1, -1), (-1, 1), (1, 1)];
    straight_moves(pos, Role::Bishop, from, delta, true, only_captures, buf);
}

/// The valid rook moves.
pub fn rook_moves(
    pos: &Position,
    from: Square,
    only_captures: bool,
    buf: &mut Vec<MoveWithBlockers>,
) {
    let delta = &[(0, -1), (-1, 0), (1, 0), (0, 1)];
    straight_moves(pos, Role::Rook, from, delta, true, only_captures, buf);
}

/// The valid queen moves.
pub fn queen_moves(
    pos: &Position,
    from: Square,
    only_captures: bool,
    buf: &mut Vec<MoveWithBlockers>,
) {
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
    straight_moves(pos, Role::Queen, from, delta, true, only_captures, buf);
}

/// The valid king moves.
pub fn king_moves(
    pos: &Position,
    from: Square,
    only_captures: bool,
    buf: &mut Vec<MoveWithBlockers>,
) {
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
    straight_moves(pos, Role::King, from, delta, false, only_captures, buf);

    if only_captures {
        return;
    }

    let (friendly, rank) = match pos.turn {
        Color::White => (pos.white, 0),
        Color::Black => (pos.black, 7),
    };
    let pieces = pos.white | pos.black;

    let king_src = from;
    for rook_src in pos.castling_rights & friendly {
        let (king_file, rook_file, min_file, max_file) = if king_src.x() > rook_src.x() {
            (2, 3, rook_src.x().min(2), king_src.x().max(3))
        } else {
            (6, 5, king_src.x().min(5), rook_src.x().max(6))
        };
        let king_dst = Square::from_x_y(king_file, rank);
        let rook_dst = Square::from_x_y(rook_file, rank);

        let mut blockers = BitBoard {
            bits: ((2 << max_file) - (1 << min_file)) << (rank * 8),
        };
        blockers.remove(king_src);
        blockers.remove(rook_src);

        if (pieces & blockers) == BitBoard::EMPTY {
            let mov = PieceMove::Castle {
                king_src,
                rook_src,
                king_dst,
                rook_dst,
            };
            buf.push(MoveWithBlockers { mov, blockers });
        }
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
impl From<DuckMove> for MoveSummary {
    fn from(mov: DuckMove) -> Self {
        MoveSummary {
            from: mov.from,
            to: mov.to,
            promotion: None,
        }
    }
}
impl From<AnyMove> for MoveSummary {
    fn from(mov: AnyMove) -> Self {
        match mov {
            AnyMove::Piece(mov) => MoveSummary::from(mov),
            AnyMove::Duck(mov) => MoveSummary::from(mov),
        }
    }
}

impl Display for MoveSummary {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(from) = self.from {
            write!(f, "{from}")?;
        }
        write!(f, "{}", self.to)?;
        if let Some(role) = self.promotion {
            write!(f, "{}", Piece::new(Color::Black, role).fen_char())?;
        }
        Ok(())
    }
}
