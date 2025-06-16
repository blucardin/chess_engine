
use burn::backend::{Wgpu};
use burn::backend::wgpu::WgpuDevice;
use burn::data::dataloader::batcher::Batcher;
use burn::data::dataset::Dataset;
use burn::prelude::{Backend, Config, Module};
use burn::record::{CompactRecorder, Recorder};
use burn::tensor::activation::softmax;
use burn::tensor::TensorData;
use chess_engine::{Board, Transposition};
use chess_neural_network_training::data_batcher::TranspositionBatcher;
use chess_neural_network_training::model::{Model, ModelRecord};
use chess_neural_network_training::training::TrainingConfig;
use chess_neural_network_training::transposition_dataset::TranspositionItem;

type MyBackend = Wgpu<f32, i32>;

pub struct BoardEvaluator {
    device: WgpuDevice,
    model: Model<MyBackend>,
    batcher: TranspositionBatcher,
}

impl BoardEvaluator {
    pub fn new() -> Self {

        let device = WgpuDevice::default();
        // println!("CUDA Device: {}", device.index);
        let artifact_dir = "/Users/sam/RustroverProjects/chess/model";

        let config = TrainingConfig::load(format!("{artifact_dir}/config.json"))
            .expect("Config should exist for the model; run train first");
        let record: ModelRecord<MyBackend> = CompactRecorder::new()
            .load(format!("{artifact_dir}/model").into(), &device)
            .expect("Trained model should exist; run train first");

        let model = config.model.init::<>(&device).load_record(record);

        let batcher = TranspositionBatcher::default();


        BoardEvaluator {
            device,
            model,
            batcher
        }
    }

    pub fn infer_probability_of_white_winning(&self, transposition : &Transposition) -> f32 {
        let item = TranspositionItem {
            transposition: *transposition, 
            label: false,
        };
        let batch = self.batcher.batch(vec![item], &self.device);
        let output = self.model.forward(batch.transposition_tensor);

        let softened = softmax(output.clone(), 1);
        // println!("Output: {}", softened);

        softened.flatten::<1>(0, 1).into_data().to_vec().unwrap()[1] // replace to vec with as slice
        
        // let predicted = output.argmax(1).flatten::<1>(0, 1).into_scalar();
    }
}
