use std::path::PathBuf;
use burn::data::dataset::SqliteDatasetError;
use r2d2::Pool;
use r2d2_sqlite::rusqlite::OpenFlags;
use r2d2_sqlite::SqliteConnectionManager;
use chess_neural_network_training::*;
use chess_engine::*;
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};


fn bench_data_access_strategies(c: &mut Criterion) {

    let blank_board = Board::new(PieceColor::White);
    let sqlite_flags = OpenFlags::SQLITE_OPEN_READ_ONLY;

    let manager =
        SqliteConnectionManager::file(PathBuf::from("training_data/moves_database.db3")).with_flags(sqlite_flags);

    let conn_pool = Pool::new(manager)
        .map_err(SqliteDatasetError::ConnectionPool)
        .unwrap();

    let vector = make_vec(&conn_pool);

    // println!("{}", get_sqlite_sorting(24, &blank_board, &conn_pool).0);
    // 
    // 
    // println!("{}", get_vec(24, &blank_board, &vector).0);


    let mut group = c.benchmark_group("Data_Access_Strategies");
    for i in [70789, 72043, 6853] {
        group.throughput(Throughput::Bytes(i as u64));
        group.bench_with_input(BenchmarkId::new("SqliteAccess", i), &i,
                               |b, i| b.iter(|| get_sqlite_sorting(*i as usize, &blank_board, &conn_pool)));
        group.bench_with_input(BenchmarkId::new("SqliteAccessByIndex", i), &i,
                               |b, i| b.iter(|| get_sqlite_index(*i as usize, &blank_board, &conn_pool)));
        group.bench_with_input(BenchmarkId::new("SqliteAccessByIndexNoDeSer", i), &i,
                               |b, i| b.iter(|| get_sqlite_index_no_deserialization(*i as usize, &blank_board, &conn_pool)));
        group.bench_with_input(BenchmarkId::new("VectorAccess", i), &i,
                               |b, i| b.iter(|| get_vec(*i as usize, &blank_board, &vector)));
    }
    group.finish();
}

criterion_group!(benches, bench_data_access_strategies);
criterion_main!(benches);



// fn main() {
//     let blank_board = Board::new(PieceColor::White);
//     let sqlite_flags = OpenFlags::SQLITE_OPEN_READ_ONLY;
// 
//     let manager =
//         SqliteConnectionManager::file(PathBuf::from("training_data/moves_database.db3")).with_flags(sqlite_flags);
// 
//     let conn_pool = Pool::new(manager)
//         .map_err(SqliteDatasetError::ConnectionPool)
//         .unwrap();
//     
//     println!("{}", get_sqlite_sorting(24, &blank_board, &conn_pool).0);
// 
//     let vector = make_vec(conn_pool);
// 
//     println!("{}", get_vec(24, blank_board, vector).0);
// 
// }