use std::fmt::{Display, Write};
use std::str::FromStr;

use super::Color;

/// A square on a chess board.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[rustfmt::skip]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
}

impl Square {
    /// Creates a square from its index.
    ///
    /// # Panics
    ///
    /// Panics if `id >= 64`.
    pub const fn new(id: u8) -> Self {
        assert!(id < 64);
        unsafe { Self::new_unchecked(id) }
    }

    /// Creates a square from its zero indexed x (file) and y (rank) coordinates.
    pub const fn from_x_y(x: u8, y: u8) -> Self {
        assert!(x < 8 && y < 8);
        unsafe { Self::new_unchecked(x + y * 8) }
    }

    /// Creates a square from its index, without checking if it is valid.
    ///
    /// # Safety
    ///
    /// Safe if `id < 64`. Immediate UB otherwise.
    pub const unsafe fn new_unchecked(id: u8) -> Self {
        debug_assert!(id < 64);
        std::mem::transmute(id)
    }

    /// Translates `self` by the specified amount on the X and Y axes. Returns
    /// `None` if the target square is out of bounds.
    pub fn translate(self, dx: i8, dy: i8) -> Option<Self> {
        let x = (self as i32 % 8 + dx as i32) as u32;
        let y = (self as i32 / 8 + dy as i32) as u32;
        (x < 8 && y < 8).then(|| Self::from_x_y(x as u8, y as u8))
    }

    /// Swaps the first and eight rank, second and seventh rank, etc.
    pub fn flip(self) -> Self {
        Square::new(self as u8 ^ 0b111000)
    }

    /// The zero index y (rank) coordinate.
    pub fn y(self) -> u8 {
        self as u8 / 8
    }

    /// The zero index x (file) coordinate.
    pub fn x(self) -> u8 {
        self as u8 % 8
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_char((b'a' + self.x()) as char)?;
        f.write_char((b'1' + self.y()) as char)
    }
}

impl FromStr for Square {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();

        let file = match chars.next() {
            Some(c @ 'A'..='H') => c as i8 - 'A' as i8,
            Some(c @ 'a'..='h') => c as i8 - 'a' as i8,
            _ => return Err(()),
        };
        let rank = match chars.next() {
            Some(c @ '1'..='8') => c as i8 - '1' as i8,
            _ => return Err(()),
        };
        if chars.next().is_some() {
            return Err(());
        }

        Ok(Square::A1.translate(file, rank).unwrap())
    }
}

/// A set of squares on a chess board.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BitBoard {
    pub bits: u64,
}

impl BitBoard {
    /// An empty `BitBoard`.
    pub const EMPTY: Self = BitBoard { bits: 0 };

    /// A `BitBoard` with all squares.
    pub const ALL: Self = BitBoard { bits: !0 };

    /// Tests if the bit for the square is set.
    pub fn has(self, square: Square) -> bool {
        self.bits & (1 << square as u32) != 0
    }

    /// Sets the bit of the square to 1.
    pub fn add(&mut self, square: Square) {
        self.bits |= 1 << square as u32;
    }

    /// Sets the bit of the square to 0.
    pub fn remove(&mut self, square: Square) {
        self.bits &= !(1 << square as u32);
    }

    /// Swaps the 1st and 8th ranks, 2nd and 7th ranks, etc.
    pub fn flip(&mut self) {
        self.bits = self.bits.swap_bytes();
    }

    /// The amount of squares in `self`.
    pub fn count(self) -> u32 {
        self.bits.count_ones()
    }

    /// The piece rank of the player.
    pub fn piece_rank(player: Color) -> Self {
        match player {
            Color::White => BitBoard { bits: 0xff },
            Color::Black => BitBoard { bits: 0xff << 56 },
        }
    }
}

impl From<Square> for BitBoard {
    fn from(sq: Square) -> Self {
        let mut b = BitBoard::EMPTY;
        b.add(sq);
        b
    }
}

impl std::ops::Not for BitBoard {
    type Output = BitBoard;
    fn not(self) -> BitBoard {
        BitBoard { bits: !self.bits }
    }
}

impl std::ops::BitOr for BitBoard {
    type Output = BitBoard;
    fn bitor(self, rhs: Self) -> BitBoard {
        BitBoard {
            bits: self.bits | rhs.bits,
        }
    }
}
impl std::ops::BitOrAssign for BitBoard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

impl std::ops::BitAnd for BitBoard {
    type Output = BitBoard;
    fn bitand(self, rhs: Self) -> BitBoard {
        BitBoard {
            bits: self.bits & rhs.bits,
        }
    }
}
impl std::ops::BitAndAssign for BitBoard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.bits &= rhs.bits;
    }
}

impl IntoIterator for BitBoard {
    type IntoIter = Squares;
    type Item = Square;
    fn into_iter(self) -> Self::IntoIter {
        Squares {
            remaining: self.bits,
        }
    }
}

/// An iterator over the squares in a `BitBoard`, in sorted order.
pub struct Squares {
    remaining: u64,
}

impl Iterator for Squares {
    type Item = Square;
    fn next(&mut self) -> Option<Self::Item> {
        let bit = self.remaining.trailing_zeros();
        if bit == 64 {
            None
        } else {
            self.remaining &= !(1 << bit);
            Some(Square::new(bit as u8))
        }
    }
}
