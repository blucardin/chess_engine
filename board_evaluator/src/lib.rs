use std::iter::Rev;
use std::slice::Iter;
use crate::board_evaluator::BoardEvaluator;
use chess_engine::{Board, Coordinate, Move, PieceColor, Transposition};
use quick_cache::unsync::Cache;

mod board_evaluator;

pub struct ChessEngine {
    evaluator: BoardEvaluator,
    cache: Cache<Transposition, f32>,
    leaf_nodes_visited: i32, 
    cache_hits: i32, 
    cache_misses: i32,
}

impl ChessEngine {
    pub fn next_best_move_shallow(&mut self, board: &Board) -> Move {
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

    pub fn next_best_move_minimax(&mut self, board: &Board, depth: i32) -> Move {
        println!("Minimax, searching");

        self.leaf_nodes_visited = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;

        let output = match board.turn {
            PieceColor::White => {self.max(board, depth).1}
            PieceColor::Black => {self.min(board, depth).1}
        };

        println!("Leaf_nodes_visited: {}", self.leaf_nodes_visited);
        println!("Cache hits: {}", self.cache_hits);
        println!("Cache misses: {}", self.cache_misses);

        output
    }

    fn min(&mut self, board: &Board, depth: i32) -> (f32, Move) {
    

        let (value, piece_move) = board
            .get_all_moves_for_turn()
            .into_iter()
            .map(|piece_move| {
                let mut board = board.clone();
                board.apply_move(&piece_move);

                if depth == 0 {
                    (self.infer_probability_of_white_winning_cached(&board), piece_move)
                } else {
                    (self.max(&board, depth - 1).0, piece_move)
                }
                
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap(); 

        (value, piece_move)
    }

    fn max(&mut self, board: &Board, depth: i32) -> (f32, Move) {

        let (value, piece_move) = board
            .get_all_moves_for_turn()
            .into_iter()
            .map(|piece_move| {
                let mut board = board.clone();
                board.apply_move(&piece_move);

                if depth == 0 {
                    (self.infer_probability_of_white_winning_cached(&board), piece_move)
                } else {
                    (self.max(&board, depth - 1).0, piece_move)
                }
                
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).unwrap(); 

        (value, piece_move)
    }

    pub fn next_best_move_minimax_ab(&mut self, board: &Board, depth: i32) -> Move {
        println!("Minimax_ab, searching");

        self.leaf_nodes_visited = 0; 
        self.cache_hits = 0;
        self.cache_misses = 0; 
        
        let output = match board.turn {
            PieceColor::White => {self.max_ab(board, depth, -f32::INFINITY, f32::INFINITY).1.unwrap()}
            PieceColor::Black => {self.min_ab(board, depth, -f32::INFINITY, f32::INFINITY).1.unwrap()}
        };
        
        println!("Leaf_nodes_visited: {}", self.leaf_nodes_visited);
        println!("Cache hits: {}", self.cache_hits);
        println!("Cache misses: {}", self.cache_misses);
        
        output
    }

    fn min_ab(&mut self, board: &Board, depth: i32, alpha: f32, beta: f32) -> (f32, Option<Move>) {

        let mut beta = beta; 
        
        let mut min_value = f32::INFINITY;
        let mut min_piece_move = None;

        // println!("min");
        for piece_move in board.get_all_moves_for_turn() {
            // println!("min_move");

            let mut board = board.clone();
            board.apply_move(&piece_move);

            let eval; 
            if depth == 0 {
                // println!("eval_min");
                eval = self.infer_probability_of_white_winning_cached(&board);
            } else {
                eval = self.max_ab(&board, depth - 1, alpha, min_value).0;  // todo:: replace min_value with beta
            }

            if eval < min_value {
                min_value = eval;
                min_piece_move = Some(piece_move);
            }

            if min_value <= alpha { 
                break;
            }
            
            if min_value < beta {
                beta = min_value;
            }
                
        }

        (min_value, min_piece_move)
    }

    fn max_ab(&mut self, board: &Board, depth: i32, alpha: f32, beta: f32) -> (f32, Option<Move>) {
        
        let mut alpha = alpha;

        let mut max_value = -f32::INFINITY; 
        let mut max_piece_move = None;

        // println!("max");
        for piece_move in board.get_all_moves_for_turn() {
            // println!("max_move");

            let mut board = board.clone();
            board.apply_move(&piece_move);

            let eval;
            if depth == 0 {
                // println!("eval_max");
                eval = self.infer_probability_of_white_winning_cached(&board);
            } else {
                eval = self.min_ab(&board, depth - 1, alpha, beta).0;
            }
            
            if eval > max_value { 
                max_value = eval;
                max_piece_move = Some(piece_move);
            }

            if max_value >= beta {
                break;
            }
            
            if max_value > alpha {
                alpha = max_value;
            }
        }

        (max_value, max_piece_move) // to handel checkmates, remove the unwrap and just return the negative infinity value
    }

    fn evaluate_move(&mut self, board: &Board, piece_move: &Move) -> f32 {
        let mut board = board.clone();
        board.apply_move(piece_move);
        // println!("eval_move");
        self.infer_probability_of_white_winning_cached(&board)
    }

    pub fn infer_probability_of_white_winning(&mut self, board: &Board) -> f32 {
        self.leaf_nodes_visited += 1;
    
        let probability_of_bottom_winning = self.evaluator.infer_probability_of_bottom_winning(&board.generate_transposition());
        self.evaluator.convert_probability_of_bottom_winning_to_white_winning(probability_of_bottom_winning, &board.turn) 
    }

    pub fn infer_probability_of_white_winning_cached(&mut self, board: &Board) -> f32 { 

        self.leaf_nodes_visited += 1;

        let transposition = board.generate_transposition();
        
        // *self.cache.get_or_insert_with(&transposition, || { Ok::<f32, MyError>(self.evaluator.infer_probability_of_white_winning(&transposition)) }).unwrap().unwrap()
        match self.cache.get(&transposition) { // todo: replace with get or insert with
            Some(probability_of_bottom_winning) => {
                // println!("cache hit");
                self.cache_hits += 1;
                self.evaluator.convert_probability_of_bottom_winning_to_white_winning(*probability_of_bottom_winning, &board.turn)
            }
            None => {
                self.cache_misses += 1;
                let probability_of_bottom_winning = self.evaluator.infer_probability_of_bottom_winning(&transposition);
                self.cache.insert(transposition, probability_of_bottom_winning);
                self.evaluator.convert_probability_of_bottom_winning_to_white_winning(probability_of_bottom_winning, &board.turn)
            }
        }
    }

    pub fn next_best_move_minimax_ab_id(&mut self, board: &Board, depth: i32) -> Move {
        println!("Minimax_ab, searching");

        self.leaf_nodes_visited = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;
        
        let mut moves_and_values: Vec<_> = board.get_all_moves_for_turn()
            .iter()
            .map(|piece_move| (piece_move.clone(), self.evaluate_move(board, piece_move)))
            .collect();

        moves_and_values.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap()); // sorts from the least white probability to most white probability
        
        // println!("{:?}", moves_and_values);

        let moves: Vec<_> = moves_and_values.into_iter().map(|a| a.0).collect();

        let output = match board.turn {
            PieceColor::White => {self.max_ab_id(board, &moves.into_iter().rev().collect(), depth, -f32::INFINITY, f32::INFINITY).1.unwrap()}
            PieceColor::Black => {self.min_ab_id(board, &moves, depth, -f32::INFINITY, f32::INFINITY).1.unwrap()}
        };

        println!("Leaf_nodes_visited: {}", self.leaf_nodes_visited);
        println!("Cache hits: {}", self.cache_hits);
        println!("Cache misses: {}", self.cache_misses);

        output
    }

    fn min_ab_id(&mut self, board: &Board, moves: &Vec<Move>, depth: i32, alpha: f32, beta: f32) -> (f32, Option<Move>) {

        let mut beta = beta;

        let mut min_value = f32::INFINITY;
        let mut min_piece_move = None;

        // println!("min");
        for piece_move in moves {
            // println!("min_move");

            let mut board = board.clone();
            board.apply_move(&piece_move);

            let eval;
            if depth == 0 {
                // println!("eval_min");
                eval = self.infer_probability_of_white_winning_cached(&board);
            } else {
                eval = self.max_ab_id(&board, &board.get_all_moves_for_turn(), depth- 1, alpha, min_value).0; // todo:: replace min_value with beta
            }

            if eval < min_value {
                min_value = eval;
                min_piece_move = Some(piece_move);
            }

            if min_value <= alpha {
                break;
            }

            if min_value < beta {
                beta = min_value;
            }

        }

        (min_value, min_piece_move.cloned())
    }

    fn max_ab_id(&mut self, board: &Board, moves: &Vec<Move>, depth: i32, alpha: f32, beta: f32) -> (f32, Option<Move>) {
        
        let mut alpha = alpha;

        let mut max_value = -f32::INFINITY;
        let mut max_piece_move = None;

        // println!("max");
        for piece_move in moves {
            // println!("max_move");

            let mut board = board.clone();
            board.apply_move(&piece_move);

            let eval;
            if depth == 0 {
                // println!("eval_max");
                eval = self.infer_probability_of_white_winning_cached(&board);
            } else { 
                eval = self.min_ab_id(&board, &board.get_all_moves_for_turn(), depth - 1, alpha, beta).0;
            }

            if eval > max_value {
                max_value = eval;
                max_piece_move = Some(piece_move);
            }

            if max_value >= beta {
                break;
            }

            if max_value > alpha {
                alpha = max_value;
            }
        }

        (max_value, max_piece_move.cloned()) // to handel checkmates, remove the unwrap and just return the negative infinity value
    }

    pub fn next_best_move_natural_minimax_ab(&mut self, board: &Board, depth: i32) -> Move {
        // println!("Minimax_ab, searching");

        self.leaf_nodes_visited = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;

        let output : Move  = match board.turn {
            PieceColor::Black => {
                
                let mut min_value = f32::INFINITY;
                let mut min_move = None;

                // println!("min");
                for piece_move in board.get_all_moves_for_turn() {
                    // println!("min_move");
                    // for x in 0..depth {
                    //     print!("\t");
                    // }
                    // println!("{:?}", piece_move);

                    let mut board = board.clone();
                    board.apply_move(&piece_move);

                    let eval;
                    if depth == 0 {
                        // println!("eval_min");
                        eval = board.natural_score();
                        self.leaf_nodes_visited += 1;

                    } else {
                        eval = self.max_natural_ab(&board, depth - 1, -f32::INFINITY, min_value);
                    }
                    // for x in 0..depth {
                    //     print!("\t");
                    // }
                    // println!("{:?}", eval);

                    if eval < min_value {
                        min_value = eval;
                        min_move = Some(piece_move);
                    }

                    // alpha/beta cut here is not possible because we are at root node
                }

                min_move.unwrap()
            }
            PieceColor::White => {

                let mut max_value = -f32::INFINITY;
                let mut max_move = None;

                // println!("max");
                for piece_move in board.get_all_moves_for_turn() {
                    // println!("max_move");
                    // 
                    // for x in 0..depth {
                    //     print!("\t");
                    // }
                    // println!("{:?}", piece_move);

                    let mut board = board.clone();
                    board.apply_move(&piece_move);

                    let eval;
                    if depth == 0 {
                        // println!("eval_max");
                        eval = board.natural_score();
                        self.leaf_nodes_visited += 1;
                    } else {
                        eval = self.min_natural_ab(&board, depth - 1, max_value, f32::INFINITY);
                    }

                    // for x in 0..depth {
                    //     print!("\t");
                    // }
                    // println!("{:?}", eval);

                    if eval > max_value {
                        max_value = eval;
                        max_move = Some(piece_move);
                    }
                    
                    // alpha/beta cut here is not possible because we are at root node
                }
                
                max_move.unwrap() // it put me in checkmate, but called computer function again, causing it to error out here. 
            }
        };
        
        // println!("Leaf_nodes_visited: {}", self.leaf_nodes_visited);
        // println!("Cache hits: {}", self.cache_hits);
        // println!("Cache misses: {}", self.cache_misses);
        
        output
        
    }

    fn min_natural_ab(&mut self, board: &Board, depth: i32, alpha: f32, beta: f32) -> f32 {

        let mut beta = beta;

        let mut min_value = f32::INFINITY;

        // println!("min");
        for piece_move in board.get_all_moves_for_turn() {
            // println!("min_move");

            // for x in 0..depth {
            //     print!("\t");
            // }
            // println!("{:?}", piece_move);

            let mut board = board.clone();
            board.apply_move(&piece_move);

            let eval;
            if depth == 0 {
                // println!("eval_min");
                eval = board.natural_score();
                self.leaf_nodes_visited += 1;
                
            } else {
                eval = self.max_natural_ab(&board, depth - 1, alpha, beta);

            }

            // for x in 0..depth {
            //     print!("\t");
            // }
            // println!("{:?}", eval);

            if eval < min_value {
                min_value = eval;
            }

            if min_value <= alpha {
                break;
            }

            if min_value < beta {
                beta = min_value;
            }

        }

        min_value
    }

    fn max_natural_ab(&mut self, board: &Board, depth: i32, alpha: f32, beta: f32) -> f32 {

        let mut alpha = alpha;

        let mut max_value = -f32::INFINITY;

        // println!("max");
        for piece_move in board.get_all_moves_for_turn() {
            // println!("max_move");

            // for x in 0..depth {
            //     print!("\t");
            // }
            // println!("{:?}", piece_move);

            let mut board = board.clone();
            board.apply_move(&piece_move);

            let eval;
            if depth == 0 {
                // println!("eval_max");
                eval = board.natural_score();
                self.leaf_nodes_visited += 1;
            } else {
                eval = self.min_natural_ab(&board, depth - 1, alpha, beta);
            }

            // for x in 0..depth {
            //     print!("\t");
            // }
            // println!("{:?}", eval);

            if eval > max_value {
                max_value = eval;
            }

            if max_value >= beta {
                break;
            }

            if max_value > alpha {
                alpha = max_value;
            }
        }

        max_value
    }

    pub fn new(cache_items_capacity: usize) -> Self {
        Self {
            evaluator: BoardEvaluator::new(),
            cache: Cache::new(cache_items_capacity),
            leaf_nodes_visited: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

// #[derive(Debug)]
// enum MyError {
//     EvaluationFailed,
//     // other variants
// }

