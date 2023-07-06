use std::path::Path;

use burn::{config::Config, optim::{AdamConfig, decay::WeightDecayConfig}, data::dataloader::DataLoaderBuilder, train::{LearnerBuilder, metric::{LossMetric, AccuracyMetric}}, record::{CompactRecorder, NoStdTrainingRecorder, Recorder}, tensor::backend::{ADBackend, Backend}, module::Module};
use log::info;

use crate::core::{dataset::{AslanDataBatcher, AslanDataset}, model::Model};

// use a base directory for all artifacts
static ARTIFACT_DIR: &str = "training/artifacts";

#[derive(Config)]
pub struct AslanDatasetConfig {
    #[config(default = 3)]
    pub num_epochs: usize,

    #[config(default = 64)]
    pub batch_size: usize,

    #[config(default = 4)]
    pub num_workers: usize,

    #[config(default = 42)]
    pub seed: u64,

    pub optimizer: AdamConfig,
}


pub async fn run<B: ADBackend>(device: B::Device) {
    let base_path: String = std::env::var("VOLUME_PATH").unwrap_or("./".to_string());
    let model_artifact_dir = format!("{}/{}", base_path,ARTIFACT_DIR); 

    // Config
    let config_optimizer = AdamConfig::new();
    let config = AslanDatasetConfig::new(config_optimizer);

    info!("Loading Dataset");
    let batcher_train = AslanDataBatcher::<B>::new(device.clone());
    let batcher_train = batcher_train.initialize_map().await;

    let batcher_valid = AslanDataBatcher::<B::InnerBackend>::new(device.clone());
    let batcher_valid = batcher_valid.initialize_map().await;

    let dataloader_train = DataLoaderBuilder::new(batcher_train)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(AslanDataset::train().await);

    let dataloader_valid = DataLoaderBuilder::new(batcher_valid)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(AslanDataset::validate().await);
    
    info!("Starting Training");

    // // Model
    let model: Model<B> = Model::new();

       
    let learner = LearnerBuilder::new(model_artifact_dir.as_str())
        .metric_train_plot(AccuracyMetric::new())
        .metric_valid_plot(AccuracyMetric::new())
        .metric_train_plot(LossMetric::new())
        .metric_valid_plot(LossMetric::new())
        .with_file_checkpointer(1, CompactRecorder::new())
        .devices(vec![device])
        .num_epochs(config.num_epochs)
        .build(model, config.optimizer.init(), 1e-4);

        let model_trained = learner.fit(dataloader_train, dataloader_valid);

    config
        .save(format!("{model_artifact_dir}/config.json").as_str())
        .unwrap();

    NoStdTrainingRecorder::new()
        .record(
            model_trained.into_record(),
            format!("{model_artifact_dir}/model").into(),
        )
        .expect("Failed to save trained model");

    info!("Done Training");
}