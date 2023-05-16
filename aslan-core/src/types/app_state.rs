use apalis::prelude::{Job, JobContext, JobResult, JobError};
use aslan_data::DataNode;
use serde::{Serialize, Deserialize};
use log::{info};
use rand::prelude::*;
use crate::api::task;
use crate::db::mongodb::MongoClient;
use crate::api::predict::{generate_prediction};
use logging_timer::{time, stime};

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainJob {
    pub symbol: String,
    pub path: String,
    pub market: String,
    pub status: Status,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobList {
    pub done: Vec<String>,
    pub pending: Vec<String>,
    pub running: Vec<String>,
    pub failed: Vec<String>,
    pub retry: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    Pending,
    Running,
    Complete,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LossItem{
    pub iteration: usize,
    pub loss: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LossBreakdown{
    pub stage: String,
    pub loss_items: Vec<LossItem>,
    pub average_loss: f64,
    pub average_price: f64,
    pub price_to_loss_ratio: f64,
}

pub struct WaveDistriubtion{
    pub std: f64,
    pub data: Vec<f64>,
    pub prediction: Vec<f64>,
    pub generated_data: Vec<f64>,
}

impl Job for TrainJob {
    const NAME: &'static str = "apalis::TrainJob";
}

pub async fn build_model_v2(symbol: String, path: String, market: String){
    // get symbols from database
    let mongo_client = MongoClient::new().await;
    let symbols = mongo_client.get_symbols(market.clone()).await;
    let mut tasks = Vec::new();
    let mut full_normalized_data = Vec::new();
    let mut full_data = Vec::new();
    for symbol in symbols{
        let symbol = symbol.clone();
        let path = path.clone();
        let market = market.clone();
        let mongo_client = mongo_client.clone();
        let task = tokio::spawn(async move {
            //get data from database
            info!("Getting data from database");
            let data = mongo_client.get_symbol_data(symbol, path, market).await;

            //checking for size
            if data.len() < 2 {
                info!("Data size is too small. Skipping");
                return (Vec::new(), Vec::new());
            }

            info!("Normalizing data");
            let normalized_data = aslan_data::AslanDataChunks::normalize_data(&data);
            (normalized_data, data)
        });
        tasks.push(task);
    }

    for task in tasks{
        let  (mut data,mut normalized_data) = task.await.unwrap();
        
        let mut_norm = normalized_data.as_mut();
        let mut_data = data.as_mut();

        full_data.append(mut_data);
        full_normalized_data.append(mut_norm);
    }

    info!("Initializing data");
    let (_, mut nodes) = initialize_data_v2(&full_normalized_data);

    info!("Training model with wavereduce");
    wavereduce_training(full_data, nodes.as_mut(), 100, 7);

    mongo_client.export_data("OMEGA".to_string(), nodes,path.clone(),market.clone()).await;
    info!("Building data model complete");

    //adding model to the model list
    info!("Adding model to model list");
    mongo_client.add_model_entry("OMEGA".to_string(), path.clone(),market.clone()).await;

}
// TODO: propergate errors up stack
pub async fn build_model(symbol: String, path: String, market: String) {
    info!("Building data model for {}", symbol);
    let mongo_client = MongoClient::new().await;

    let mut normalized_data = Vec::new();
    let mut data = Vec::new();
    
    let mut open_data = mongo_client.get_symbol_data(symbol.clone(), "OPEN".to_string(), market.clone()).await;
    let mut high_data = mongo_client.get_symbol_data(symbol.clone(), "HIGH".to_string(), market.clone()).await;
    let mut low_data = mongo_client.get_symbol_data(symbol.clone(), "LOW".to_string(), market.clone()).await;
    let mut close_data = mongo_client.get_symbol_data(symbol.clone(), "CLOSE".to_string(), market.clone()).await;

    info!("Normalizing data");
    let mut normalized_open = aslan_data::AslanDataChunks::normalize_data(&open_data);
    let mut normalized_high = aslan_data::AslanDataChunks::normalize_data(&high_data);
    let mut normalized_low = aslan_data::AslanDataChunks::normalize_data(&low_data);
    let mut normalized_close = aslan_data::AslanDataChunks::normalize_data(&close_data);

    info!("Concatenating Normalized data");
    normalized_data.append(normalized_open.as_mut());
    normalized_data.append(normalized_high.as_mut());
    normalized_data.append(normalized_low.as_mut());
    normalized_data.append(normalized_close.as_mut());

    info!("Concatenating data");
    data.append(open_data.as_mut());
    data.append(high_data.as_mut());
    data.append(low_data.as_mut());
    data.append(close_data.as_mut());

    //initialize the data
    info!("Initializing data");
    let (_, mut nodes) = initialize_data_v2(&data);

    info!("Training model with wavereduce");
    wavereduce_training(data, nodes.as_mut(), 100, 7);

    mongo_client.export_data(symbol.clone(), nodes,"OMEGA".to_string(),market.clone()).await;
    info!("Building data model complete");

}



fn initialize_data(data: &Vec<f64>) -> (Vec<f64>, Vec<DataNode>) {
    let normalized_data = aslan_data::AslanDataChunks::normalize_data(data);
    let mut nodes_v2 = aslan_data::DataNode::generate_nodes(&normalized_data, 0.07);
    aslan_data::DataNode::initialize_node_edges(nodes_v2.as_mut());
    let averaged_data = aslan_data::DataNode::parse_data(&nodes_v2, &normalized_data);
    aslan_data::DataNode::set_distance_scores(nodes_v2.as_mut(), &averaged_data);
    aslan_data::DataNode::set_weights(nodes_v2.as_mut());
    (averaged_data, nodes_v2)
}

#[time]
fn initialize_data_v2(normalized_data: &Vec<f64>) -> (Vec<f64>, Vec<DataNode>) {
    info!("Generating nodes");
    let mut nodes_v2 = aslan_data::DataNode::generate_nodes(&normalized_data, 0.07);
    info!("Setting distance scores");
    aslan_data::DataNode::set_distance_scores(nodes_v2.as_mut(), normalized_data);
    info!("Setting weights");
    aslan_data::DataNode::set_weights(nodes_v2.as_mut());
    info!("Initialization complete");
    (normalized_data.to_vec(), nodes_v2)
}


fn wavereduce_training (data: Vec<f64>,nodes: &mut Vec<DataNode>, iterations: usize, chunk_size: usize){
    // create chunks of the data which will be used to refine the model
    let chunks: Vec<&[f64]> = data.chunks(chunk_size).collect();

    // number of times to run the refinement
    for iter in 0..iterations{
        info!("Iteration: {}", iter);
        // randomly select a chunk to use for the refinement
        let mut rng = rand::thread_rng();
        let random_chunk = rng.gen_range(0..chunks.len());
        let chunk = chunks[random_chunk];

        // randomly select a node to use as the seed for the refinement
        let partition_seed = chunk[0];

        // generate the results of the refinement
        let waveresultsize = 100;
        let wavereduce = aslan_wavereduce::WaveReduce::new(partition_seed, chunk_size,waveresultsize);
        let wavereduce_results = wavereduce.generate_results(nodes);

        let mut distribution:Vec<WaveDistriubtion>  = Vec::new();

        // calculate the standard deviation between the generated data and the actual data
        for result in wavereduce_results.results{
            let generated:Vec<f64> = result.result.into_iter().map(|x| x.state ).collect();
            let denomalized = denormalize_data(partition_seed, &generated);
            let mut sum = 0.0;
            for (i, value) in chunk.iter().enumerate(){
                sum += (value - denomalized[i]).powi(2);
            }
            let std = (sum / (chunk.len() as f64)).sqrt();

            let wave_distribution = WaveDistriubtion{
                std: std,
                data: chunk.to_vec(),
                prediction: denomalized,
                generated_data: generated,
            };
            distribution.push(wave_distribution);
        }

        //sort from the lowest standard deviation to the highest
        distribution.sort_by(|a, b| a.std.partial_cmp(&b.std).unwrap());
        for dist in distribution.iter(){
            info!("{}: prediction {:?}, data {:?}", dist.std, dist.prediction, dist.data);
        }
        distribution.reverse();

        // update the nodes with the new data
        for (i,dist) in distribution.iter().enumerate(){
            let node_pointer = dist.generated_data[0];
            let found_node = nodes.iter_mut().find(|x| x.average == node_pointer).unwrap();
            
            for data in 1..dist.generated_data.len(){
                found_node.update_edge(dist.generated_data[data], i as f64);
            }
        }
        aslan_data::DataNode::set_weights(nodes);
    }
    
}

fn evaluate_loss_function(test_data: &Vec<f64>, nodes: &Vec<DataNode>, chunk_size: usize,stage:String) -> LossBreakdown {
    // create chunks of test data
    let chunks = test_data.chunks(chunk_size);
    let mut loss_breakdown = Vec::new();
    for (i,test_entry )in chunks.enumerate(){
        let prediction = generate_prediction(&nodes, test_entry[0], test_entry.len());
        info!("Test Data: {:?}", test_entry);
        info!("Prediction: {:?}", prediction.generated_data);

        //calculate standard deviation of the prediction
        let mut sum = 0.0;
        for (i, value) in test_entry.iter().enumerate(){
            sum += (value - prediction.generated_data[i]).powi(2);
        }
        let std = (sum / (test_entry.len() as f64)).sqrt();
        info!("Loss/Std: {} At Iteration {}", std, i);
        let loss_item = LossItem{
            iteration: i,
            loss: std,
        };
        loss_breakdown.push(loss_item);
    }
    
    //calculate average loss
    let mut sum = 0.0;
    for loss_item in &loss_breakdown{
        sum += loss_item.loss;
    }
    let average_loss = sum / (loss_breakdown.len() as f64);
    info!("Average Loss: {}", average_loss);
    let average_price = test_data.iter().sum::<f64>() / (test_data.len() as f64);
    LossBreakdown{
        stage: stage,
        loss_items: loss_breakdown,
        average_loss: average_loss,
        average_price: average_price,
        price_to_loss_ratio: average_price / average_loss,
    }

}


fn denormalize_data(seed: f64, data: &Vec<f64>) -> Vec<f64> {
    let mut denormalized_data = Vec::new();
    denormalized_data.push(seed);
    let mut last_value = seed;
    for value in data{
        let denormalized_value = last_value + value;
        denormalized_data.push(denormalized_value);
        last_value = denormalized_value;
    }
    denormalized_data
}