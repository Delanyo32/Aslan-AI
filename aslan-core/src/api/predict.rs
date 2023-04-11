use std::char::UNICODE_VERSION;

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
    data: Option<BootstrapResult>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct PredictParameters{
    pub symbol: String,
    pub market: String,
    pub path: String,
    pub seed: Vec<f64>,
    pub size: usize,
}


#[post("/predict")]
pub async fn generate(data: web::Json<PredictParameters>) -> Json<DataResponse> {
    let mongodb = MongoClient::new().await;
    // check if model exists in models metadata database
    if !mongodb.find_model_entry(data.symbol.clone(),data.path.clone()).await{
        let response = DataResponse {
            message: "Model does not exist".to_string(),
            data: None,
        };
        return Json(response)
    }

    // calculate the differences between entries
    let predection_parameters =  convert_seed(data.seed.clone());

    let mut nodes: Vec<DataNode> = Vec::new();
    // find all the nodes contain the differences
    for parameter in predection_parameters.iter(){
        let node = mongodb.find_node(data.symbol.clone(),data.path.clone(),data.market.clone(),*parameter).await;
        nodes = [node,nodes].concat();
    }

    //1. create wavereduce arrays based on the first node and fetching likely nodes from mongodb
    //2. overlay the wavereduce arrays to create a single array 
    //3. use boostrap to generate the prediction
    
    // info!("Generating prediction for symbol: {}", data.symbol);
    
    // let open_nodes = mongodb.get_node_data(&path.symbol).await;

    // info!("Model Loaded for symbol: {}", path.symbol);
    // let prediction = generate_prediction(&open_nodes, path.seed, path.size);

    let response = DataResponse {
        message: "Predicted Data".to_string(),
        data: None,
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

// convert seed array into  differences
pub fn convert_seed(seed: Vec<f64>) -> Vec<f64> {
    let mut differences: Vec<f64> = Vec::new();
    let mut previous = seed[0];
    for i in 1..seed.len() {
        let current = seed[i];
        let difference = current - previous;
        //convert difference to 2 decimal places
        let difference = (difference * 100.0).round() / 100.0;
        differences.push(difference);
        previous = current;
    }
    differences
}