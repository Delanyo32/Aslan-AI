use burn::{module::Module, tensor::{backend::{Backend, ADBackend}, Tensor, activation::softmax}, nn::{self, loss::CrossEntropyLoss}, train::{ClassificationOutput, TrainStep, TrainOutput, ValidStep}};
use log::info;

use super::dataset::DataBatch;



#[derive(Module, Debug)]
pub struct Model<B: Backend> {
    linear1: nn::Linear<B>,
    linear2: nn::Linear<B>,
}

//6685
impl<B: Backend> Model<B> {
    pub fn new() -> Self {
        let linear1 = nn::LinearConfig::new(6685, 100);
        let linear2 = nn::LinearConfig::new(100,6685);
        Self {
            linear1: linear1.with_bias(false).init(),
            linear2: linear2.with_bias(false).init(),
        }
    }

    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.linear1.forward(input);
        //info!("Linear 1: {:?}", x);
        let x = self.linear2.forward(x);
        //info!("Linear 2: {:?}", x);
        x
    }

// Find out how to code forward pass given that the clasification passes in a DataBatch
// log target and output data debug the learning rate and the loss HERE!!
    pub fn forward_classification(&self, item: DataBatch<B>) -> ClassificationOutput<B> {
        let targets = item.outputs;

        let output = self.forward(item.inputs);
        
        let loss = CrossEntropyLoss::new(None);
        let loss = loss.forward(output.clone(), targets.clone());

        ClassificationOutput {
            loss,
            output,
            targets,
        }
    }
}

impl<B: ADBackend> TrainStep<DataBatch<B>, ClassificationOutput<B>> for Model<B> {
    fn step(&self, item: DataBatch<B>) -> TrainOutput<ClassificationOutput<B>> {
        let item = self.forward_classification(item);

        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<DataBatch<B>, ClassificationOutput<B>> for Model<B> {
    fn step(&self, item: DataBatch<B>) -> ClassificationOutput<B> {
        self.forward_classification(item)
    }
}