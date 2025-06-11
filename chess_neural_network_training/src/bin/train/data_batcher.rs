
use burn::{
    data::dataloader::batcher::Batcher,
    prelude::*,
};
use crate::transposition_dataset::TranspositionItem;

#[derive(Clone, Default)]
pub struct TranspositionBatcher {}

#[derive(Clone, Debug)]
pub struct TranspositionBatch<B: Backend> {
    pub transposition_tensor: Tensor<B, 4>,
    pub targets: Tensor<B, 2, Int>,
}

impl<B: Backend> Batcher<B, TranspositionItem, TranspositionBatch<B>> for TranspositionBatcher {
    fn batch(&self, items: Vec<TranspositionItem>, device: &B::Device) -> TranspositionBatch<B> {
        let images = items
            .iter()
            .map(|item| TensorData::from(item.transposition).convert::<B::BoolElem>())
            .map(|data| Tensor::<B, 3>::from_data(data, device))
            .map(|tensor| tensor.reshape([1, 8, 8, 10]))
            .collect();

        let targets = items
            .iter()
            .map(|item| {
                Tensor::<B, 1, Int>::from_data([match item.label {true => 1, false => 0}.elem::<B::IntElem>()], device) // double check that this outputs the right tensor, [target] so the batch becomes [[target1], [target2]]
            })
            .map(|tensor| tensor.reshape([1, 1]))
            .collect();

        let images = Tensor::cat(images, 0);
        let targets = Tensor::cat(targets, 0);

        TranspositionBatch { transposition_tensor: images, targets }
    }
}