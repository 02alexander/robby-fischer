use crate::{
    board::Board,
    chess::{Color, Piece, Role, Square},
};

/// A piece move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceMove {
    Normal {
        from: Square,
        to: Square,
        cap: Option<Square>,
        promote: Option<Role>,
    },
    Castle {
        king_src: Square,
        rook_src: Square,
        king_dst: Square,
        rook_dst: Square,
    },
}

fn get_piece(board: &Board, square: Square) -> Option<Piece> {
    board.position[square.file][square.rank]
}
fn get_color(board: &Board, square: Square) -> Option<Color> {
    get_piece(board, square).map(|piece| piece.color)
}

/// The pawn moves for the current player from the square.
pub fn pawn_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let mut move_to = |to: Square, cap: Option<Square>| {
        let promotes: &[_] = match to.rank {
            7 => &[
                Some(Role::Queen),
                Some(Role::Knight),
                Some(Role::Rook),
                Some(Role::Bishop),
            ],
            _ => &[None],
        };

        for &promote in promotes {
            buf.push(PieceMove::Normal {
                from,
                to,
                cap,
                promote,
            });
        }
    };

    let to = from.translate(0, 1).unwrap();
    if get_piece(pos, to).is_none() {
        move_to(to, None);
        if from.rank == 1 {
            let to = to.translate(0, 1).unwrap();
            if get_piece(pos, to).is_none() {
                move_to(to, None);
            }
        }
    }

    for dx in [-1, 1] {
        if let Some(to) = to.translate(dx, 0) {
            match get_color(pos, to) {
                Some(Color::White) => {}
                Some(Color::Black) => move_to(to, Some(to)),
                None => {
                    if from.rank == 4 {
                        let cap = to.translate(0, -1).unwrap();
                        if get_color(pos, cap) == Some(Color::Black) {
                            move_to(to, Some(cap));
                        }
                    }
                }
            }
        }
    }
}

/// The moves of pieces other than pawns.
fn straight_moves(
    pos: &Board,
    from: Square,
    delta: &[(isize, isize)],
    sliding: bool,
    buf: &mut Vec<PieceMove>,
) {
    for &(dx, dy) in delta {
        let mut to = from;

        while let Some(to2) = to.translate(dx, dy) {
            to = to2;

            match get_color(pos, to) {
                Some(Color::White) => break,
                Some(Color::Black) => {
                    buf.push(PieceMove::Normal {
                        from,
                        to,
                        cap: Some(to),
                        promote: None,
                    });
                    break;
                }
                None => {
                    buf.push(PieceMove::Normal {
                        from,
                        to,
                        cap: None,
                        promote: None,
                    });
                }
            }

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
    straight_moves(pos, from, delta, false, buf);
}

/// The valid bishop moves.
pub fn bishop_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let delta = &[(-1, -1), (1, -1), (-1, 1), (1, 1)];
    straight_moves(pos, from, delta, true, buf);
}

/// The valid rook moves.
pub fn rook_moves(pos: &Board, from: Square, buf: &mut Vec<PieceMove>) {
    let delta = &[(0, -1), (-1, 0), (1, 0), (0, 1)];
    straight_moves(pos, from, delta, true, buf);
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
    straight_moves(pos, from, delta, true, buf);
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
    straight_moves(pos, from, delta, false, buf);

    let king_src = from;
    for file in 0..8 {
        let rook_src = Square::new(file, 0);
        if get_piece(pos, rook_src) != Some(Piece::new(Color::White, Role::Rook)) {
            continue;
        }

        let (king_file, rook_file) = if king_src.file > rook_src.file {
            (2, 3)
        } else {
            (6, 5)
        };
        let king_dst = Square::new(king_file, 0);
        let rook_dst = Square::new(rook_file, 0);

        buf.push(PieceMove::Castle {
            king_src,
            rook_src,
            king_dst,
            rook_dst,
        });
    }
}
