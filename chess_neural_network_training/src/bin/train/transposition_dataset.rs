use burn::data::dataset::transform::PartialDataset;
use burn::data::dataset::{
    Dataset, SqliteDatasetError
};
use chess_engine::move_serialization::{MovesAndLabelRaw, CONFIG};
use chess_engine::{Board, Move, PieceColor, BOARD_TILE_DIM};
use r2d2::Pool;
use r2d2_sqlite::{
    rusqlite::OpenFlags,
    SqliteConnectionManager,
};
use serde::{Deserialize, Serialize};
use serde_rusqlite::from_rows;
use std::path::PathBuf;


/// MNIST item.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TranspositionItem {
    /// Image as a 2D array of floats.
    pub transposition: [[[bool; 10]; BOARD_TILE_DIM as usize]; BOARD_TILE_DIM as usize],

    /// Label of the image.
    pub label: bool,
}

// type MappedDataset =
//     MapperDataset<SqliteDataset<MovesAndLabelRaw>, BytesToTransposition, MovesAndLabelRaw>;

pub struct BoardDataset {
    conn_pool: Pool<SqliteConnectionManager>,
    blank_board: Board,
    len: usize,
}

impl Dataset<TranspositionItem> for BoardDataset {
    fn get(&self, index: usize) -> Option<TranspositionItem> {
        let connection = self.conn_pool.get().unwrap();
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

        let mut new_board = self.blank_board.clone();

        for piece_move in &moves[0..moves.len() - (raw_moves_and_label.id as usize - index)] {
            new_board.apply_move(piece_move);
            // println!("Applied move {:?}", *piece_move);
        }

        Some(TranspositionItem {
            transposition: new_board.generate_transposition(),
            label: raw_moves_and_label.white_winner,
        })
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl BoardDataset {
    fn new(db_file: &str) -> Self {
        let sqlite_flags = OpenFlags::SQLITE_OPEN_READ_ONLY;

        let manager =
            SqliteConnectionManager::file(PathBuf::from(db_file)).with_flags(sqlite_flags);

        let conn_pool = Pool::new(manager)
            .map_err(SqliteDatasetError::ConnectionPool)
            .unwrap();

        let connection = conn_pool.get().unwrap();
        let mut statement = connection
            .prepare(
                "SELECT id
                FROM games
                ORDER BY id DESC
                LIMIT 1;",
            )
            .unwrap();

        let len: usize = statement
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .get(0)
            .unwrap();

        Self {
            conn_pool,
            blank_board: Board::new(PieceColor::White),
            len,
        }
    }
}

type PartialData = PartialDataset<BoardDataset, TranspositionItem>;

pub struct SplitBoardDataset {
    dataset: PartialData,
}

impl Dataset<TranspositionItem> for SplitBoardDataset {
    fn get(&self, index: usize) -> Option<TranspositionItem> {
        self.dataset.get(index)
    }

    fn len(&self) -> usize {
        self.dataset.len()
    }
}

impl SplitBoardDataset {
    pub fn new(db_file: &str, split: &str) -> Self {
        let dataset = BoardDataset::new(db_file);

        let len = dataset.len();

        let data_split = match split {
            "train" => PartialData::new(dataset, 0, len * 8 / 10), // Get first 80% dataset
            "test" => PartialData::new(dataset, len * 8 / 10, len), // Take remaining 20%
            _ => panic!("Invalid split type"),                     // Handle unexpected split types
        };

        Self {
            dataset: data_split,
        }
    }
}
