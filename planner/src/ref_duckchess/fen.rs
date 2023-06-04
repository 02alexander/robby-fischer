use std::fmt::{self, Display, Formatter, Write};

use super::{BitBoard, Color, Piece, Position, Role, Square};

/// An error when parsing FEN.
#[derive(Clone, Copy, Debug)]
pub enum FenError {
    MissingPieces,
    InvalidPieces,
    MissingPlayer,
    InvalidPlayer,
    MissingCastlingRights,
    InvalidCastlingRights,
    MissingEnPassant,
    InvalidEnPassant,
    MissingCounter,
    InvalidCounter,
}

impl Display for FenError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use FenError::*;
        f.write_str(match self {
            MissingPieces => "missing pieces",
            InvalidPieces => "invalid pieces",
            MissingPlayer => "missing player",
            InvalidPlayer => "invalid player",
            MissingCastlingRights => "missing castling rights",
            InvalidCastlingRights => "invalid castling rights",
            MissingEnPassant => "missing en passant",
            InvalidEnPassant => "invalid en passant",
            MissingCounter => "missing counter",
            InvalidCounter => "invalid counter",
        })
    }
}
impl std::error::Error for FenError {}

/// Parses the piece part of a FEN string.
fn parse_pieces(fen_pieces: &str) -> Result<Vec<(Option<Piece>, Square)>, FenError> {
    let mut pieces = Vec::new();

    let mut rank = 7;
    let mut file = 0;

    for c in fen_pieces.bytes() {
        match c {
            b'/' if rank > 0 && file == 8 => {
                rank -= 1;
                file = 0;
            }
            b'1'..=b'8' => {
                let amount = c - b'0';
                file += amount;
                if file > 8 {
                    return Err(FenError::InvalidPieces);
                }
            }
            c if file < 8 => {
                let piece = match c {
                    b'*' => None,
                    _ => match Piece::from_fen_char(c as char) {
                        Some(piece) => Some(piece),
                        None => return Err(FenError::InvalidPieces),
                    },
                };

                let square = Square::from_x_y(file, rank);
                pieces.push((piece, square));
                file += 1;
            }
            _ => return Err(FenError::InvalidPieces),
        }
    }

    if (rank, file) != (0, 8) {
        return Err(FenError::InvalidPieces);
    }

    Ok(pieces)
}

/// Parses a FEN.
pub fn parse_fen(fen: &str) -> Result<Position, FenError> {
    let mut parts = fen.split_whitespace();

    // Get the pieces on the board.
    let piece_str = parts.next().ok_or(FenError::MissingPieces)?;
    let pieces = parse_pieces(piece_str)?;

    let mut roles = [None; 64];
    let mut white = BitBoard::EMPTY;
    let mut black = BitBoard::EMPTY;
    let mut duck = None;

    for (piece, square) in pieces {
        if let Some(piece) = piece {
            roles[square as usize] = Some(piece.role);
            match piece.color {
                Color::White => white.add(square),
                Color::Black => black.add(square),
            }
        } else {
            if duck.is_some() {
                return Err(FenError::InvalidPieces);
            }
            duck = Some(square);
        }
    }

    // Get the player to move, and the type of move.
    let (turn, is_duck_move) = match parts.next() {
        Some("w") => (Color::White, false),
        Some("wd") => (Color::White, true),
        Some("b") => (Color::Black, false),
        Some("bd") => (Color::Black, true),
        Some(_) => return Err(FenError::InvalidPlayer),
        None => return Err(FenError::MissingPlayer),
    };

    // Get the castling rights.
    let mut castling_rights = BitBoard::EMPTY;

    let castle_str = parts.next().ok_or(FenError::MissingCastlingRights)?;
    if castle_str != "-" {
        for c in castle_str.chars() {
            let (rank, file, occupied) = match c {
                'A'..='H' => (0, c as u8 - b'A', white),
                'a'..='h' => (7, c as u8 - b'a', black),
                _ => return Err(FenError::InvalidCastlingRights),
            };
            let square = Square::from_x_y(file, rank);
            if roles[square as usize] != Some(Role::Rook) || !occupied.has(square) {
                return Err(FenError::InvalidCastlingRights);
            }

            castling_rights.add(square);
        }
    }

    // Get the en passant square.
    let en_passant_str = parts.next().ok_or(FenError::MissingEnPassant)?;
    let en_passant = match en_passant_str {
        "-" => None,
        square => {
            let square: Square = square.parse().map_err(|_| FenError::InvalidEnPassant)?;

            if (white | black).has(square) {
                return Err(FenError::InvalidEnPassant);
            }
            Some(square)
        }
    };

    // Get the fifty move counter.
    let fifty_move_str = parts.next().ok_or(FenError::MissingCounter)?;
    let fifty_move_counter = fifty_move_str
        .parse()
        .map_err(|_| FenError::InvalidCounter)?;

    Ok(Position {
        roles,
        white,
        black,
        duck,
        castling_rights,
        en_passant,
        turn,
        is_duck_move,
        fifty_move_counter,
    })
}

/// Writes the FEN of the position to the formatter.
pub fn write_fen(pos: &Position, f: &mut Formatter) -> fmt::Result {
    // Write the pieces.
    for y in (0..8).rev() {
        if y != 7 {
            f.write_char('/')?;
        }

        let mut empty_count = 0;
        for x in 0..8 {
            let square = Square::new(x + y * 8);

            if let Some(role) = pos.roles[square as usize] {
                if empty_count > 0 {
                    f.write_char((b'0' + empty_count) as char)?;
                    empty_count = 0;
                }
                let color = match pos.white.has(square) {
                    true => Color::White,
                    false => Color::Black,
                };
                f.write_char(Piece::new(color, role).fen_char())?;
            } else if pos.duck == Some(square) {
                if empty_count > 0 {
                    f.write_char((b'0' + empty_count) as char)?;
                    empty_count = 0;
                }
                f.write_char('*')?;
            } else {
                empty_count += 1;
            }
        }
        if empty_count > 0 {
            f.write_char((b'0' + empty_count) as char)?;
        }
    }

    // Write the player to move, and the type of move.
    f.write_str(match (pos.turn, pos.is_duck_move) {
        (Color::White, false) => " w",
        (Color::White, true) => " wd",
        (Color::Black, false) => " b",
        (Color::Black, true) => " bd",
    })?;

    // Write the castling rights.
    if pos.castling_rights == BitBoard::EMPTY {
        f.write_str(" -")?;
    } else {
        f.write_char(' ')?;
        for square in pos.castling_rights {
            let chr = match square.y() {
                0 => b'A' + square.x(),
                7 => b'a' + square.x(),
                _ => panic!("invalid castling square: {square}"),
            };
            f.write_char(chr as char)?;
        }
    }

    // Write the en passant square.
    if let Some(square) = pos.en_passant {
        write!(f, " {square}")?;
    } else {
        f.write_str(" -")?;
    }

    // Write the counters.
    write!(f, " {} {}", pos.fifty_move_counter, 1)
}
