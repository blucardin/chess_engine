use crate::{PieceWeights, SizeOfCoordinate, SizeOfOffset, BOARD_WEIGHTS, FILES};
use std::ops;
use bincode::{Decode, Encode};
use crate::board::BOARD_TILE_DIM;
use crate::piece::{PieceColor, PiecePerson};

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
pub struct Coordinate { // todo: remove this pub and use a "new_unchecked" constructor function along with a new constructor function to establish that this value is below BoardTileSize.  
    pub x: SizeOfCoordinate,
    pub y: SizeOfCoordinate,
}
impl Coordinate {
    pub fn to_uci_coordinate(&self) -> String {
        let x_string = (BOARD_TILE_DIM - self.x).to_string();
        let y_string = FILES[self.y as usize].to_string();
        format!("{}{}", x_string, y_string)
    }

    pub fn to_text(&self) -> String {
        format!("{}{}", self.x, self.y)
    }

    pub fn value(
        &self,
        piece_person: PiecePerson,
        color: PieceColor,
        weights: &PieceWeights,
    ) -> f32 {
        color.convert_signed(
            piece_person.value(weights)
                + BOARD_WEIGHTS[self.x as usize]
                + BOARD_WEIGHTS[self.y as usize],
        )
    }
}

// impl ops::Add<(SizeOfCoordinate, SizeOfCoordinate)> for Coordinate {
//     type Output = Coordinate;
//     fn add(self, rhs: (SizeOfCoordinate, SizeOfCoordinate)) -> Self::Output {
//         Self::Output {
//             x: self.x + rhs.0,
//             y: self.y + rhs.1,
//         }
//     }
// }

impl ops::Add<(SizeOfOffset, SizeOfOffset)> for Coordinate {
    type Output = Coordinate;
    fn add(self, rhs: (SizeOfOffset, SizeOfOffset)) -> Self::Output {
        Self::Output {
            x: (self.x as SizeOfOffset + rhs.0) as SizeOfCoordinate,
            y: (self.y as SizeOfOffset + rhs.1) as SizeOfCoordinate,
        }
    }
}