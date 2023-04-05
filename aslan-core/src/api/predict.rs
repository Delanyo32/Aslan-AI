use actix_web::{
    post,
    web::{self, Json},
};
use aslan_bootstrap::BootstrapResult;
use aslan_data::DataNode;
use serde::{Deserialize, Serialize};
use log::{info};

use crate::db::mongodb::MongoClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct DataResponse {
    message: String,
    data: BootstrapResult,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictParameters{
    pub symbol: String,
    pub seed: f64,
    pub size: usize,
}


#[post("/predict")]
pub async fn generate(path: web::Json<PredictParameters>) -> Json<DataResponse> {
    
    info!("Generating prediction for symbol: {}", path.symbol);
    let mongodb = MongoClient::new().await;
    let open_nodes = mongodb.get_node_data(&path.symbol).await;

    info!("Model Loaded for symbol: {}", path.symbol);
    let prediction = generate_prediction(&open_nodes, path.seed, path.size);

    let response = DataResponse {
        message: "Predicted Data".to_string(),
        data: prediction,
    };
    Json(response)
}

pub fn generate_prediction(nodes: &Vec<DataNode>, partition_seed:f64,partition_size:usize)->BootstrapResult{
    //generate wavereduce results
    info!("Running Wavereduce");
    let waveresultsize = 100;
    let wavereduce = aslan_wavereduce::WaveReduce::new(partition_seed, partition_size,waveresultsize);
    let wavereduce_results = wavereduce.generate_results(nodes);

    info!("Running Boostrap");
    let random_results = wavereduce_results.get_random_results(5);

    //flatten the top results
    let flat_results = aslan_wavereduce::WaveReduceSolution::flatten_results(random_results);

    //generate bootstrap results
    let boostrap_iterations = 100;
    let open_bootstrap = aslan_bootstrap::Bootstrap::new(boostrap_iterations,flat_results);
    let bootstrap_results = open_bootstrap.run(partition_seed, partition_size);

    bootstrap_results
}