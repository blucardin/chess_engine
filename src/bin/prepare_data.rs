use chess::move_serialization::{MovesAndLabelRaw, CONFIG};
use chess::{Board, Move, PieceColor};
use pgn_reader::{BufferedReader, Outcome, SanPlus, Skip, Visitor};
use serde_rusqlite::*;
use std::fs::File;
use std::io;
use std::io::ErrorKind;


struct Looker {
    moves: i64,
    normal_termination: bool,
    white_winner: Option<bool>,
    skip: bool,
    board: Board,
    games: i32,
    move_list: Vec<Move>,
    connection: rusqlite::Connection,
}


impl Looker {
    fn new() -> Result<Looker> {
        let path = "./training_data/moves_database.db3";
        let connection = rusqlite::Connection::open(path)?;

        // if !connection.table_exists(None, "games")? {
        connection.execute(
            "CREATE TABLE games (id INT, white_winner BOOL, move_list BLOB)",
            [],
        )?;
        // }

        Ok(Looker {
            moves: 0,
            normal_termination: false,
            white_winner: None,
            board: Board::new(PieceColor::White),
            skip: false,
            games: 0,
            move_list: Vec::with_capacity(265), // Average number of moves was 65.6802940515, max was 256
            connection: connection,
        })
    }
}

impl Visitor for Looker {
    type Result = usize;

    fn begin_game(&mut self) {
        self.normal_termination = false;
        self.white_winner = None;
        self.board = Board::new(PieceColor::White);
        self.skip = false;
        self.move_list.clear();
    }

    fn header(&mut self, key: &[u8], value: pgn_reader::RawHeader<'_>) {
        // println!("{:?} {:?}", key, value);

        let key_decoded = std::str::from_utf8(key).unwrap();

        if key_decoded == "Result" {
            let decoded_header = value.decode_utf8().unwrap();

            if decoded_header == "1-0" {
                self.white_winner = Some(true);
            } else if decoded_header == "0-1" {
                self.white_winner = Some(false);
            }
        } else if key_decoded == "Termination" {
            if value.decode_utf8().unwrap() == "Normal" {
                self.normal_termination = true;
            }
        }
    }
    fn end_headers(&mut self) -> Skip {
        if !self.normal_termination || self.white_winner.is_none() {
            self.skip = true;
            return Skip(true);
        }
        self.games += 1;
        Skip(false)
    }

    fn san(&mut self, san_plus: SanPlus) {
        let piece_move = self.board.move_from_san(san_plus);
        self.move_list.push(piece_move);
        // println!("{}", self.board);
        self.moves += 1;
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn outcome(&mut self, _outcome: Option<Outcome>) {
        // println!("{}", self.move_list.join(" "));
        println!("moves parsed: {}", self.moves);
        // todo: logic for saving the move and white_winner to the sql database
        self.connection.execute("INSERT INTO games (id, white_winner, move_list) VALUES (:id, :white_winner, :move_list)", to_params_named(&MovesAndLabelRaw {
            id: self.moves,
            white_winner: self.white_winner.unwrap(),
            move_list: bincode::encode_to_vec(&self.move_list, CONFIG).unwrap()
        }).unwrap().to_slice().as_slice()).unwrap();
    }

    fn end_game(&mut self) -> Self::Result {
        // println!("{}, {}", self.games, self.moves);
        // self.moves
        self.moves as usize
    }
}

fn main() -> io::Result<()> {
    //     let pgn = br#"
    // [Event "Rated Classical game"]
    // [Site "https://lichess.org/j1dkb5dw"]
    // [White "BFG9k"]
    // [Black "mamalak"]
    // [Result "1-0"]
    // [UTCDate "2012.12.31"]
    // [UTCTime "23:01:03"]
    // [WhiteElo "1639"]
    // [BlackElo "1403"]
    // [WhiteRatingDiff "+5"]
    // [BlackRatingDiff "-8"]
    // [ECO "C00"]
    // [Opening "French Defense: Normal Variation"]
    // [TimeControl "600+8"]
    // [Termination "Normal"]
    //
    // 1. e4 e6 2. d4 b6 3. a3 Bb7 4. Nc3 Nh6 5. Bxh6 gxh6 6. Be2 Qg5 7. Bg4 h5 8. Nf3 Qg6 9. Nh4 Qg5 10. Bxh5 Qxh4 11. Qf3 Kd8 12. Qxf7 Nc6 13. Qe8# 1-0"#;

    // let mut reader = BufferedReader::new_cursor(&pgn[..]);

    let file = File::open("training_data/lichess_db_standard_rated_2013-01.pgn")?;
    let mut reader = BufferedReader::new(file);

    let mut counter = Looker::new()
        .map_err(|err| io::Error::new(ErrorKind::ConnectionRefused, "Database error"))?;

    let mut total_moves = 0;
    while let Ok(Some(moves)) = reader.read_game(&mut counter) {
        total_moves = moves;
    }
    // let moves = reader.read_game(&mut counter)?;

    println!("Total Number of Moves is {}", total_moves);

    Ok(())
}
