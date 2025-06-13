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
use chess_neural_network_training::MovesDone;

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
    blank_board: Board,
    len: usize,
    vector_of_moves: Vec<(Move, MovesDone)>,
}

impl Dataset<TranspositionItem> for BoardDataset {
    fn get(&self, index: usize) -> Option<TranspositionItem> {

        let mut new_board = self.blank_board.clone();

        let mut i = index;
        
        let mut vector = vec![];

        while let (piece_move, MovesDone::MoreMoves) = self.vector_of_moves.get(i).unwrap() {
            // println!("{}", new_board);
            // println!("{:?}", piece_move);
            vector.push(piece_move);
            i -= 1;
        }
        
        let (piece_move, winner) = self.vector_of_moves.get(i).unwrap();
        vector.push(piece_move);

        for piece_move in vector.iter().rev() {
            new_board.apply_move(*piece_move);
        }

        let white_winner = match winner {
            MovesDone::WhiteWinner => {true}
            MovesDone::BlackWinner => {false}
            MovesDone::MoreMoves => {panic!("Not supposed to happen")}
        };

        Some(TranspositionItem {
            transposition: new_board.generate_transposition(),
            label: white_winner,
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
        
        Self {
            blank_board: Board::new(PieceColor::White),
            len: output.len(), 
            vector_of_moves: output, 
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
