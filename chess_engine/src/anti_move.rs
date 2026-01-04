use bincode::{Decode, Encode};
use crate::board::Board;
use crate::coordinate::Coordinate;
use crate::{MoveType, Side, Square};
use crate::piece::{PieceColor, PiecePerson};
use crate::piece_move::Move;

#[derive(Debug, Clone, PartialEq)]
pub enum AntiMove {
    RegularOrPromote {
        original_position: Coordinate,
        original_square: Square,
        final_position: Coordinate,
        square_taken: Square,
        last_doubled_jumped_pawn: Option<Coordinate>,
    },
    Castle {
        side: Side,
        last_doubled_jumped_pawn: Option<Coordinate>,
    },
    EnPassant {
        original_position: Coordinate,
        final_position: Coordinate,
        last_doubled_jumped_pawn: Option<Coordinate>,
    },
}


impl AntiMove {

    // must be called before the move has been applied to the board
    pub fn from_piece_move(board: &Board, piece_move: &Move) -> AntiMove {
        let last_doubled_jumped_pawn = board.last_doubled_jumped_pawn;
        match *piece_move {
            Move::Regular {
                initial_position,
                final_position,
                ..
            }
            | Move::Promote {
                initial_position,
                final_position,
                ..
            } => {
                AntiMove::RegularOrPromote {
                    original_position: initial_position,
                    original_square: board[initial_position],
                    final_position,
                    square_taken: board[final_position],
                    last_doubled_jumped_pawn,
                }
            }
            Move::Castle { side } => {
                AntiMove::Castle {
                    side,
                    last_doubled_jumped_pawn,
                }
            }
            Move::EnPassant { initial_position, final_position } => {
                AntiMove::EnPassant {
                    original_position: initial_position,
                    final_position,
                    last_doubled_jumped_pawn,
                }
            }
        }
    }
}



