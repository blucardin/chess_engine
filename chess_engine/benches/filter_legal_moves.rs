use std::time::Duration;
use chess_engine::board::Board;
use criterion::{criterion_group, criterion_main, Bencher, BenchmarkId, Criterion, Throughput};
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use chess_engine::Move;
use chess_engine::piece::PieceColor;

struct MoveCounter {
    board: Board,
    piece_moves: Vec<Vec<Move>>
}

impl MoveCounter {
    fn new() -> MoveCounter {
        MoveCounter {
            board : Board::new(PieceColor::White),
            piece_moves: Vec::new(),
        }
    }
}

impl Visitor for MoveCounter {
    type Result = Vec<Vec<Move>>;

    fn begin_game(&mut self) {

        self.board = Board::new(PieceColor::White);

        self.piece_moves.push(Vec::new());
    }

    fn san(&mut self, san_plus: SanPlus) {
        let piece_move = self.board.move_from_san(san_plus.san);
        self.board.apply_move(&piece_move);

        let length = self.piece_moves.len();
        self.piece_moves[length - 1].push(piece_move);
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        self.piece_moves.clone()
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    // https://lichess.org/4si2z6iq

    let pgn = br#"
    1. e4 Nc6 2. d4 e5 3. d5 Nce7 4. Nf3 Ng6 {5. Bc4 Nf6 6. Nc3 Bc5 7. O-O h6 8. Be3 d6 9. Bxc5 dxc5 10. a3 a6 11. b4 b5 12. Bb3 c4 13. Ba2 Qd6 14. a4 O-O 15. axb5 Qxb4 16. Qd2 axb5 17. Rfb1 Qc5 18. Rxb5 Qd6 19. Rbb1 Rd8 20. Bxc4}
    "#;

    let mut reader = BufferedReader::new_cursor(&pgn[..]);

    let mut counter = MoveCounter::new();
    let piece_moves = reader.read_game(&mut counter).unwrap().unwrap();

    let mut group = c.benchmark_group("Move Filtering Comparison");
    group.warm_up_time(Duration::from_secs_f32(0.5));
    group.measurement_time(Duration::from_secs_f32(0.5));

    let mut seen_before: Vec<f32> = vec![];

    for game in piece_moves {
        let mut board = Board::new(PieceColor::White);

        for piece_move in game {

            let input: Vec<Move> = board.get_all_moves_for_turn_unchecked_iter().collect();

            println!("Length of input vector: {}", input.len());

            let mut throughput : f32 = input.len() as f32;
            while seen_before.contains(&throughput) {
                throughput += 0.001;
            };
            seen_before.push(throughput);

            // assert!(board.filter_legal_moves_clone(input.clone().into_iter()).eq(board.filter_legal_moves(input.clone().into_iter())));
            group.throughput(Throughput::Elements(input.len() as u64));

            group.bench_with_input(BenchmarkId::new("With Cloning", throughput), &input.clone(), |b: &mut Bencher, i: &Vec<Move>| {
                b.iter(|| board.filter_legal_moves(i.clone().into_iter()))
            });

            group.bench_with_input(BenchmarkId::new("Without Cloning", throughput), &input.clone(), |b: &mut Bencher, i: &Vec<Move>| {
                b.iter(|| board.filter_legal_moves_iter(i.clone().into_iter()))
            });

            board.apply_move(&piece_move);
        }
    }


    group.finish()
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
