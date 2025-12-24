use std::path::PathBuf;
use burn::data::dataset::SqliteDatasetError;
use r2d2::Pool;
use r2d2_sqlite::rusqlite::OpenFlags;
use r2d2_sqlite::SqliteConnectionManager;
use chess_neural_network_training::*;
use chess_engine::board::Board;
use chess_engine::piece::PieceColor;

fn main() {
    let blank_board = Board::new(PieceColor::White);
    let sqlite_flags = OpenFlags::SQLITE_OPEN_READ_ONLY;

    let manager =
        SqliteConnectionManager::file(PathBuf::from("training_data/moves_database.db3")).with_flags(sqlite_flags);

    let conn_pool = Pool::new(manager)
        .map_err(SqliteDatasetError::ConnectionPool)
        .unwrap();
    
    println!("{}", get_sqlite_sorting(24, &blank_board, &conn_pool).0);

    let vector = make_vec(&conn_pool);

    println!("{}", get_vec(24, &blank_board, &vector).0);

}