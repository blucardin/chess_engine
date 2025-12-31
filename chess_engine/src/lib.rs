mod anti_move;
pub mod board;
pub mod coordinate;
pub mod move_serialization;
pub mod piece;
pub mod piece_move;

use bincode::{Decode, Encode};
// use rand::seq::IndexedRandom;
use approx::{abs_diff_eq, assert_abs_diff_eq};
use std::cmp::PartialEq;

extern crate approx;
extern crate either;

use crate::anti_move::AntiMove;
use crate::board::Board;
pub use crate::piece_move::Move;
use board::BOARD_TILE_DIM;
use coordinate::Coordinate;
use pgn_reader::{BufferedReader, Rank, SanPlus, Skip, Visitor};
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

#[derive(Encode, Decode, Debug, Clone, Copy, PartialEq)]
pub enum Side {
    QueenSide,
    KingsSide,
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
#[derive(PartialEq)]
pub enum Outcome {
    Playing,
    Checkmate { winner: PieceColor },
    Draw,
}

impl Outcome {
    fn to_option_pgn_reader_outcome(self) -> Option<pgn_reader::Outcome> {
        match self {
            Outcome::Playing => {
                None
                // panic!("Playing state does not map to pgn_reader outcome")
            }
            Outcome::Checkmate { winner } => {
                let winner = match winner {
                    PieceColor::Black => {pgn_reader::Color::Black}
                    PieceColor::White => {pgn_reader::Color::White}
                };
                Some(pgn_reader::Outcome::Decisive { winner })
            }
            Outcome::Draw => {
                Some(pgn_reader::Outcome::Draw)
            }
        }
    }

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
    use crate::board::Board;
    use pgn_reader;
    use pgn_reader::{BufferedReader, RawTag, Skip, Visitor};

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
{https://lichess.org/d5kge8qf}
1. e4 e5 2. Bc4 Nf6 3. d3 Bc5 4. h3 d6 5. a3 Nc6 6. Ne2 Be6 7. Bxe6 fxe6 8. b4 Bb6 9. Nbc3 d5 10. exd5 exd5 11. Bg5 Qd6 12. Bxf6 Qxf6 13. Nxd5 Qxf2+ 14. Kd2 O-O-O 15. Nxb6+ axb6 16. g4 e4 17. Rf1 Qd4 18. Nxd4 Nxd4 19. c3 Nb5 20. d4 c5 21. bxc5 bxc5 22. Kc2 cxd4 23. Qb1 d3+ 24. Kd2 Rd5 25. Rf5 Rxf5 26. gxf5 Nd6 27. Qf1 Rf8 28. Qg2 Rd8 29. Qxg7 e3+ 30. Ke1 Nxf5 31. Qxh7 d2+ 32. Ke2 Ng3+ 33. Kxe3 d1=Q 34. Rxd1 Rxd1 35. Qg8+ Kc7 36. Qxg3+ Kb6 37. Qg6+ Ka7 38. h4 Re1+ 39. Kd2 Ra1 40. Qd6 Ra2+ 41. Kd3 Ra1 42. Kc4 Rh1 43. Qd4+ Ka8 44. a4 Ra1 45. Qd7 Rh1 46. Qd8+ Ka7 47. a5 Ra1 48. h5 b5+ 49. Kxb5 Rb1+ 50. Kc4 Rb8 51. Qc7+ Rb7 52. Qxb7+ Kxb7 53. h6 Ka6 54. h7 Kxa5 55. h8=Q Kb6 56. Qe5 Kc6 57. Kb4 Kd7 58. Qf6 Kc7 59. c4 Kd7 60. c5 Kc7 61. c6 Kb6 62. Qg6 Kc7 63. Kc5 Kd8 64. Qh7 Kc8 65. Qf5+ Kb8 66. Kd6 Ka7 67. c7 Kb6 68. c8=Q Ka7 1-0

1. e4 d6 2. e5 f5 3. exf6

{https://lichess.org/4si2z6iq}
1. e4 Nc6 2. d4 e5 3. d5 Nce7 4. Nf3 Ng6 5. Bc4 Nf6 6. Nc3 Bc5 7. O-O h6 8. Be3 d6 9. Bxc5 dxc5 10. a3 a6 11. b4 b5 12. Bb3 c4 13. Ba2 Qd6 14. a4 O-O 15. axb5 Qxb4 16. Qd2 axb5 17. Rfb1 Qc5 18. Rxb5 Qd6 19. Rbb1 Rd8 20. Bxc4 Rxa1 21. Rxa1 Bb7 22. Ra7 Ba8 23. Nb5 Qc5 24. Rxc7 Qb6 25. Qc3 Nxe4 26. Qd3 Nxf2 27. Qe2 Ng4+ 28. Kh1 Nf2+ 29. Kg1 Nd3+ 30. Kf1 Ndf4 31. Qe4 Bxd5 32. Bxd5 Qxb5+ 33. Bc4 Rd1+ 34. Kf2 Qb6+ 35. Qe3 Qxc7 36. Qb3 Qc5+ 37. Kg3 Nd5 38. c3 Rh1 39. Bxd5 Nh8 40. Nxe5 Qe3+ 41. Nf3 h5 42. h4 Qe7 43. Qb8+ Kh7 44. c4 Ng6 45. Ng5+ Kh6 46. Nxf7+ Kh7 47. Ng5+ Kh6 48. Qg8 Qe5+ 49. Kf2 Qe1+ 50. Kf3 Nxh4+ 51. Kf4 Rf1+ 52. Nf3 g5+ 53. Qxg5+ Kh7 54. Qxh5+ Kg7 55. Qf7+ Kh6 56. Qf8+ Kh5 57. Bf7+ Ng6+ 58. Bxg6+ Kxg6 59. Qf5+ Kg7 60. Qg5+ Kf7 61. Qf5+ Kg7 62. Qg5+ Kf7 63. Qf5+ Ke7 64. Qg5+ Kd7 65. Qg7+ Kc6 66. Qf6+ Kc5 0-1

{https://lichess.org/vb3w3rmn}
1. e4 c5 2. f4 d5 3. exd5 Qxd5 4. Nc3 Qd8 5. Bc4 Bf5 6. d3 a6 7. g4 Bd7 8. a4 e6 9. Bd2 Bc6 10. Nf3 Bxf3 11. Qxf3 Qh4+ 12. Qg3 Qxg3+ 13. hxg3 Nc6 14. O-O-O O-O-O 15. f5 Ne5 16. fxe6 Nxc4 17. dxc4 fxe6 18. Rde1 Bd6 19. Bf4 Bxf4+ 20. gxf4 Nh6 21. g5 Nf5 22. Rxe6 Rd4 23. Rf1 Rxc4 24. Re5 g6 25. Kd2 Rd8+ 26. Kc1 Rd7 27. Nd5 Rd6 28. Ne7+ Nxe7 29. Rxe7 Rd7 30. Rxd7 Kxd7 31. b3 Re4 32. Kb2 Ke6 33. Kc3 Kf5 34. Rh1 Re7 35. Rf1 Re4 36. Rh1 Rxf4 37. Rxh7 Kxg5 38. Rxb7 Rf6 39. Rc7 Kf4 40. Rxc5 g5 41. b4 g4 42. Rc4+ Kf3 43. Rc5 Rg6 44. Rf5+ Kg2 45. b5 axb5 46. axb5 g3 47. Kb4 Kh1 48. Rd5 g2 49. Rd1+ g1=Q 50. Rxg1+ Kxg1 51. c4 Kf2 52. c5 Ke3 53. b6 Kd4 54. b7 Rg1 55. Kb5 Rb1+ 56. Kc6 Rb4 57. Kc7 Kxc5 58. b8=Q Rxb8 59. Kxb8 1/2-1/2

[Site "https://lichess.org/3wkrqhx8"]
1. d4 e6 2. c4 c5 3. d5 exd5 4. cxd5 d6 5. e4 Nf6 6. Bg5 Be7 7. Nc3 h6 8. Bh4 b5 9. Bxb5+ Bd7 10. Bd3 a6 11. Nf3 Qa5 12. O-O O-O 13. Bc2 Bg4 14. Qd3 Qb4 15. Rab1 Nxe4 16. Bxe7 Re8 17. Nxe4 Bf5 18. a3 Bxe4 19. Qxe4 Qxe4 20. Bxe4 Rxe7 21. Rfe1 Rc7 22. Bf5 a5 23. Re8# 1-0

[Site "https://lichess.org/nxeztu97"]
1. e4 c5 2. Nf3 Nc6 3. Bc4 Nf6 4. Ng5 d5 5. exd5 Nxd5 6. Qf3 f6 7. Bxd5 fxg5 8. Qf7+ Kd7 9. Qf5+ Kc7 10. Qxg5 Nd4 11. Qg3+ Kb6 12. Bb3 e6 13. O-O Ne2+ 14. Kh1 Nxg3+ 15. fxg3 Bd6 16. Nc3 Rf8 17. Rxf8 Qxf8 18. Kg1 c4 19. Bxc4 Bc5+ 20. d4 Bxd4+ 21. Kh1 Bxc3 22. bxc3 Kc5 23. Bb3 Qf1# 0-1

[Site "https://lichess.org/5tdppn3c"]
1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. Nc3 Nf6 5. d3 d6 6. Bg5 h6 7. Bxf6 Qxf6 8. O-O Ne7 9. Nd5 Nxd5 10. Bxd5 h5 11. h3 Bg4 12. Bxb7 Rb8 13. Bc6+ Ke7 14. hxg4 hxg4 15. Nd2 Qh4 16. g3 Qh2# 0-1

[Site "https://lichess.org/cm1ummbb"]
1. c4 Nf6 2. b3 Nc6 3. Bb2 e5 4. g3 d5 5. Bg2 dxc4 6. Nf3 cxb3 7. Qxb3 Be6 8. Qxb7 Bd5 9. Qa6 Nb4 10. Qa4+ Nc6 11. O-O Bd6 12. Nc3 O-O 13. Nxd5 Nxd5 14. Qxc6 Nb4 15. Qc4 Qd7 16. Nxe5 Qe7 17. Nf3 Rab8 18. Qc3 Nd5 19. Qxg7# 1-0

[Site "https://lichess.org/d47vphcq"]
1. f4 b6 2. e4 Bb7 3. d3 e6 4. Nf3 Qe7 5. Be2 Nc6 6. Be3 O-O-O 7. Nc3 f6 8. Qd2 d6 9. O-O-O Qf7 10. Kb1 g6 11. d4 Bg7 12. e5 fxe5 13. fxe5 dxe5 14. dxe5 Rxd2 15. Rxd2 Nxe5 16. Nxe5 Bxe5 17. Rf1 Qg7 18. Bg4 Kb8 19. Bxe6 Ne7 20. Rf7 Qg8 21. Rxe7 Qg7 22. Rxg7 Bxg7 23. Bd4 Bxd4 24. Rxd4 c5 25. Rd2 Re8 26. Bg4 Re1+ 27. Nd1 Bxg2 28. Rxg2 Kb7 29. Kc1 Ka6 30. Kd2 Re8 31. Be2+ b5 32. Nc3 c4 33. Rg5 b4 34. Nd5 Ka5 35. Nc7+ Ka4 36. Nxe8 c3+ 37. bxc3 bxc3+ 38. Kxc3 Ka3 39. Ra5# 1-0

[Site "https://lichess.org/h0c3itl6"]
1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 b5 5. Bb3 Nf6 6. O-O Nxe4 7. Re1 d5 8. d4 Be6 9. Nxe5 Nxd4 10. Qxd4 Bc5 11. Qd3 Bxf2+ 12. Kf1 Bxe1 13. Kxe1 Qh4+ 14. g3 Qxh2 15. Bf4 Qf2+ 16. Kd1 Qg1+ 17. Ke2 g5 18. Bxd5 Qg2+ 19. Ke1 Bxd5 20. Qxd5 Qf2+ 21. Kd1 Qf1# 0-1

        "#;

        let mut reader = BufferedReader::new_cursor(&pgn[..]);

        let mut counter = MoveCounter::new();
        let moves = reader.read_all(&mut counter);

        // assert_eq!(add(1, 2), 3);
    }

    #[test]
    fn test_anti_move_sets_board_back_to_og_state() {
        struct MoveCounter {
            anti_moves: Vec<AntiMove>,
            board: Board,
        }

        impl MoveCounter {
            fn new() -> MoveCounter {
                MoveCounter {
                    anti_moves: vec![],
                    board: Board::new(PieceColor::White),
                }
            }
        }

        impl Visitor for MoveCounter {
            type Result = usize;

            fn begin_game(&mut self) {
                self.board = Board::new(PieceColor::White);
            }

            fn san(&mut self, san_plus: SanPlus) {
                let piece_move = self.board.move_from_san(san_plus.san);
                let anti_move = AntiMove::from_piece_move(&self.board, &piece_move);

                self.anti_moves.push(anti_move.clone());
                // println!("piece_move {:?}", piece_move);
                // println!("board \n{}", self.board);
                let board_before_move = self.board.clone();
                // println!("Before move: \n{}", self.board);

                self.board.apply_move(&piece_move);

                // println!("After move: \n{}", self.board);

                let mut board_after_anti_move = self.board.clone();
                board_after_anti_move.apply_anti_move(&anti_move);
                // println!("After antimove: \n{}", board_after_anti_move);

                // if board_before_move.squares != board_after_anti_move.squares {
                //     for idy in 0..board_before_move.squares.len() {
                //         for idx in 0..board_before_move.squares[idy].len() {
                //             if board_before_move.squares[idy][idx] != board_after_anti_move.squares[idy][idx] {
                //                 println!("Differing piece, \nbefore_move: {:?}, \nAfter anti move{:?}", board_before_move.squares[idy][idx], board_after_anti_move.squares[idy][idx])
                //             }
                //         }
                //     }
                // }

                assert_eq!(board_before_move.squares, board_after_anti_move.squares);
            }
            fn begin_variation(&mut self) -> Skip {
                Skip(true) // stay in the mainline
            }

            fn end_game(&mut self) -> Self::Result {

                for anti_move in self.anti_moves.iter().rev() {
                    self.board.apply_anti_move(&anti_move);
                    // println!("After each antimove: \n{}", self.board);
                }

                // println!("After antimoves: \n{}", self.board);
                // println!("New Board: \n{}", Board::new(PieceColor::White));

                assert_eq!(self.board.squares, Board::new(PieceColor::White).squares);

                self.board = Board::new(PieceColor::White);
                self.anti_moves.clear();

                1
            }
        }

        let pgn = br#"
{https://lichess.org/d5kge8qf}
1. e4 e5 2. Bc4 Nf6 3. d3 Bc5 4. h3 d6 5. a3 Nc6 6. Ne2 Be6 7. Bxe6 fxe6 8. b4 Bb6 9. Nbc3 d5 10. exd5 exd5 11. Bg5 Qd6 12. Bxf6 Qxf6 13. Nxd5 Qxf2+ 14. Kd2 O-O-O 15. Nxb6+ axb6 16. g4 e4 17. Rf1 Qd4 18. Nxd4 Nxd4 19. c3 Nb5 20. d4 c5 21. bxc5 bxc5 22. Kc2 cxd4 23. Qb1 d3+ 24. Kd2 Rd5 25. Rf5 Rxf5 26. gxf5 Nd6 27. Qf1 Rf8 28. Qg2 Rd8 29. Qxg7 e3+ 30. Ke1 Nxf5 31. Qxh7 d2+ 32. Ke2 Ng3+ 33. Kxe3 d1=Q 34. Rxd1 Rxd1 35. Qg8+ Kc7 36. Qxg3+ Kb6 37. Qg6+ Ka7 38. h4 Re1+ 39. Kd2 Ra1 40. Qd6 Ra2+ 41. Kd3 Ra1 42. Kc4 Rh1 43. Qd4+ Ka8 44. a4 Ra1 45. Qd7 Rh1 46. Qd8+ Ka7 47. a5 Ra1 48. h5 b5+ 49. Kxb5 Rb1+ 50. Kc4 Rb8 51. Qc7+ Rb7 52. Qxb7+ Kxb7 53. h6 Ka6 54. h7 Kxa5 55. h8=Q Kb6 56. Qe5 Kc6 57. Kb4 Kd7 58. Qf6 Kc7 59. c4 Kd7 60. c5 Kc7 61. c6 Kb6 62. Qg6 Kc7 63. Kc5 Kd8 64. Qh7 Kc8 65. Qf5+ Kb8 66. Kd6 Ka7 67. c7 Kb6 68. c8=Q Ka7 1-0

1. e4 d6 2. e5 f5 3. exf6

{https://lichess.org/4si2z6iq}
1. e4 Nc6 2. d4 e5 3. d5 Nce7 4. Nf3 Ng6 5. Bc4 Nf6 6. Nc3 Bc5 7. O-O h6 8. Be3 d6 9. Bxc5 dxc5 10. a3 a6 11. b4 b5 12. Bb3 c4 13. Ba2 Qd6 14. a4 O-O 15. axb5 Qxb4 16. Qd2 axb5 17. Rfb1 Qc5 18. Rxb5 Qd6 19. Rbb1 Rd8 20. Bxc4 Rxa1 21. Rxa1 Bb7 22. Ra7 Ba8 23. Nb5 Qc5 24. Rxc7 Qb6 25. Qc3 Nxe4 26. Qd3 Nxf2 27. Qe2 Ng4+ 28. Kh1 Nf2+ 29. Kg1 Nd3+ 30. Kf1 Ndf4 31. Qe4 Bxd5 32. Bxd5 Qxb5+ 33. Bc4 Rd1+ 34. Kf2 Qb6+ 35. Qe3 Qxc7 36. Qb3 Qc5+ 37. Kg3 Nd5 38. c3 Rh1 39. Bxd5 Nh8 40. Nxe5 Qe3+ 41. Nf3 h5 42. h4 Qe7 43. Qb8+ Kh7 44. c4 Ng6 45. Ng5+ Kh6 46. Nxf7+ Kh7 47. Ng5+ Kh6 48. Qg8 Qe5+ 49. Kf2 Qe1+ 50. Kf3 Nxh4+ 51. Kf4 Rf1+ 52. Nf3 g5+ 53. Qxg5+ Kh7 54. Qxh5+ Kg7 55. Qf7+ Kh6 56. Qf8+ Kh5 57. Bf7+ Ng6+ 58. Bxg6+ Kxg6 59. Qf5+ Kg7 60. Qg5+ Kf7 61. Qf5+ Kg7 62. Qg5+ Kf7 63. Qf5+ Ke7 64. Qg5+ Kd7 65. Qg7+ Kc6 66. Qf6+ Kc5 0-1

{https://lichess.org/vb3w3rmn}
1. e4 c5 2. f4 d5 3. exd5 Qxd5 4. Nc3 Qd8 5. Bc4 Bf5 6. d3 a6 7. g4 Bd7 8. a4 e6 9. Bd2 Bc6 10. Nf3 Bxf3 11. Qxf3 Qh4+ 12. Qg3 Qxg3+ 13. hxg3 Nc6 14. O-O-O O-O-O 15. f5 Ne5 16. fxe6 Nxc4 17. dxc4 fxe6 18. Rde1 Bd6 19. Bf4 Bxf4+ 20. gxf4 Nh6 21. g5 Nf5 22. Rxe6 Rd4 23. Rf1 Rxc4 24. Re5 g6 25. Kd2 Rd8+ 26. Kc1 Rd7 27. Nd5 Rd6 28. Ne7+ Nxe7 29. Rxe7 Rd7 30. Rxd7 Kxd7 31. b3 Re4 32. Kb2 Ke6 33. Kc3 Kf5 34. Rh1 Re7 35. Rf1 Re4 36. Rh1 Rxf4 37. Rxh7 Kxg5 38. Rxb7 Rf6 39. Rc7 Kf4 40. Rxc5 g5 41. b4 g4 42. Rc4+ Kf3 43. Rc5 Rg6 44. Rf5+ Kg2 45. b5 axb5 46. axb5 g3 47. Kb4 Kh1 48. Rd5 g2 49. Rd1+ g1=Q 50. Rxg1+ Kxg1 51. c4 Kf2 52. c5 Ke3 53. b6 Kd4 54. b7 Rg1 55. Kb5 Rb1+ 56. Kc6 Rb4 57. Kc7 Kxc5 58. b8=Q Rxb8 59. Kxb8 1/2-1/2

[Site "https://lichess.org/3wkrqhx8"]
1. d4 e6 2. c4 c5 3. d5 exd5 4. cxd5 d6 5. e4 Nf6 6. Bg5 Be7 7. Nc3 h6 8. Bh4 b5 9. Bxb5+ Bd7 10. Bd3 a6 11. Nf3 Qa5 12. O-O O-O 13. Bc2 Bg4 14. Qd3 Qb4 15. Rab1 Nxe4 16. Bxe7 Re8 17. Nxe4 Bf5 18. a3 Bxe4 19. Qxe4 Qxe4 20. Bxe4 Rxe7 21. Rfe1 Rc7 22. Bf5 a5 23. Re8# 1-0

[Site "https://lichess.org/nxeztu97"]
1. e4 c5 2. Nf3 Nc6 3. Bc4 Nf6 4. Ng5 d5 5. exd5 Nxd5 6. Qf3 f6 7. Bxd5 fxg5 8. Qf7+ Kd7 9. Qf5+ Kc7 10. Qxg5 Nd4 11. Qg3+ Kb6 12. Bb3 e6 13. O-O Ne2+ 14. Kh1 Nxg3+ 15. fxg3 Bd6 16. Nc3 Rf8 17. Rxf8 Qxf8 18. Kg1 c4 19. Bxc4 Bc5+ 20. d4 Bxd4+ 21. Kh1 Bxc3 22. bxc3 Kc5 23. Bb3 Qf1# 0-1

[Site "https://lichess.org/5tdppn3c"]
1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. Nc3 Nf6 5. d3 d6 6. Bg5 h6 7. Bxf6 Qxf6 8. O-O Ne7 9. Nd5 Nxd5 10. Bxd5 h5 11. h3 Bg4 12. Bxb7 Rb8 13. Bc6+ Ke7 14. hxg4 hxg4 15. Nd2 Qh4 16. g3 Qh2# 0-1

[Site "https://lichess.org/cm1ummbb"]
1. c4 Nf6 2. b3 Nc6 3. Bb2 e5 4. g3 d5 5. Bg2 dxc4 6. Nf3 cxb3 7. Qxb3 Be6 8. Qxb7 Bd5 9. Qa6 Nb4 10. Qa4+ Nc6 11. O-O Bd6 12. Nc3 O-O 13. Nxd5 Nxd5 14. Qxc6 Nb4 15. Qc4 Qd7 16. Nxe5 Qe7 17. Nf3 Rab8 18. Qc3 Nd5 19. Qxg7# 1-0

[Site "https://lichess.org/d47vphcq"]
1. f4 b6 2. e4 Bb7 3. d3 e6 4. Nf3 Qe7 5. Be2 Nc6 6. Be3 O-O-O 7. Nc3 f6 8. Qd2 d6 9. O-O-O Qf7 10. Kb1 g6 11. d4 Bg7 12. e5 fxe5 13. fxe5 dxe5 14. dxe5 Rxd2 15. Rxd2 Nxe5 16. Nxe5 Bxe5 17. Rf1 Qg7 18. Bg4 Kb8 19. Bxe6 Ne7 20. Rf7 Qg8 21. Rxe7 Qg7 22. Rxg7 Bxg7 23. Bd4 Bxd4 24. Rxd4 c5 25. Rd2 Re8 26. Bg4 Re1+ 27. Nd1 Bxg2 28. Rxg2 Kb7 29. Kc1 Ka6 30. Kd2 Re8 31. Be2+ b5 32. Nc3 c4 33. Rg5 b4 34. Nd5 Ka5 35. Nc7+ Ka4 36. Nxe8 c3+ 37. bxc3 bxc3+ 38. Kxc3 Ka3 39. Ra5# 1-0

[Site "https://lichess.org/h0c3itl6"]
1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 b5 5. Bb3 Nf6 6. O-O Nxe4 7. Re1 d5 8. d4 Be6 9. Nxe5 Nxd4 10. Qxd4 Bc5 11. Qd3 Bxf2+ 12. Kf1 Bxe1 13. Kxe1 Qh4+ 14. g3 Qxh2 15. Bf4 Qf2+ 16. Kd1 Qg1+ 17. Ke2 g5 18. Bxd5 Qg2+ 19. Ke1 Bxd5 20. Qxd5 Qf2+ 21. Kd1 Qf1# 0-1
        "#;

        let mut reader = BufferedReader::new_cursor(&pgn[..]);

        let mut counter = MoveCounter::new();
        let moves = reader.read_all(&mut counter);
    }

    #[test]
    fn test_checkmate() {
        struct MoveCounter {
            board: Board,
        }

        impl MoveCounter {
            fn new() -> MoveCounter {
                MoveCounter {
                    board: Board::new(PieceColor::White),
                }
            }
        }

        impl Visitor for MoveCounter {
            type Result = usize;

            fn begin_game(&mut self) {
                self.board = Board::new(PieceColor::White);
            }

            fn san(&mut self, san_plus: SanPlus) {
                assert_eq!(self.board.outcome(), Outcome::Playing);
                let piece_move = self.board.move_from_san(san_plus.san);
                self.board.apply_move(&piece_move);
            }
            fn begin_variation(&mut self) -> Skip {
                Skip(true) // stay in the mainline
            }

            fn end_game(&mut self) -> Self::Result {
                1
            }

            fn outcome(&mut self, outcome: Option<pgn_reader::Outcome>) {
                assert_eq!(self.board.outcome().to_option_pgn_reader_outcome(), outcome);
            }
        }

        let pgn = br#"
[Site "https://lichess.org/3wkrqhx8"]
1. d4 e6 2. c4 c5 3. d5 exd5 4. cxd5 d6 5. e4 Nf6 6. Bg5 Be7 7. Nc3 h6 8. Bh4 b5 9. Bxb5+ Bd7 10. Bd3 a6 11. Nf3 Qa5 12. O-O O-O 13. Bc2 Bg4 14. Qd3 Qb4 15. Rab1 Nxe4 16. Bxe7 Re8 17. Nxe4 Bf5 18. a3 Bxe4 19. Qxe4 Qxe4 20. Bxe4 Rxe7 21. Rfe1 Rc7 22. Bf5 a5 23. Re8# 1-0

[Site "https://lichess.org/nxeztu97"]
1. e4 c5 2. Nf3 Nc6 3. Bc4 Nf6 4. Ng5 d5 5. exd5 Nxd5 6. Qf3 f6 7. Bxd5 fxg5 8. Qf7+ Kd7 9. Qf5+ Kc7 10. Qxg5 Nd4 11. Qg3+ Kb6 12. Bb3 e6 13. O-O Ne2+ 14. Kh1 Nxg3+ 15. fxg3 Bd6 16. Nc3 Rf8 17. Rxf8 Qxf8 18. Kg1 c4 19. Bxc4 Bc5+ 20. d4 Bxd4+ 21. Kh1 Bxc3 22. bxc3 Kc5 23. Bb3 Qf1# 0-1

[Site "https://lichess.org/5tdppn3c"]
1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. Nc3 Nf6 5. d3 d6 6. Bg5 h6 7. Bxf6 Qxf6 8. O-O Ne7 9. Nd5 Nxd5 10. Bxd5 h5 11. h3 Bg4 12. Bxb7 Rb8 13. Bc6+ Ke7 14. hxg4 hxg4 15. Nd2 Qh4 16. g3 Qh2# 0-1

[Site "https://lichess.org/cm1ummbb"]
1. c4 Nf6 2. b3 Nc6 3. Bb2 e5 4. g3 d5 5. Bg2 dxc4 6. Nf3 cxb3 7. Qxb3 Be6 8. Qxb7 Bd5 9. Qa6 Nb4 10. Qa4+ Nc6 11. O-O Bd6 12. Nc3 O-O 13. Nxd5 Nxd5 14. Qxc6 Nb4 15. Qc4 Qd7 16. Nxe5 Qe7 17. Nf3 Rab8 18. Qc3 Nd5 19. Qxg7# 1-0

[Site "https://lichess.org/d47vphcq"]
1. f4 b6 2. e4 Bb7 3. d3 e6 4. Nf3 Qe7 5. Be2 Nc6 6. Be3 O-O-O 7. Nc3 f6 8. Qd2 d6 9. O-O-O Qf7 10. Kb1 g6 11. d4 Bg7 12. e5 fxe5 13. fxe5 dxe5 14. dxe5 Rxd2 15. Rxd2 Nxe5 16. Nxe5 Bxe5 17. Rf1 Qg7 18. Bg4 Kb8 19. Bxe6 Ne7 20. Rf7 Qg8 21. Rxe7 Qg7 22. Rxg7 Bxg7 23. Bd4 Bxd4 24. Rxd4 c5 25. Rd2 Re8 26. Bg4 Re1+ 27. Nd1 Bxg2 28. Rxg2 Kb7 29. Kc1 Ka6 30. Kd2 Re8 31. Be2+ b5 32. Nc3 c4 33. Rg5 b4 34. Nd5 Ka5 35. Nc7+ Ka4 36. Nxe8 c3+ 37. bxc3 bxc3+ 38. Kxc3 Ka3 39. Ra5# 1-0

[Site "https://lichess.org/h0c3itl6"]
1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 b5 5. Bb3 Nf6 6. O-O Nxe4 7. Re1 d5 8. d4 Be6 9. Nxe5 Nxd4 10. Qxd4 Bc5 11. Qd3 Bxf2+ 12. Kf1 Bxe1 13. Kxe1 Qh4+ 14. g3 Qxh2 15. Bf4 Qf2+ 16. Kd1 Qg1+ 17. Ke2 g5 18. Bxd5 Qg2+ 19. Ke1 Bxd5 20. Qxd5 Qf2+ 21. Kd1 Qf1# 0-1
       "#;

        let mut reader = BufferedReader::new_cursor(&pgn[..]);

        let mut counter = MoveCounter::new();
        let _ = reader.read_all(&mut counter);
    }
}
