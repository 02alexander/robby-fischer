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
    Duck,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Action {
    Move(Square, Square),
    Add(Square, Piece),
    Remove(Square, Piece),
}

impl Role {
    pub const ALL: [Role; 7] = [
        Role::Pawn,
        Role::Knight,
        Role::Bishop,
        Role::Rook,
        Role::Queen,
        Role::King,
        Role::Duck,
    ];

    pub const MAX_ROLE_HEIGHT: f64 = 0.079;
    pub fn height(&self) -> f64 {
        match *self {
            Role::Pawn => 0.038,
            Role::Knight => 0.054,
            Role::Bishop => 0.056,
            Role::Rook => 0.041,
            Role::Queen => 0.065,
            Role::King => 0.079,
            Role::Duck => 0.045,
        }
    }

    /// where the arm should grip the piece
    pub fn grip_height(&self) -> f64 {
        match *self {
            Role::Pawn => 0.025,
            Role::Knight => 0.025,
            Role::Bishop => 0.032,
            Role::Rook => 0.025,
            Role::Queen => 0.045,
            Role::King => 0.045,
            Role::Duck => 0.025,
        }
    }
}

/// A piece.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Piece {
    pub color: Color,
    pub role: Role,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Square {
    pub file: u8,
    pub rank: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Position {
    pub board: [[Option<Piece>; 8]; 8],
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
            Piece { role: Role::Duck, color: _ } => 'd',
        }
    }

    /// The piece for the specified character in FEN.
    #[rustfmt::skip]
    pub const fn from_fen_char(ch: char) -> Option<Self> {
        Some(match ch {
            'd' => Piece { role: Role::Duck, color: Color::White },
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

impl Position {
    pub fn diff(&self, other: Position) -> Vec<Action> {
        let mut added = Vec::new();
        let mut removed = Vec::new();

        for file in 0..8 {
            for rank in 0..8 {
                if self.board[file][rank] != other.board[file][rank] {
                    if let Some(piece) = self.board[file][rank] {
                        removed.push(Action::Remove(Square::new(file as u8, rank as u8), piece));
                    }
                    if let Some(piece) = other.board[file][rank] {
                        added.push(Action::Add(Square::new(file as u8, rank as u8), piece));
                    }
                }
            }
        }

        removed.extend(added);
        removed
    }

    pub fn from_partial_fen(fen: &str) -> Self {
        let mut cur_rank = 7;
        let mut cur_file = 0;
        let mut board = [[None; 8]; 8];
        for ch in fen.chars() {
            if cur_file >= 8 {
                cur_file = 0;
            }
            if ch.is_numeric() {
                cur_file += String::from_iter(std::iter::once(ch))
                    .parse::<u8>()
                    .unwrap();
            } else if ch == '/' {
                cur_rank -= 1;
            } else {
                board[cur_file as usize][cur_rank as usize] =
                    Some(Piece::from_fen_char(ch).unwrap());
                cur_file += 1;
            }
        }
        Position { board }
    }
}

impl Default for Position {
    fn default() -> Self {
        Position::from_partial_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR")
    }
}

impl Square {
    pub fn new(file: u8, rank: u8) -> Self {
        Square { file, rank }
    }
}

impl std::ops::Index<Square> for Position {
    type Output = Option<Piece>;

    fn index(&self, sqr: Square) -> &Self::Output {
        &self.board[sqr.file as usize][sqr.rank as usize]
    }
}

impl std::ops::IndexMut<Square> for Position {
    fn index_mut(&mut self, sqr: Square) -> &mut Self::Output {
        &mut self.board[sqr.file as usize][sqr.rank as usize]
    }
}
