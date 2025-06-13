use crate::data_batcher::TranspositionBatch;
use burn::nn::loss::{BinaryCrossEntropyLossConfig, CrossEntropyLossConfig};
use burn::nn::Sigmoid;
use burn::tensor::backend::AutodiffBackend;
use burn::train::{ClassificationOutput, MultiLabelClassificationOutput, TrainOutput, TrainStep, ValidStep};
use burn::{
    nn::{
        Dropout, DropoutConfig, Linear, LinearConfig, Relu
    },
    prelude::*,
};

#[derive(Module, Debug)]
pub struct Model<B: Backend> {
    dropout: Dropout,
    linear1: Linear<B>,
    // linear2: Linear<B>,
    // linear3: Linear<B>,
    linear4: Linear<B>,
    linear5: Linear<B>,
    activation: Relu,
    // sigmoid: Sigmoid,
}

#[derive(Config, Debug)]
pub struct ModelConfig {
    // hidden_size: usize,
    #[config(default = "0.5")] // todo: I feel like this is too high
    dropout: f64,
}

impl ModelConfig {
    /// Returns the initialized model.
    pub fn init<B: Backend>(&self, device: &B::Device) -> Model<B> {
        Model {
            activation: Relu::new(),
            linear1: LinearConfig::new(8 * 8 * 10, 512).init(device),
            // linear2: LinearConfig::new(1024, 2048).init(device),
            // linear3: LinearConfig::new(2048, 1024).init(device),
            linear4: LinearConfig::new(512, 128).init(device),
            linear5: LinearConfig::new(128, 2).init(device),
            // sigmoid: Sigmoid::new(),    // todo: replace with hard sigmoid
            dropout: DropoutConfig::new(self.dropout).init(),
        }
    }
}

impl<B: Backend> Model<B> {
    /// # Shapes
    ///   - Images [batch_size, height, width]
    ///   - Output [batch_size, num_classes]
    pub fn forward(&self, images: Tensor<B, 4>) -> Tensor<B, 2> {
        let [batch_size, height, width, fields] = images.dims();
        // 64, 8, 8, 10

        // Create a channel at the second dimension.
        let x = images.reshape([batch_size, height * width * fields]);
        // 64, 640
        
        let x = self.linear1.forward(x);
        // let x = self.dropout.forward(x);
        let x = self.activation.forward(x);

        // let x = self.linear2.forward(x);
        // let x = self.dropout.forward(x);
        // let x = self.activation.forward(x);
        // 
        // let x = self.linear3.forward(x);
        // let x = self.dropout.forward(x);
        // let x = self.activation.forward(x);

        let x = self.linear4.forward(x);
        // let x = self.dropout.forward(x);
        let x = self.activation.forward(x);

        let x = self.linear5.forward(x); // [batch_size, num_classes]

        // self.sigmoid.forward(x)
        
        // x.squeeze_dims(&[1isize])
        // // 64

        x
    }
}

impl<B: Backend> Model<B> {
    pub fn forward_classification(
        &self,
        transpositions: Tensor<B, 4>,
        targets: Tensor<B, 1, Int>,
    ) -> ClassificationOutput<B> {
        
        let output = self.forward(transpositions);
        let loss = CrossEntropyLossConfig::new()
            .init(&output.device())
            .forward(output.clone(), targets.clone());

        // let targets_with_extra_dim = targets.unsqueeze_dim(1);

        ClassificationOutput::new(loss, output, targets)
    }
}

impl<B: AutodiffBackend> TrainStep<TranspositionBatch<B>, ClassificationOutput<B>> for Model<B> {
    fn step(&self, batch: TranspositionBatch<B>) -> TrainOutput<ClassificationOutput<B>> {
        let item = self.forward_classification(batch.transposition_tensor, batch.targets);

        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<TranspositionBatch<B>, ClassificationOutput<B>> for Model<B> {
    fn step(&self, batch: TranspositionBatch<B>) -> ClassificationOutput<B> {
        self.forward_classification(batch.transposition_tensor, batch.targets)
    }
}