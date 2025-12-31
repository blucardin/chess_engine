use crate::{PieceWeights, BOARD_WEIGHTS, FILES};
use std::ops;
use bincode::{Decode, Encode};
use crate::board::BOARD_TILE_DIM;
use crate::piece::{PieceColor, PiecePerson};

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
pub struct Coordinate {
    pub x: isize, // todo: why is this isize again??
    pub y: isize,
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

impl ops::Add<(isize, isize)> for Coordinate {
    type Output = Coordinate;
    fn add(self, rhs: (isize, isize)) -> Self::Output {
        Self::Output {
            x: self.x + rhs.0,
            y: self.y + rhs.1,
        }
    }
}