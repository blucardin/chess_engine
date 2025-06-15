use crate::board_evaluator::BoardEvaluator;
use chess_engine::{Board, Move, PieceColor};

mod board_evaluator;

pub struct ChessEngine {
    evaluator: BoardEvaluator,
}

impl ChessEngine {
    pub fn next_best_move_shallow(&self, board: &Board) -> Move {
        let possible_moves = board.get_all_moves_for_turn();
        let mut moves: Vec<_> = possible_moves
            .iter()
            .map(|piece_move| (piece_move.clone(), self.evaluate_move(board, piece_move)))
            .collect();

        moves.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // println!("{:?}", moves);

        if board.turn == PieceColor::White {
            return moves[moves.len() - 1].clone().0;
        }

        moves[0].clone().0
    }

    pub fn next_best_move_minimax(&self, board: &Board, depth: i32) -> Move {
        if board.turn == PieceColor::White {
            return self.min(board, depth).1;
        }

        self.max(board, depth).1
    }

    fn min(&self, board: &Board, depth: i32) -> (f32, Move) {
    

        let (value, piece_move) = board
            .get_all_moves_for_turn()
            .into_iter()
            .map(|piece_move| {
                let mut board = board.clone();
                board.apply_move(&piece_move);

                if depth == 0 {
                    (self.evaluator.infer_probability_of_white_winning(&board), piece_move)
                } else {
                    (self.max(&board, depth - 1).0, piece_move)
                }
                
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap();

        (value, piece_move)
    }

    fn max(&self, board: &Board, depth: i32) -> (f32, Move) {

        let (value, piece_move) = board
            .get_all_moves_for_turn()
            .into_iter()
            .map(|piece_move| {
                let mut board = board.clone();
                board.apply_move(&piece_move);

                if depth == 0 {
                    (self.evaluator.infer_probability_of_white_winning(&board), piece_move)
                } else {
                    (self.max(&board, depth - 1).0, piece_move)
                }
                
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap();

        (value, piece_move)
    }

    fn evaluate_move(&self, board: &Board, piece_move: &Move) -> f32 {
        let mut board = board.clone();
        board.apply_move(piece_move);
        self.evaluator.infer_probability_of_white_winning(&board)
    }

    pub fn new() -> Self {
        Self {
            evaluator: BoardEvaluator::new(),
        }
    }
}
