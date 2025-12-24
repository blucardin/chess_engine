pub mod move_serialization;
pub mod coordinate;
pub mod piece;
pub mod board;

use bincode::{Decode, Encode};
// use rand::seq::IndexedRandom;
use std::cmp::PartialEq;
extern crate either;
extern crate approx;

use pgn_reader::{Rank, SanPlus};
use board::BOARD_TILE_DIM;
use coordinate::Coordinate;
use piece::{Piece, PieceColor, PiecePerson};
// 0.9.0

const DEFAULT_BOARD_TILE_SIZE: f32 = 100.0;
pub const DEFAULT_BOARD_HEIGHT: f32 = BOARD_TILE_DIM as f32 * DEFAULT_BOARD_TILE_SIZE;
const PIECES_FOLDER: &str = "pieces-basic-png";

const ROOK_SEARCH_OFFSETS: [(isize, isize); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_SEARCH_OFFSETS: [(isize, isize); 4] = [(1, 1), (-1, 1), (-1, -1), (1, -1)];

const KING_SEARCH_OFFSETS: [(isize, isize); 8] = [
    (1, -1),
    (1, 0),
    (1, 1),
    (0, -1),
    (0, 1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
];

const KNIGHT_SEARCH_OFFSETS: [(isize, isize); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];

const BOARD_WEIGHTS: [f32; 8] = [0., 0.3, 0.6, 0.9, 0.9, 0.6, 0.3, 0.];

pub type Transposition = [[[bool; 10]; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

pub type SmallTransposition = [[[bool; 12]; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

pub type BoardSquares = [[Square; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize];

// pub type PieceWeights = [f32 ; 5];

pub struct PieceWeights {
    pub pawn: f32,
    pub knight: f32,
    pub bishop: f32,
    pub rook: f32,
    pub queen: f32,
}

const FILES: [&'static str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];


#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum Side {
    QueenSide,
    KingsSide,
}

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

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Square {
    Filled(Piece),
    Empty,
    Boundary,
}

#[derive(Encode, Decode, PartialEq, Eq, Clone, Copy, Debug)]
pub enum MoveType {
    Jump,
    Take,
}

#[derive(Debug)]
pub enum Outcome {
    Playing,
    Checkmate { winner: PieceColor },
    Draw,
}

pub const POSSIBLE_PAWN_PROMOTES: [PiecePerson; 4] = [
    PiecePerson::Queen,
    PiecePerson::Rook { moved: true },
    PiecePerson::Bishop,
    PiecePerson::Knight,
];

fn rank_to_y(rank: Rank) -> usize {
    (BOARD_TILE_DIM as usize - 1) - rank as usize
}

#[cfg(test)]
mod tests {
    use approx::{abs_diff_eq, assert_abs_diff_eq};
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use pgn_reader;
    use pgn_reader::{BufferedReader, Skip, Visitor};
    use crate::board::Board;

    #[test]
    fn test_move_delta_against_regular_board_eval() {
        const DEFAULT_PIECE_WEIGHTS: PieceWeights = PieceWeights {
            pawn: 1.0,
            knight: 2.0,
            bishop: 3.0,
            rook: 4.0,
            queen: 5.0,
        };
        //     [
        // }1., 4., 2., 3., 5.];
        struct MoveCounter {
            value: f32,
            board: Board,
        }

        impl MoveCounter {
            fn new() -> MoveCounter {
                MoveCounter {
                    value: 0.,
                    board: Board::new(PieceColor::White),
                }
            }
        }

        impl Visitor for MoveCounter {
            type Result = usize;

            fn begin_game(&mut self) {
                self.board = Board::new(PieceColor::White);
                self.value = self.board.natural_score(&DEFAULT_PIECE_WEIGHTS);
            }

            fn san(&mut self, san_plus: SanPlus) {
                let piece_move = self.board.move_from_san(san_plus.san);
                // println!("piece_move {:?}", piece_move);
                // println!("board {}", self.board);
                let score_delta = self.board.score_delta(&piece_move, &DEFAULT_PIECE_WEIGHTS);

                self.value += score_delta;

                self.board.apply_move(&piece_move);
                let natural_score = self.board.natural_score(&DEFAULT_PIECE_WEIGHTS);

                let epsilon = 0.0000030;

                if !abs_diff_eq!(natural_score, self.value, epsilon = epsilon) {
                    println!("{}", self.board);
                    println!("{:?}", piece_move);
                    println!("Score Delta: {:?}", score_delta);
                }

                assert_abs_diff_eq!(natural_score, self.value, epsilon = epsilon);
            }

            fn begin_variation(&mut self) -> Skip {
                Skip(true) // stay in the mainline
            }

            fn end_game(&mut self) -> Self::Result {
                1
            }
        }

        // https://lichess.org/d5kge8qf
        // My own
        // https://lichess.org/4si2z6iq

        let pgn = br#"
1. e4 e5 2. Bc4 Nf6 3. d3 Bc5 4. h3 d6 5. a3 Nc6 6. Ne2 Be6 7. Bxe6 fxe6 8. b4 Bb6 9. Nbc3 d5 10. exd5 exd5 11. Bg5 Qd6 12. Bxf6 Qxf6 13. Nxd5 Qxf2+ 14. Kd2 O-O-O 15. Nxb6+ axb6 16. g4 e4 17. Rf1 Qd4 18. Nxd4 Nxd4 19. c3 Nb5 20. d4 c5 21. bxc5 bxc5 22. Kc2 cxd4 23. Qb1 d3+ 24. Kd2 Rd5 25. Rf5 Rxf5 26. gxf5 Nd6 27. Qf1 Rf8 28. Qg2 Rd8 29. Qxg7 e3+ 30. Ke1 Nxf5 31. Qxh7 d2+ 32. Ke2 Ng3+ 33. Kxe3 d1=Q 34. Rxd1 Rxd1 35. Qg8+ Kc7 36. Qxg3+ Kb6 37. Qg6+ Ka7 38. h4 Re1+ 39. Kd2 Ra1 40. Qd6 Ra2+ 41. Kd3 Ra1 42. Kc4 Rh1 43. Qd4+ Ka8 44. a4 Ra1 45. Qd7 Rh1 46. Qd8+ Ka7 47. a5 Ra1 48. h5 b5+ 49. Kxb5 Rb1+ 50. Kc4 Rb8 51. Qc7+ Rb7 52. Qxb7+ Kxb7 53. h6 Ka6 54. h7 Kxa5 55. h8=Q Kb6 56. Qe5 Kc6 57. Kb4 Kd7 58. Qf6 Kc7 59. c4 Kd7 60. c5 Kc7 61. c6 Kb6 62. Qg6 Kc7 63. Kc5 Kd8 64. Qh7 Kc8 65. Qf5+ Kb8 66. Kd6 Ka7 67. c7 Kb6 68. c8=Q Ka7 1-0

1. e4 d6 2. e5 f5 3. exf6

1. e4 Nc6 2. d4 e5 3. d5 Nce7 4. Nf3 Ng6 5. Bc4 Nf6 6. Nc3 Bc5 7. O-O h6 8. Be3 d6 9. Bxc5 dxc5 10. a3 a6 11. b4 b5 12. Bb3 c4 13. Ba2 Qd6 14. a4 O-O 15. axb5 Qxb4 16. Qd2 axb5 17. Rfb1 Qc5 18. Rxb5 Qd6 19. Rbb1 Rd8 20. Bxc4 Rxa1 21. Rxa1 Bb7 22. Ra7 Ba8 23. Nb5 Qc5 24. Rxc7 Qb6 25. Qc3 Nxe4 26. Qd3 Nxf2 27. Qe2 Ng4+ 28. Kh1 Nf2+ 29. Kg1 Nd3+ 30. Kf1 Ndf4 31. Qe4 Bxd5 32. Bxd5 Qxb5+ 33. Bc4 Rd1+ 34. Kf2 Qb6+ 35. Qe3 Qxc7 36. Qb3 Qc5+ 37. Kg3 Nd5 38. c3 Rh1 39. Bxd5 Nh8 40. Nxe5 Qe3+ 41. Nf3 h5 42. h4 Qe7 43. Qb8+ Kh7 44. c4 Ng6 45. Ng5+ Kh6 46. Nxf7+ Kh7 47. Ng5+ Kh6 48. Qg8 Qe5+ 49. Kf2 Qe1+ 50. Kf3 Nxh4+ 51. Kf4 Rf1+ 52. Nf3 g5+ 53. Qxg5+ Kh7 54. Qxh5+ Kg7 55. Qf7+ Kh6 56. Qf8+ Kh5 57. Bf7+ Ng6+ 58. Bxg6+ Kxg6 59. Qf5+ Kg7 60. Qg5+ Kf7 61. Qf5+ Kg7 62. Qg5+ Kf7 63. Qf5+ Ke7 64. Qg5+ Kd7 65. Qg7+ Kc6 66. Qf6+ Kc5 0-1
        "#;
        let mut reader = BufferedReader::new_cursor(&pgn[..]);

        let mut counter = MoveCounter::new();
        let moves = reader.read_game(&mut counter);

        // assert_eq!(add(1, 2), 3);
    }
}
