use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde_rusqlite::from_rows;
use chess_engine::{Board, Move};
use chess_engine::move_serialization::{MovesAndLabelRaw, CONFIG};


// default sqlite sorting method
#[inline]
pub fn get_sqlite_sorting(index: usize, blank_board: &Board, conn_pool : &Pool<SqliteConnectionManager>) -> (Board, bool) {
    let connection = conn_pool.get().unwrap();
    let mut statement = connection
        .prepare_cached(
            "SELECT *
                FROM games
                WHERE id > ?
                ORDER BY id ASC
                LIMIT 1;
                ",
        )
        .unwrap();

    let mut res = from_rows::<MovesAndLabelRaw>(statement.query([index]).unwrap());

    let raw_moves_and_label = res.next().unwrap().unwrap();

    let (moves, _len): (Vec<Move>, usize) =
        bincode::decode_from_slice(&raw_moves_and_label.move_list[..], CONFIG).unwrap();

    let mut new_board = blank_board.clone();

    for piece_move in &moves[0..(moves.len() - (raw_moves_and_label.id as usize - index)) + 1] {
        new_board.apply_move(piece_move);
        // println!("Applied move {:?}", *piece_move);
    }
    
    // label is raw_moves_and_label.white_winner

    (new_board, raw_moves_and_label.white_winner)
}

#[inline]
pub fn get_sqlite_index(index: usize, blank_board: &Board, conn_pool : &Pool<SqliteConnectionManager>) -> (Board, Vec<Move>) {
    let connection = conn_pool.get().unwrap();
    let mut statement = connection
        .prepare_cached(
            "SELECT *
                FROM games
                WHERE id = ?
                LIMIT 1;
                ",
        )
        .unwrap();

    let mut res = from_rows::<MovesAndLabelRaw>(statement.query([index]).unwrap());

    let raw_moves_and_label = res.next().unwrap().unwrap();

    let (moves, _len): (Vec<Move>, usize) =
        bincode::decode_from_slice(&raw_moves_and_label.move_list[..], CONFIG).unwrap();

    let mut new_board = blank_board.clone();
    
    // for piece_move in &moves[0..(moves.len() - (raw_moves_and_label.id as usize - index)) + 1] {
    //     new_board.apply_move(piece_move);
    //     // println!("Applied move {:?}", *piece_move);
    // }
    
    // label is raw_moves_and_label.white_winner
    
    (new_board, moves) // return move to prevent it from being optimized out
}

#[inline]
pub fn get_sqlite_index_no_deserialization(index: usize, blank_board: &Board, conn_pool : &Pool<SqliteConnectionManager>) -> (Board, MovesAndLabelRaw) {
    let connection = conn_pool.get().unwrap();
    let mut statement = connection
        .prepare_cached(
            "SELECT *
                FROM games
                WHERE id = ?
                LIMIT 1;
                ",
        )
        .unwrap();

    let mut res = from_rows::<MovesAndLabelRaw>(statement.query([index]).unwrap());

    let raw_moves_and_label = res.next().unwrap().unwrap();

    let mut new_board = blank_board.clone();

    // for piece_move in &moves[0..(moves.len() - (raw_moves_and_label.id as usize - index)) + 1] {
    //     new_board.apply_move(piece_move);
    //     // println!("Applied move {:?}", *piece_move);
    // }

    // label is raw_moves_and_label.white_winner

    (new_board, raw_moves_and_label) // return move to prevent it from being optimized out
}


pub fn make_vec(conn_pool : &Pool<SqliteConnectionManager>) ->  Vec<(Move, MovesDone)> {
    let connection = conn_pool.get().unwrap();

    let mut statement = connection
        .prepare_cached(
            "SELECT *
                FROM games;
                ",
        )
        .unwrap();

    let res = from_rows::<MovesAndLabelRaw>(statement.query([]).unwrap());
    
    let mut output: Vec<(Move, MovesDone)> = vec![];

    for game in res {
        let game = game.unwrap();

        let (moves, _len): (Vec<Move>, usize) =
            bincode::decode_from_slice(&game.move_list[..], CONFIG).unwrap();
        
        let mut moves_iterator = moves.iter();
        
        output.push((moves_iterator.next().unwrap().clone(), match game.white_winner {
            true => {MovesDone::WhiteWinner}
            false => {MovesDone::BlackWinner}
        }));
        
        for piece_move in moves_iterator {
            output.push((piece_move.clone(), MovesDone::MoreMoves));
        }
    }
    
    output
    
}

#[derive(Debug)]
pub enum MovesDone {
    MoreMoves,
    WhiteWinner,
    BlackWinner, 
}

// storing the moves in a vec in memory
#[inline]
pub fn get_vec(index: usize, blank_board: &Board, moves_vector : &Vec<(Move, MovesDone)>) -> (Board, bool) {
    let mut new_board = blank_board.clone();
    
    let mut i = index; 
    
    while let (piece_move, MovesDone::MoreMoves) = moves_vector.get(i).unwrap() {
        println!("{}", new_board);
        println!("{:?}", piece_move);
        new_board.apply_move(piece_move);
        i -= 1; 
    }
    
    let (piece_move, winner) = moves_vector.get(i).unwrap();
    new_board.apply_move(piece_move);
    
    match winner {
        MovesDone::WhiteWinner => {(new_board, true) }
        MovesDone::BlackWinner => {(new_board, false)}
        MovesDone::MoreMoves => {panic!("Not supposed to happen")}
    }
}



// sqlite with indexing instead of sorting