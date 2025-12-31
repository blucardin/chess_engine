use bincode::{Decode, Encode};
use crate::coordinate::Coordinate;
use crate::{MoveType, Side};
use crate::piece::PiecePerson;

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum Move {
    Regular {
        initial_position: Coordinate,
        final_position: Coordinate,
        move_type: MoveType,
    },
    Promote {
        initial_position: Coordinate,
        final_position: Coordinate,
        move_type: MoveType,
        piece_person: PiecePerson,
    },
    Castle {
        side: Side,
    },
    EnPassant {
        initial_position: Coordinate,
        final_position: Coordinate,
    },
}

impl Move {
    // fn to_byte(&self) -> u16 {
    //     // let output = [false ; 8];
    //     let website = 0u16;
    //
    //     match self {
    //         Move::Regular { .. } => {}
    //         Move::Promote { .. } => {}
    //         Move::Castle { .. } => {}
    //         Move::EnPassant { .. } => {}
    //     }
    //
    //     //
    //     // let website = output.iter().fold(0u16, |v, b| (v << 1) | (*b as u16));
    //     //
    //     // println!("{}", website);
    //
    //     website
    // }

    pub fn move_to_text_cheat(&self) -> String {
        // untested
        // REMEMBER, supposed to be called before we apply the move
        let from_to = match self {
            Move::Regular {
                initial_position,
                final_position,
                ..
            }
            | Move::Promote {
                initial_position,
                final_position,
                ..
            }
            | Move::EnPassant {
                initial_position,
                final_position,
                ..
            } => {
                format!("{}{}", initial_position.to_text(), final_position.to_text())
            }
            Move::Castle { side } => {
                return match side {
                    Side::QueenSide => String::from("cq"),
                    Side::KingsSide => String::from("ck"),
                };
            }
        };

        let capture = match self {
            Move::Regular { move_type, .. } | Move::Promote { move_type, .. } => {
                *move_type == MoveType::Take
            }
            Move::EnPassant { .. } => true,
            Move::Castle { .. } => {
                panic!("Castles should be returned already")
            }
        };

        let capture_string = if capture { "x" } else { "" };

        let prefix = match self {
            Move::Regular { .. } => String::from("r"),
            Move::Promote { piece_person, .. } => {
                format!("p{}", piece_person.get_uci_name())
            }
            Move::Castle { .. } => {
                panic!("Castles should be returned already")
            }
            Move::EnPassant { .. } => String::from("e"),
        };

        format!("{prefix}{from_to}{capture_string}")
    }
}