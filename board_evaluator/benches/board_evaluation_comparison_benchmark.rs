use chess_engine::{Board, PieceColor};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pgn_reader::{BufferedReader, SanPlus, Skip, Visitor};
use board_evaluator::ChessEngine;

struct MoveCounter {
    board: Board,
}

impl MoveCounter {
    fn new() -> MoveCounter {
        MoveCounter {
            board : Board::new(PieceColor::White)
        }
    }
}

impl Visitor for MoveCounter {
    type Result = Board;

    fn begin_game(&mut self) {
        self.board = Board::new(PieceColor::White);
    }

    fn san(&mut self, san_plus: SanPlus) {
        let piece_move = self.board.move_from_san(san_plus.san);
        self.board.apply_move(&piece_move);
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        self.board.clone()
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    // https://lichess.org/4si2z6iq

    let pgn = br#"
    1. e4 Nc6 2. d4 e5 3. d5 Nce7 4. Nf3 Ng6 5. Bc4 Nf6 6. Nc3 Bc5 7. O-O h6 8. Be3 d6 9. Bxc5 dxc5 10. a3 a6 11. b4 b5 12. Bb3 c4 13. Ba2 Qd6 14. a4 O-O 15. axb5 Qxb4 16. Qd2 axb5 17. Rfb1 Qc5 18. Rxb5 Qd6 19. Rbb1 Rd8 20. Bxc4
    "#;
    
    let mut reader = BufferedReader::new_cursor(&pgn[..]);

    let mut counter = MoveCounter::new();
    let board = reader.read_game(&mut counter).unwrap().unwrap();
    
    let mut engine = ChessEngine::new(0);

    let mut group = c.benchmark_group("Move Generation Comparison");
    
    for i in [2, 3, 4, 5].iter() {
        group.throughput(Throughput::Elements(*i as u64));
        group.bench_with_input(BenchmarkId::new("NoEvalSortId", i), i, |b, &i| {
            b.iter(|| {engine.next_best_move_natural_minimax_ab_no_eval_sort_id(&board, i)});
        });
        group.bench_with_input(BenchmarkId::new("NoEval", i), i, |b, &i| {
            b.iter(|| {engine.next_best_move_natural_minimax_ab_no_eval(&board, i)});
        });
    }
    group.finish()
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
