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
            return self.min(board, depth).1; // NOTE TO SELF: I somehow wrote the minimax backwards for the functional solution, so the min is actually the max, and if it is white's turn, you call the max
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
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap(); // Here is where the issue is, this is max where it is supposed to be min

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
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap(); // Here is where the issue is, this is min where it is supposed to be max

        (value, piece_move)
    }

    pub fn next_best_move_minimax_ab(&self, board: &Board, depth: i32) -> Move {
        println!("Minimax, searching");
        if board.turn == PieceColor::White {
            return self.max_ab(board, depth).1;
        }

        self.min_ab(board, depth).1
    }

    fn min_ab(&self, board: &Board, depth: i32) -> (f32, Move) {


        let (mut min_value, mut min_piece_move) = (f32::INFINITY, None);

        // println!("min");
        for piece_move in board.get_all_moves_for_turn() {
            // println!("min_move");


            let mut board = board.clone();
            board.apply_move(&piece_move);

            let eval; 
            if depth == 0 {
                println!("eval_min");
                eval = self.evaluator.infer_probability_of_white_winning(&board);
            } else {
                eval = self.max_ab(&board, depth - 1).0;
            }

            if eval < min_value {
                min_value = eval;
                min_piece_move = Some(piece_move);
            }
                
        }

        (min_value, min_piece_move.unwrap())
    }

    fn max_ab(&self, board: &Board, depth: i32) -> (f32, Move) {

        let (mut max_value, mut max_piece_move) = (-f32::INFINITY, None);

        // println!("max");
        for piece_move in board.get_all_moves_for_turn() {
            // println!("max_move");

            let mut board = board.clone();
            board.apply_move(&piece_move);

            let eval;
            if depth == 0 {
                println!("eval_max");
                eval = self.evaluator.infer_probability_of_white_winning(&board);
            } else {
                eval = self.min_ab(&board, depth - 1).0;
            }

            if eval > max_value {
                max_value = eval;
                max_piece_move = Some(piece_move);
            }

        }

        (max_value, max_piece_move.unwrap())
    }

    fn evaluate_move(&self, board: &Board, piece_move: &Move) -> f32 {
        let mut board = board.clone();
        board.apply_move(piece_move);
        println!("eval_move");
        self.evaluator.infer_probability_of_white_winning(&board)
    }

    pub fn new() -> Self {
        Self {
            evaluator: BoardEvaluator::new(),
        }
    }
}
