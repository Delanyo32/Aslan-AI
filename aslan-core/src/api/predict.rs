use std::char::UNICODE_VERSION;

use actix_web::{
    post,
    web::{self, Json},
};
use apalis::{prelude::{JobContext, Storage,JobResult, Job, JobError}, postgres::PostgresStorage};
use aslan_bootstrap::BootstrapResult;
use aslan_data::DataNode;
use serde::{Deserialize, Serialize};
use log::{info, error};

use crate::db::mongodb::MongoClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct DataResponse {
    message: String,
    data: Option<BootstrapResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictJob {
    pub symbol: String,
    pub path: String,
    pub market: String,
    pub seed: Vec<f64>,
    pub size: usize,
}

impl Job for PredictJob {
    const NAME: &'static str = "apalis::PredictJob";
}


#[derive(Debug, Serialize, Deserialize)]
pub struct PredictParameters{
    pub id: String,
    pub symbol: String,
    pub market: String,
    pub path: String,
    pub seed: Vec<f64>,
    pub size: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictDataResponse {
    message: String,
}


#[post("/addPredictionJob")]
pub async fn add_predict_job(body: web::Json<PredictParameters>, storage: web::Data<PostgresStorage<PredictJob>>) -> Json<PredictDataResponse> {
    let new_job = PredictJob {
        symbol: body.symbol.clone(),
        path: body.path.clone(),
        market: body.market.clone(),
        seed: body.seed.clone(),
        size: body.size,
    };
    let storage = &*storage.into_inner();
    let mut storage = storage.clone();
    let res = storage.push(new_job).await;

    info!("Adding prediction to queue: {}", body.symbol);  
    match res {
        Ok(()) => {
            info!("Model request added to queue");
            let response = PredictDataResponse {
                message: "Prediction Job Added To Queue".to_string(),
            };
            Json(response)
        },
        Err(e) => {
            error!("Error adding job to queue: {}", e);
            let response = PredictDataResponse {
                message: format!("Error adding prediction Job to Queue: {}", e),
            };
            Json(response)
        },
    } 

}

pub async fn predict_job(job: PredictJob, _ctx: JobContext) -> Result<JobResult, JobError> {
    //TODO propergate errors up stack to control job result
    //build_model(job.symbol, job.path, job.market).await;
    let params  = PredictParameters{
        id: "0".to_string(), //TODO: fix this hack",
        symbol: job.symbol.clone(),
        path: job.path.clone(),
        market: job.market.clone(),
        seed: job.seed.clone(),
        size: job.size,
    };

    let final_results = generate_results(params).await;
    info!("Final Results: {:?}", final_results);
    
    Ok(JobResult::Success)
}

#[post("/predict")]
pub async fn generate(data: web::Json<PredictParameters>) -> Json<DataResponse> {
    let params  = PredictParameters{
        id: data.id.clone(),
        symbol: data.symbol.clone(),
        path: data.path.clone(),
        market: data.market.clone(),
        seed: data.seed.clone(),
        size: data.size,
    };
    
    let final_results = generate_results(params).await;
    info!("Final Results: {:?}", final_results);

    let response = DataResponse {
        message: "Predicted Data".to_string(),
        data: Some(BootstrapResult { generated_data: final_results
            , average_data: vec![] }),
    };
    Json(response)
}

pub async fn generate_results( data: PredictParameters)-> Vec<f64>{
    let mongodb = MongoClient::new().await;
    

    let symbols = mongodb.get_symbols(data.market.clone()).await;
    let mut predictions = Vec::new();
    let mut tasks = Vec::new();
 

    for symbol in symbols {
        let symbol = symbol.clone();
        let path = data.path.clone();
        let market = data.market.clone();
        let size = data.size;
        let seed = data.seed.clone();
        let mongodb = mongodb.clone();
        let task = tokio::spawn( async move  {
        
            let prediction = predict(symbol, path, market, size, seed, &mongodb).await;
            return prediction;
        });
        tasks.push(task);
    }

    for task in tasks {
        let prediction = task.await.unwrap();
        match prediction {
            Ok(prediction) => {
                info!("Prediction Generated");
                predictions.push(prediction.generated_data);
            },
            Err(e) => {
                info!("Prediction Failed: {}", e);
                continue;
            }
        }
    }
    let mut final_results = Vec::new();

    let prediction_size = predictions[0].len();
    
    for i in 0..prediction_size{
        let mut sum = 0.0;
        for prediction in &predictions{
            sum += prediction[i];
        }
        //change result to 2 decimal places
        let entry = (sum/predictions.len() as f64 * 100.0).round() / 100.0;
        final_results.push(entry);
    }
    return final_results;

}

// gets a model based on a symbol and generates a prediction
pub async fn predict(symbol: String, path: String, market: String, size: usize,seed: Vec<f64>, mongodb: &MongoClient) -> Result<BootstrapResult,String> {

    // check if model exists in models metadata database
    if !mongodb.find_model_entry(symbol.clone(),path.clone()).await{
        return Err("Model does not exist".to_string());
    }

    // load model into memory
    let model = mongodb.load_model(symbol.clone(),path.clone(),market.clone()).await;
    info!("Model Loaded for symbol: {}", symbol);

    let mut result_space =  Vec::new();

    // calculate the differences between entries
    if seed.len() < 2{
        return Err("Seed must be at least 2 entries".to_string());
    }
    let predection_parameters =  convert_seed(seed.clone());

    // find all the nodes contain the differences
    for (index, parameter) in predection_parameters.iter().enumerate(){
        
        let partition_size = predection_parameters.len() + size - (index+1);
        // make this a parameter in the future
        let wave_result_size = 100;

        let wavereduce = aslan_wavereduce::WaveReduce::new(*parameter, partition_size,wave_result_size);
        let wavereduce_results = wavereduce.generate_results(&model);

        //print a random result from the wavereduce results
        let flat_results = aslan_wavereduce::WaveReduceSolution::flatten_results(&wavereduce_results.results);
        for result in flat_results{
            let (_, right) = result.split_at(result.len() - size);
            result_space.push(right.to_vec());
        }
    }

    let boostrap_iterations = 100;
    let open_bootstrap = aslan_bootstrap::Bootstrap::new(boostrap_iterations,result_space);
    let bootstrap_results = open_bootstrap.run(seed[seed.len()-1], size);
    Ok(bootstrap_results)
    //
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