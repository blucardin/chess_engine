use std::path::PathBuf;
use burn::backend::{Autodiff, Wgpu};
use burn::backend::wgpu::WgpuDevice;
use burn::data::dataloader::batcher::Batcher;
use burn::data::dataset::{Dataset, SqliteDatasetError};
use burn::prelude::{Backend, Config, Module};
use burn::record::{CompactRecorder, FullPrecisionSettings, PrettyJsonFileRecorder, Recorder};
use burn::tensor::activation::softmax;
use burn::tensor::{PrintOptions, set_print_options};
use r2d2::Pool;
use r2d2_sqlite::rusqlite::OpenFlags;
use r2d2_sqlite::SqliteConnectionManager;
use chess_engine::{Board, PieceColor};
use chess_neural_network_training::data_batcher::TranspositionBatcher;
use chess_neural_network_training::{get_sqlite_sorting, get_vec, make_vec};
use chess_neural_network_training::model::{Model, ModelRecord};
use chess_neural_network_training::training::TrainingConfig;
use chess_neural_network_training::transposition_dataset::{SplitBoardDataset, TranspositionItem};

pub fn infer<B: Backend>(model: &Model<B>, device: &B::Device, item: TranspositionItem) {

    let label = item.label;
    let batcher = TranspositionBatcher::default();
    let batch = batcher.batch(vec![item], device);
    let output = model.forward(batch.transposition_tensor);

    let softed = softmax(output.clone(), 1);
    println!("Output: {}", softed);

    let predicted = output.argmax(1).flatten::<1>(0, 1).into_scalar();

    println!("Predicted {} Expected {}", predicted, label);
}


fn main() {

    type MyBackend = Wgpu<f32, i32>;

    let device = burn::backend::wgpu::WgpuDevice::default();
    // println!("CUDA Device: {}", device.index);
    let artifact_dir = "positional_model";

    let config = TrainingConfig::load(format!("{artifact_dir}/config.json"))
        .expect("Config should exist for the model; run train first");
    let record: ModelRecord<MyBackend>  = PrettyJsonFileRecorder::<FullPrecisionSettings>::new()
        .load(format!("{artifact_dir}/model.json").into(), &device)
        .expect("Trained model should exist; run train first");

    let model = config.model.init::<>(&device).load_record(record);

    // let print_options = PrintOptions {
    //     threshold: 10000,
    //     edge_items: 0,
    //     precision: Some(5),
    //     
    // };
    // 
    // set_print_options(print_options);

    println!("{}", model.conv1.weight.val());

    let blank_board = Board::new(PieceColor::White);
    let sqlite_flags = OpenFlags::SQLITE_OPEN_READ_ONLY;

    let manager =
        SqliteConnectionManager::file(PathBuf::from("training_data/moves_database.db3")).with_flags(sqlite_flags);

    let conn_pool = Pool::new(manager)
        .map_err(SqliteDatasetError::ConnectionPool)
        .unwrap();

    let vector = make_vec(&conn_pool);

    // println!("{}", get_vec(24, &blank_board, &vector).0);

    for index in 0..26 {
        let (transposition_board, white_winner) = get_vec(index, &blank_board, &vector);
        println!("{}",transposition_board);
        infer::<MyBackend>(
            &model,
            &device,
            TranspositionItem {
                transposition: transposition_board.generate_small_transposition(),
                label: white_winner,
            }
        );
    }
}