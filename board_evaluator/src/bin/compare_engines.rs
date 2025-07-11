use board_evaluator::ChessEngine;
use chess_engine::{Board, BoardSquares, Coordinate, Outcome, PieceColor, PieceWeights, Square};
use rand::prelude::IndexedRandom;
use std::collections::HashSet;

#[derive(Clone, Eq, Hash, PartialEq)]
enum Turn {
    FirstAlgorithmTurn,
    SecondAlgorithmTurn,
}

impl Turn {
    pub fn flip(&self) -> Self {
        match self {
            Turn::FirstAlgorithmTurn => Turn::SecondAlgorithmTurn,
            Turn::SecondAlgorithmTurn => Turn::FirstAlgorithmTurn,
        }
    }
}

fn main() {
    let mut first_wins = 0;
    let mut second_wins = 0;
    let mut draws = 0;
    let mut num_moves = 0;
    let mut loops = 0;
    let mut insufficient_material = 0;
    let mut too_many_moves = 0;

    let mut i = 0;

    let mut rng = rand::rng();

    let mut engine = ChessEngine::new(10);

    let max_moves = 600;

    let board = Board::new(PieceColor::White);

    for first_move in board.get_all_moves_for_turn() {
        // let mut board = board.clone();
        // board.apply_move(&first_move);

        // for second_move in board.get_all_moves_for_turn() {
        {
            for mut turn in [Turn::FirstAlgorithmTurn, Turn::SecondAlgorithmTurn] {
                let mut board = board.clone();
                board.apply_move(&first_move);

                i += 1;

                let mut moves_for_initial_board = 0;

                let mut past_boards: HashSet<(BoardSquares, Turn)> = HashSet::new();

                loop {
                    // println!("{}", board);
                    // println!("{:?}", board.get_all_moves_for_turn());
                    match turn {
                        Turn::FirstAlgorithmTurn => {

                            engine.weights = PieceWeights {
                                pawn: 1.0,
                                knight: 2.5,
                                bishop: 3.0,
                                rook: 4.0,
                                queen: 8.0,
                            };
                                
                            board.apply_move(
                                &engine.next_best_move_natural_minimax_ab_no_eval_sort_id(&board, 3, ), //[1., 4., 2., 3., 5.]
                            );
                        }
                        Turn::SecondAlgorithmTurn => {

                            engine.weights = PieceWeights {
                                pawn: 1.0,
                                knight: 3.0,
                                bishop: 3.5,
                                rook: 5.0,
                                queen: 10.0,
                            };
                            
                            board.apply_move(
                                &engine.next_best_move_natural_minimax_ab_no_eval_sort_id(&board, 3, ), // [1., 3., 2., 2., 8.]
                                // pawn, rook, knight, bishop, queen
                            );
                        }
                    }
                    // println!("Compare Engine Board: \n{}", board);
                    // println!("Compare Engine Turn: {:?}", board.turn);

                    if past_boards.contains(&(board.squares, turn.clone())) {
                        loops += 1;
                        board.apply_move(&board.get_all_moves_for_turn().choose(&mut rng).unwrap());
                        turn = turn.flip();
                    } else {
                        past_boards.insert((board.squares, turn.clone()));
                    }
                    
                    match board.outcome() {
                        Outcome::Playing => num_moves += 1,
                        Outcome::Checkmate { .. } => {
                            match turn {
                                Turn::FirstAlgorithmTurn => first_wins += 1,
                                Turn::SecondAlgorithmTurn => second_wins += 1,
                            }
                            break;
                        }
                        Outcome::Draw => {
                            draws += 1;
                            break;
                        }
                    }

                    if !board.sufficient_material() {
                        insufficient_material += 1;
                        break;
                    }

                    turn = turn.flip();

                    moves_for_initial_board += 1;
                    if moves_for_initial_board > max_moves {
                        too_many_moves += 1;
                        break;
                    }
                }
                println!(
                    "num_moves {}, loops: {}, insufficient_material: {}, too_many_moves: {}, 1st wins: {}, 2nd wins: {}, draws: {}, i: {}",
                    num_moves,
                    loops,
                    insufficient_material,
                    too_many_moves,
                    first_wins,
                    second_wins,
                    draws,
                    i
                );
                // println!("{}", loops);
                // println!("{}", insufficient_material);

            }
        }
    }

    // println!("{}", first_wins);
    // println!("{}", second_wins);
    // println!("{}", num_moves);
    // println!("Draws: {}", draws);

    println!(
        "Ratio of first strategy wins to second {}",
        first_wins as f32 / second_wins as f32
    );
}

// [" ", " ", " ", " ", " ", " ", " ", " "]
// [" ", " ", " ", " ", " ", " ", " ", "♔"]
// [" ", " ", " ", " ", " ", " ", " ", " "]
// [" ", " ", " ", " ", " ", " ", " ", " "]
// [" ", " ", "♙", " ", "♗", " ", "♙", "♙"]
// [" ", " ", "♕", " ", " ", " ", " ", " "]
// ["♚", " ", " ", " ", " ", " ", " ", " "]
// [" ", " ", " ", " ", " ", " ", " ", " "]
