use chess_engine::{Board, Move, PieceColor};
use crate::board_evaluator::BoardEvaluator;

mod board_evaluator;

pub struct ChessEngine {
    evaluator: BoardEvaluator
}

impl ChessEngine {
    pub fn next_best_move(&self, board: &Board) -> Move {

        let possible_moves = board.get_all_moves_for_turn();

        let mut moves: Vec<_> = possible_moves.iter()
            .map(|piece_move| (piece_move.clone(), self.evaluate_move(board, piece_move)))
            .collect();

        moves.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        println!("{:?}", moves);
        
        if board.turn == PieceColor::White {
            return moves[moves.len() - 1].clone().0
        }
        
        moves[0].clone().0
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

