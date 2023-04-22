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

    // load model into memory
    let model = mongodb.load_model(data.symbol.clone(),data.path.clone(),data.market.clone()).await;
    info!("Model Loaded for symbol: {}", data.symbol);

    let mut result_space =  Vec::new();

    // calculate the differences between entries
    if data.seed.len() < 2{
        let response = DataResponse {
            message: "Seed must be at least 2 entries".to_string(),
            data: None,
        };
        return Json(response)
    }
    let predection_parameters =  convert_seed(data.seed.clone());

    // find all the nodes contain the differences
    for (index, parameter) in predection_parameters.iter().enumerate(){
        
        let partition_size = predection_parameters.len() + data.size - (index+1);
        // make this a parameter in the future
        let wave_result_size = 100;

        let wavereduce = aslan_wavereduce::WaveReduce::new(*parameter, partition_size,wave_result_size);
        let wavereduce_results = wavereduce.generate_results(&model);

        //print a random result from the wavereduce results
        let flat_results = aslan_wavereduce::WaveReduceSolution::flatten_results(&wavereduce_results.results);
        info!("---------------------------------");
        info!("Parameter: {}", parameter);
        info!("Partition Size: {}", partition_size);
        info!("Wave Result Size: {}", wavereduce_results.results.len());
        info!("Result Size: {}", flat_results.len());
        for result in flat_results{
            let (_, right) = result.split_at(result.len() - data.size);
            result_space.push(right.to_vec());
        }
        info!("---------------------------------");
    }

    let boostrap_iterations = 100;
    let open_bootstrap = aslan_bootstrap::Bootstrap::new(boostrap_iterations,result_space);
    let bootstrap_results = open_bootstrap.run(data.seed[data.seed.len()-1], data.size);

    let response = DataResponse {
        message: "Predicted Data".to_string(),
        data: Some(bootstrap_results),
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

    //flatten the top results
    let flat_results = aslan_wavereduce::WaveReduceSolution::flatten_results(&wavereduce_results.results);

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