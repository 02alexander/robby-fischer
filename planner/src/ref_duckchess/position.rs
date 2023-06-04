use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use super::moves::{self, DuckMove, MoveWithBlockers};
use super::{AnyMove, BitBoard, Color, FenError, PieceMove, Role, Square};

/// A duck chess position.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Position {
    /// The role on each square. `None` for empty squares and the square with the
    /// duck.
    pub roles: [Option<Role>; 64],
    /// The squares with white pieces.
    pub white: BitBoard,
    /// The squares with black pieces.
    pub black: BitBoard,
    /// The square with the duck.
    pub duck: Option<Square>,
    /// The rooks that can be castled with.
    pub castling_rights: BitBoard,
    /// The square that can be captured with en passant.
    pub en_passant: Option<Square>,
    /// The player whose turn it is.
    pub turn: Color,
    /// True if the next move is a duck move.
    pub is_duck_move: bool,
    /// The fifty move counter. Increases after piece moves, and the game is a draw
    /// when it reaches 100.
    pub fifty_move_counter: u16,
}

impl Position {
    /// The squares the duck can move to.
    pub fn duck_targets(&self) -> BitBoard {
        let mut targets = !(self.black | self.white);
        if let Some(sq) = self.duck {
            targets.remove(sq);
        }
        targets
    }

    /// Plays a duck move.
    pub fn play_duck(&mut self, target: Option<Square>) -> DuckMoveInfo {
        assert!(self.is_duck_move);

        let from = self.duck;
        self.duck = target;

        self.turn = !self.turn;
        self.is_duck_move = false;

        DuckMoveInfo { from }
    }

    /// Unplays a duck move, with the info returned from `play_duck`.
    pub fn undo_duck(&mut self, info: DuckMoveInfo) {
        assert!(!self.is_duck_move);

        self.turn = !self.turn;
        self.is_duck_move = true;

        self.duck = info.from;
    }

    /// The piece moves in the position. Includes moves blocked by the duck.
    pub fn piece_moves(&self, only_captures: bool, buf: &mut Vec<MoveWithBlockers>) {
        assert!(!self.is_duck_move);
        assert!(buf.is_empty());

        let my_pieces = match self.turn {
            Color::White => self.white,
            Color::Black => self.black,
        };

        for from in my_pieces {
            match self.roles[from as usize].unwrap() {
                Role::Pawn => moves::pawn_moves(self, from, only_captures, buf),
                Role::Knight => moves::knight_moves(self, from, only_captures, buf),
                Role::Bishop => moves::bishop_moves(self, from, only_captures, buf),
                Role::Rook => moves::rook_moves(self, from, only_captures, buf),
                Role::Queen => moves::queen_moves(self, from, only_captures, buf),
                Role::King => moves::king_moves(self, from, only_captures, buf),
            }
        }
    }

    /// Plays a piece move.
    pub fn play_piece(&mut self, mov: PieceMove) -> PieceMoveInfo {
        assert!(!self.is_duck_move);

        let info = PieceMoveInfo {
            mov,
            casting_rights: self.castling_rights,
            en_passant: self.en_passant,
            fifty_move_counter: self.fifty_move_counter,
        };

        match mov {
            PieceMove::Normal {
                role,
                from,
                to,
                cap,
                en_passant,
                ..
            } => {
                if role == Role::King {
                    self.castling_rights &= !BitBoard::piece_rank(self.turn);
                }
                self.castling_rights.remove(from);
                self.castling_rights.remove(to);
                if let Some((_, cap)) = cap {
                    self.castling_rights.remove(cap);
                }

                self.en_passant = en_passant;
                if role == Role::Pawn || cap.is_some() {
                    self.fifty_move_counter = 0;
                } else {
                    self.fifty_move_counter = self.fifty_move_counter.saturating_add(1);
                }
            }
            PieceMove::Castle { .. } => {
                self.castling_rights &= !BitBoard::piece_rank(self.turn);
                self.en_passant = None;
                self.fifty_move_counter = self.fifty_move_counter.saturating_add(1);
            }
        }

        for (piece, square) in mov.removed(self.turn) {
            match piece.color {
                Color::White => self.white.remove(square),
                Color::Black => self.black.remove(square),
            }
            self.roles[square as usize] = None;
        }
        for (piece, square) in mov.added(self.turn) {
            match piece.color {
                Color::White => self.white.add(square),
                Color::Black => self.black.add(square),
            }
            self.roles[square as usize] = Some(piece.role);
        }

        self.is_duck_move = true;

        info
    }

    /// Unplays a piece move, with the info returned from `play_piece`.
    pub fn undo_piece(&mut self, info: PieceMoveInfo) {
        assert!(self.is_duck_move);

        for (piece, square) in info.mov.added(self.turn) {
            match piece.color {
                Color::White => self.white.remove(square),
                Color::Black => self.black.remove(square),
            }
            self.roles[square as usize] = None;
        }
        for (piece, square) in info.mov.removed(self.turn) {
            match piece.color {
                Color::White => self.white.add(square),
                Color::Black => self.black.add(square),
            }
            self.roles[square as usize] = Some(piece.role);
        }

        self.castling_rights = info.casting_rights;
        self.fifty_move_counter = info.fifty_move_counter;
        self.en_passant = info.en_passant;
        self.is_duck_move = false;
    }

    /// All possible moves.
    pub fn moves(&self) -> Vec<AnyMove> {
        if self.is_duck_move {
            let targets = self.duck_targets();
            targets
                .into_iter()
                .map(|target| {
                    AnyMove::Duck(DuckMove {
                        from: self.duck,
                        to: target,
                    })
                })
                .collect()
        } else {
            let mut buf = Vec::new();
            self.piece_moves(false, &mut buf);
            buf.into_iter().map(|mv| AnyMove::Piece(mv.mov)).collect()
        }
    }

    /// Plays a piece or duck move.
    pub fn play_move(&mut self, mov: AnyMove) {
        match mov {
            AnyMove::Duck(mov) => {
                self.play_duck(Some(mov.to));
            }
            AnyMove::Piece(mov) => {
                self.play_piece(mov);
            }
        }
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        super::fen::write_fen(self, f)
    }
}

impl FromStr for Position {
    type Err = FenError;

    fn from_str(s: &str) -> Result<Self, FenError> {
        super::fen::parse_fen(s)
    }
}

/// Information to undo duck moves with `undo_duck`.
pub struct DuckMoveInfo {
    from: Option<Square>,
}

/// Information to undo piece moves with `undo_piece`.
pub struct PieceMoveInfo {
    mov: PieceMove,
    casting_rights: BitBoard,
    en_passant: Option<Square>,
    fifty_move_counter: u16,
}
