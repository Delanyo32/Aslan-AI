use apalis::prelude::{Job, JobContext, JobResult, JobError};
use aslan_data::DataNode;
use serde::{Serialize, Deserialize};
use log::{info};
use rand::prelude::*;
use crate::db::mongodb::MongoClient;
use crate::api::predict::{generate_prediction};

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainJob {
    pub symbol: String,
    pub path: String,
    pub market: String,
    pub status: Status,
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

pub async fn build_model(symbol: String, path: String, market: String) {
    info!("Building data model for {}", symbol);
    let mongo_client = MongoClient::new().await;
    let data = mongo_client.get_symbol_data(symbol.clone(), path.clone(), market.clone()).await;

    //truncate data from full data for testing
    info!("Truncating data for testing. Data Size {}", data.len());
    let lenght  = data.len() /4;
    info!("Truncating data for testing. Truncating Size {}", lenght);
    let test_data = data.as_slice()[data.len()-lenght..].to_vec();

    //initialize the data
    info!("Initializing data");
    let (_, mut nodes) = initialize_data(&data);

    //evaluate loss function (std of the model)
    info!("Evaluating loss function");
    let loss_breakdown = evaluate_loss_function(&test_data, &nodes, 7, "Initialisation Stage".to_string());

    info!("Training model with wavereduce");
    wavereduce_training(data, nodes.as_mut(), 100, 7);

    info!("Evaluating loss function post WaveReduce");
    let wave_loss_breakdown = evaluate_loss_function(&test_data, &nodes, 7, "Wavereduce Stage".to_string());

    //save results to db
    info!("Saving Loss results to db");
    mongo_client.save_loss_breakdown(symbol.clone(), loss_breakdown,path.clone()).await;
    mongo_client.save_loss_breakdown(symbol.clone(), wave_loss_breakdown,path.clone()).await;

    mongo_client.export_data(symbol.clone(), nodes,path.clone(),market.clone()).await;
    info!("Building data model complete");

    //adding model to the model list
    info!("Adding model to model list");
    mongo_client.add_model_entry(symbol.clone(), path.clone(),market.clone()).await;
}

pub async fn train_model(job: TrainJob, _ctx: JobContext) -> Result<JobResult, JobError> {
    build_model(job.symbol, job.path, job.market).await;
    Ok(JobResult::Success)
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

fn wavereduce_training (data: Vec<f64>,nodes: &mut Vec<DataNode>, iterations: usize, chunk_size: usize){
    let chunks: Vec<&[f64]> = data.chunks(chunk_size).collect();
    for iter in 0..iterations{
        info!("Iteration: {}", iter);
        let mut rng = rand::thread_rng();
        let random_chunk = rng.gen_range(0..chunks.len());
        let chunk = chunks[random_chunk];

        let partition_seed = chunk[0];

        let waveresultsize = 100;
        let wavereduce = aslan_wavereduce::WaveReduce::new(partition_seed, chunk_size,waveresultsize);
        let wavereduce_results = wavereduce.generate_results(nodes);

        let mut distribution:Vec<WaveDistriubtion>  = Vec::new();
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

        //sort by std
        distribution.sort_by(|a, b| a.std.partial_cmp(&b.std).unwrap());
        for dist in distribution.iter(){
            info!("{}: prediction {:?}, data {:?}", dist.std, dist.prediction, dist.data);
        }
        distribution.reverse();

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