use std::fs::File;
use chess::{Board, PieceColor};
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use std::io;
use bevy::tasks::futures_lite::StreamExt;

struct Looker {
    moves: usize,
    normal_termination: bool,
    white_winner: Option<bool>,
    skip: bool, 
    board: Board,
    games: i32, 
}

impl Looker {
    fn new() -> Looker {
        Looker {
            moves: 0,
            normal_termination: false,
            white_winner: None,
            board: Board::new(PieceColor::White),
            skip: false,
            games: 0, 
        }
    }
}

impl Visitor for Looker {
    type Result = usize;

    fn begin_game(&mut self) {
        self.normal_termination = false;
        self.white_winner = None;
        self.board = Board::new(PieceColor::White);
        self.skip = false;
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
            return Skip(true)
        }
        self.games += 1;
        Skip(false)
    }

    fn san(&mut self, san_plus: SanPlus) {
        self.board.apply_san(san_plus);
        // println!("{}", self.board);
        // todo: logic for saving the move to the sql database
        self.moves += 1; 
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        println!("{}, {}", self.games, self.moves);
        self.moves
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

    let mut counter = Looker::new();
    
    let mut total_moves = 0; 
    while let Ok(Some(moves)) = reader.read_game(&mut counter) {
        total_moves = moves; 
    }
    // let moves = reader.read_game(&mut counter)?;

    println!("Total Number of Moves is {}", total_moves);

    Ok(())
}
