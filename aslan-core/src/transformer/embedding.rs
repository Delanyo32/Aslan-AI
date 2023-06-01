use log::info;
use rand::Rng;
use serde::{Serialize, Deserialize};

use polars::prelude::*;
use crate::db::mongodb::MongoClient;
use polars::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Embedding {
    pub token: f64,
    pub input_layer: Vec<f64>,
    pub output_layer: Vec<f64>,
}

// implement clone for embedding
impl Clone for Embedding {
    fn clone(&self) -> Self {
        Embedding {
            token: self.token,
            input_layer: self.input_layer.clone(),
            output_layer: self.output_layer.clone(),
        }
    }
}

// get all the collections from the token database
// for each collection, get the data
// find the unique tokens in the data
pub async fn generate_unique_tokens() {

    // get collections in the token database
    let mongo_client = MongoClient::new().await;
    let collections = mongo_client.list_collections("aslan-tokens".to_string()).await;

    let mut unique_tokens = Vec::new();
    // for each collection, get the data
    for collection in collections {
        // print collection
        info!("Collection: {}", collection);

        // for each collection, get the data
        let entries = mongo_client.get_collection(collection.clone(), "aslan-tokens".to_string()).await;

        for entry in entries {
            let vec_data = entry;

            let mut combined_tokens = unique_tokens.clone();
            combined_tokens.append(&mut vec_data.clone());

            
            let series: Series = Series::new("tokens", combined_tokens);
            let unique_values: Series = series.unique().unwrap();
            unique_tokens = match unique_values.f64(){
                Ok(values) => values.into_no_null_iter().collect(),
                Err(_) => Vec::new(),
            }
        }
    }
    //println!("Unique tokens: {:?}", unique_tokens);
    info!("Unique tokens size: {:?}", unique_tokens.len());
    
    // initialize embeddings
    let embedings = initialize_embedings(unique_tokens, 5).await;

    // save embeddings to database
    mongo_client.insert_embeddings(embedings).await;
    info!("Embeddings saved to database");

}

pub async fn initialize_embedings(tokens: Vec<f64>, parameters_size: usize)->Vec<Embedding>{
    let mut embeddings = Vec::new();
    for token in tokens {
        let mut rng = rand::thread_rng();
        let random_numbers: Vec<f64> = (0..parameters_size)
            .map(|_| rng.gen_range(-1.00..=1.00))
            .collect();

        let embedding = Embedding {
            token,
            input_layer: random_numbers.clone(),
            output_layer: random_numbers.clone(),
        };
        embeddings.push(embedding);
    }
    info!("Initialized embeddings");
    embeddings
} 

pub async fn train_model(){
    // using sample data for now
    // only 3 possible tokens
    let d1 = Embedding {
        token: 1.0,
        input_layer: vec![0.1, 0.2, 0.3, 0.4, 0.5],
        output_layer: vec![0.1, 0.2, 0.3, 0.4, 0.5],
    };
    let d2 = Embedding {
        token: -0.2,
        input_layer: vec![0.2, 0.3, 0.4, 0.5, 0.6],
        output_layer: vec![0.2, 0.3, 0.4, 0.5, 0.6],
    };
    let d3 = Embedding {
        token: 5.0,
        input_layer: vec![0.3, 0.4, 0.5, 0.6, 0.7],
        output_layer: vec![0.3, 0.4, 0.5, 0.6, 0.7],
    };

    let mut embeddings = Vec::new();
    embeddings.push(d1);
    embeddings.push(d2);
    embeddings.push(d3);

    let input_values = vec![1.0, -0.2, 5.0];
    make_prediction(input_values, embeddings).await;

}

pub async fn make_prediction(input_values: Vec<f64>, embeddings : Vec<Embedding>){

    // if input value in embeddings, return the parameters
    // this mimicts multiplying the input values by the weights
    let mut input_layer = Vec::new();
    let mut output_layer = Vec::new();

    for input in input_values {
        let input_embeddings = embeddings.clone();
        let input_vec =  input_embeddings.iter().filter(|&value|value.token == input).cloned().collect::<Vec<Embedding>>();
        let input_vec = input_vec.iter().flat_map(|e|e.input_layer.clone()).collect::<Vec<f64>>();
        let series = Series::new(input.to_string().as_str(), input_vec);
        input_layer.push(series.clone());
    }

    let out_embeddings = embeddings.clone();
    for out in out_embeddings {
        let out_vec = out.output_layer.clone();
        let series = Series::new(out.token.to_string().as_str(), out_vec);
        output_layer.push(series.clone());
    }
    

    // sum the layers to get the
    let sum = vec![0.0; input_layer[0].len()];
    let mut sum_series = Series::new("sum", sum);

    for series in input_layer {
        sum_series = sum_series + series;
    }

    // pass the sum to the linear activation function Skipped due to no need to implement REVISIT LATER
    // multiply a the sum with each output layer and sum the results
    let mut soft_max_input = Vec::new();
    for series in output_layer {
        let result = sum_series.clone() * series.clone();
        let out_sum = result.sum::<f64>().unwrap();
        let out_sum = vec![out_sum];
        let series = Series::new(series.name(), out_sum);
        soft_max_input.push(series);
    }

    //println!("Softmax input: {:?}", soft_max_input);

    // pass the sum to the softmax activation function
    let soft_max = softmax(soft_max_input);
    println!("Softmax: {:?}", soft_max);
    
}

pub fn softmax(set: Vec<Series>) -> Vec<Series>{
    let exps = set.iter().map(|x| x.exp()).collect::<Vec<Series>>();
    let mut exp_sum = Series::new("exp_sum", vec![0.0]);
    for exp in exps {
        exp_sum = exp_sum + exp;
    }
    let mut soft_max = Vec::new();

    for soft in  set{
        let soft = soft.exp() / exp_sum.clone();
        soft_max.push(soft);
    }
    soft_max
}