use pgn_reader::Role;
use bincode::{Decode, Encode};
use crate::{PieceWeights, PIECES_FOLDER};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceColor {
    Black,
    White,
}

impl PieceColor {
    pub fn opposite(&self) -> Self {
        match self {
            PieceColor::Black => PieceColor::White,
            PieceColor::White => PieceColor::Black,
        }
    }

    pub fn file_string(&self) -> String {
        String::from(match self {
            PieceColor::Black => "black",
            PieceColor::White => "white",
        })
    }

    pub fn convert_signed(&self, value: f32) -> f32 {
        match self {
            PieceColor::Black => -value,
            PieceColor::White => value,
        }
    }
}

#[derive(Encode, Decode, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[repr(i8)]
pub enum PiecePerson {
    Pawn = 1,
    Rook { moved: bool } = 3,
    Knight = 2,
    Bishop = 4,
    Queen = 5,
    King { moved: bool } = 6,
}

impl PiecePerson {
    pub fn new_pawn() -> Self {
        PiecePerson::Pawn
    }

    pub fn compare_with_role(&self, role: Role) -> bool {
        match role {
            Role::Pawn => {
                matches!(*self, PiecePerson::Pawn { .. })
            }
            Role::Knight => *self == PiecePerson::Knight,
            Role::Bishop => *self == PiecePerson::Bishop,
            Role::Rook => {
                matches!(*self, PiecePerson::Rook { .. })
            }
            Role::Queen => *self == PiecePerson::Queen,
            Role::King => {
                matches!(*self, PiecePerson::King { .. })
            }
        }
    }

    pub fn from_role(role: Role) -> Self {
        match role {
            Role::Pawn => {
                panic!("Cannot convert role pawn to PiecePerson because of first move ambiguity")
            }
            Role::Knight => PiecePerson::Knight,
            Role::Bishop => PiecePerson::Bishop,
            Role::Rook => PiecePerson::Rook { moved: true },
            Role::Queen => PiecePerson::Queen,
            Role::King => PiecePerson::King { moved: true },
        }
    }

    pub fn file_string(&self) -> String {
        String::from(match self {
            PiecePerson::Pawn  => "pawn",
            PiecePerson::Rook { .. } => "rook",
            PiecePerson::Knight => "knight",
            PiecePerson::Bishop => "bishop",
            PiecePerson::Queen => "queen",
            PiecePerson::King { .. } => "king",
        })
    }

    pub fn get_index(&self) -> usize {
        match self {
            PiecePerson::Pawn { .. } => 0,
            PiecePerson::Rook { .. } => 1,
            PiecePerson::Knight => 2,
            PiecePerson::Bishop => 3,
            PiecePerson::Queen => 4,
            PiecePerson::King { .. } => 5,
        }
    }

    pub fn value(&self, weights: &PieceWeights) -> f32 {
        match self {
            PiecePerson::Pawn { .. } => weights.pawn,
            PiecePerson::Rook { .. } => weights.rook,
            PiecePerson::Knight => weights.knight,
            PiecePerson::Bishop => weights.bishop,
            PiecePerson::Queen => weights.queen,
            PiecePerson::King { .. } => {
                panic!("No value for king.")
            }
        }
    }

    // [1., 4., 2., 3. 5.]
    //     match self {
    //         PiecePerson::Pawn { .. } => 1.,
    //         PiecePerson::Rook { .. } => 4.,
    //         PiecePerson::Knight => 2.,
    //         PiecePerson::Bishop => 3.,
    //         PiecePerson::Queen => 5.,
    //         PiecePerson::King { .. } => panic!("King has no value"),
    //     }
    // }

    pub fn get_uci_name(&self) -> String {
        String::from(["p", "r", "n", "b", "q", "k"][self.get_index()])
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Piece {
    pub(crate) color: PieceColor,
    pub(crate) piece_person: PiecePerson,
}

pub fn format_piece_filename(color_file_string: String, name_file_string: String) -> String {
    format!(
        "{}/{}-{}.png",
        PIECES_FOLDER, color_file_string, name_file_string
    )
}

impl Piece {
    pub(crate) fn new(color: PieceColor, piece_person: PiecePerson) -> Self {
        Piece {
            color,
            piece_person,
        }
    }
    pub fn get_asset_path(&self) -> String {
        let name_file_string = self.piece_person.file_string();
        let color_file_string = self.color.file_string();
        format_piece_filename(color_file_string, name_file_string)
    }

    pub(crate) fn get_string(&self) -> &str {
        let index = self.piece_person.get_index();

        match self.color {
            PieceColor::Black => ["♙", "♖", "♘", "♗", "♕", "♔"][index],
            PieceColor::White => ["♟", "♜", "♞", "♝", "♛", "♚"][index],
        }
    }
}