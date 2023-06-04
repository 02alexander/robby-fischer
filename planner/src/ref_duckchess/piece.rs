/// The color of a piece.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Color {
    White,
    Black,
}

impl std::ops::Not for Color {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}

/// The role of a piece.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Role {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Role {
 
    pub const MAX_ROLE_HEIGHT: f64 = 0.06;

    pub const ALL: [Role; 6] = [
        Role::Pawn,
        Role::Knight,
        Role::Bishop,
        Role::Rook,
        Role::Queen,
        Role::King,
    ];
}

/// A piece.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Piece {
    pub color: Color,
    pub role: Role,
}

impl Piece {
    pub const fn new(color: Color, role: Role) -> Self {
        Piece { color, role }
    }

    /// The character used by this piece in FEN.
    #[rustfmt::skip]
    pub fn fen_char(self) -> char {
        match self {
            Piece { role: Role::Pawn, color: Color::White } => 'P',
            Piece { role: Role::Bishop, color: Color::White } => 'B',
            Piece { role: Role::Knight, color: Color::White } => 'N',
            Piece { role: Role::Rook, color: Color::White } => 'R',
            Piece { role: Role::Queen, color: Color::White } => 'Q',
            Piece { role: Role::King, color: Color::White } => 'K',
            Piece { role: Role::Pawn, color: Color::Black } => 'p',
            Piece { role: Role::Bishop, color: Color::Black } => 'b',
            Piece { role: Role::Knight, color: Color::Black } => 'n',
            Piece { role: Role::Rook, color: Color::Black } => 'r',
            Piece { role: Role::Queen, color: Color::Black } => 'q',
            Piece { role: Role::King, color: Color::Black } => 'k',
        }
    }

    /// The piece for the specified character in FEN.
    #[rustfmt::skip]
    pub const fn from_fen_char(ch: char) -> Option<Self> {
        Some(match ch {
            'P' => Piece { role: Role::Pawn, color: Color::White },
            'B' => Piece { role: Role::Bishop, color: Color::White },
            'N' => Piece { role: Role::Knight, color: Color::White },
            'R' => Piece { role: Role::Rook, color: Color::White },
            'Q' => Piece { role: Role::Queen, color: Color::White },
            'K' => Piece { role: Role::King, color: Color::White },
            'p' => Piece { role: Role::Pawn, color: Color::Black },
            'b' => Piece { role: Role::Bishop, color: Color::Black },
            'n' => Piece { role: Role::Knight, color: Color::Black },
            'r' => Piece { role: Role::Rook, color: Color::Black },
            'q' => Piece { role: Role::Queen, color: Color::Black },
            'k' => Piece { role: Role::King, color: Color::Black },
            _ => return None,
        })
    }
}
