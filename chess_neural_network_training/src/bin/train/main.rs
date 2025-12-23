use chess_neural_network_training::training::TrainingConfig;
use burn::{
    backend::{Autodiff, Wgpu},
    optim::AdamConfig,
};
use chess_neural_network_training::model::ModelConfig;

fn main() {
    type MyBackend = Wgpu<f32, i32>;
    type MyAutodiffBackend = Autodiff<MyBackend>;

    let device = burn::backend::wgpu::WgpuDevice::default(); 
    // println!("CUDA Device: {}", device.index);
    let artifact_dir = "positional_model";
    chess_neural_network_training::training::train::<MyAutodiffBackend>(
        artifact_dir,
        TrainingConfig::new(ModelConfig::new(), AdamConfig::new()),
        device.clone(),
    );
}