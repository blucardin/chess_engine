use chess_neural_network_training::training::TrainingConfig;
use burn::{
    backend::{Autodiff, Cuda},
    optim::AdamConfig,
};
use chess_neural_network_training::model::ModelConfig;

fn main() {
    type MyBackend = Cuda<f32, i32>;
    type MyAutodiffBackend = Autodiff<MyBackend>;

    let device = burn::backend::cuda::CudaDevice::default(); 
    // println!("CUDA Device: {}", device.index);
    let artifact_dir = "C:\\Users\\bluca\\RustroverProjects\\chess_engine\\conv_model_sm";
    chess_neural_network_training::training::train::<MyAutodiffBackend>(
        artifact_dir,
        TrainingConfig::new(ModelConfig::new(), AdamConfig::new()),
        device.clone(),
    );
}