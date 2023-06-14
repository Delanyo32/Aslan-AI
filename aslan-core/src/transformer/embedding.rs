use log::info;
use rand::Rng;
use serde::{Serialize, Deserialize};

use polars::prelude::*;
use crate::{db::mongodb::MongoClient, api::predict};
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
    let embedings = initialize_embedings(unique_tokens, 100).await;

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
    // initialize embeddings and the data
    let (mut embeddings, input_set, output_set) = initialize_data().await;
    info!("Initialized embeddings and data");
    // make 15 iterations
    for iteration in 0..2 {
        info!("Starting Itteration: {}", iteration);
        let mut cross_entropy_set = Vec::new();
        let mut predictions = Vec::new();
        let mut linear_activations_set = Vec::new();
        let mut expected_prediction_probability_set = Vec::new();
    
        info!("Making predictions");
        for (index,input) in input_set.iter().enumerate() {
            // make prediction
            let (softmax,linear_activations) = make_prediction(input.clone(), embeddings.clone()).await;
    
            linear_activations_set.push(linear_activations);
            
            // get the series with the expected output and calculate cross entropy
            let expected_prediction = output_set[index];
            let expected_prediction= get_expected_prediction_probability(softmax.clone(), expected_prediction);
    
            expected_prediction_probability_set.push(expected_prediction.clone());
            let (epp_name,epp_value) = get_value(expected_prediction);
    
            // calculate cross entropy
            let cross_entropy = calculate_cross_entropy(epp_value);
            // add cross entropy to cross entropy set
            let cross_entropy_series = Series::new(epp_name.as_str(), vec![cross_entropy]);
            cross_entropy_set.push(cross_entropy_series);
    
            let prediction = get_max_value(softmax);
            predictions.push(prediction);
            
        }
    
        // calculate the total cross entropy
        let total_cross_entropy = calculate_total_cross_entropy(cross_entropy_set);
        println!("Iteration {:?} Total cross entropy: {:?}", iteration,total_cross_entropy);
    
        // improve layer 2 embeddings
        let layer_2_embeddings = calculate_new_layer_2_embeddings(embeddings.clone(), predictions.clone(), linear_activations_set.clone(), expected_prediction_probability_set.clone());
    
        // improve layer 1 embeddings
        let layer_1_embeddings = calculate_new_layer_1_embeddings(embeddings.clone(), predictions.clone());
    
        // combine new embeddings
        let mut new_embeddings = Vec::new();
        for (i, embedding) in layer_2_embeddings.iter().enumerate() {
            let new_embedding = Embedding{
                token: embedding.token,
                input_layer: layer_1_embeddings[i].input_layer.clone(),
                output_layer: layer_2_embeddings[i].output_layer.clone(),
            };
            new_embeddings.push(new_embedding);
        }
        embeddings = new_embeddings;
    }

}

//given a series, returns the name and value as a tuple
pub fn get_value(series: Series) -> (String, f64) {
    let name = series.name().parse::<f64>().unwrap_or(0.0);
    let value = match series.f64(){
        Ok(values) => values.into_no_null_iter().collect(),
        Err(_) => Vec::new(),
    };
    let value = value[0];
    (name.to_string(), value)
}

pub fn calculate_new_layer_1_embeddings(embeddings : Vec<Embedding>,predictions_set: Vec<Series>)->Vec<Embedding>{
    let mut new_embeddings = Vec::new();
    let embeddings_clone = embeddings.clone();

    // num embeddings(~6000) *(num predictions(~560) * num weights(~100))
    for (e_index,embedding) in embeddings.iter().enumerate(){
        let weights = embedding.input_layer.clone();
        let w_set = &embeddings_clone[e_index].output_layer;
        
        let mut new_weights = Vec::new();
        let learning_rate = 0.1;

        // num predictions(~560) * num weights(~100)
        for (index,weight) in weights.iter().enumerate(){

            let mut total_slope = 0.0;

            //num predictions(~560)
            for (i, prediction) in predictions_set.iter().enumerate(){
                let (_p_name,p_value) = get_value(prediction.clone());
                let w = w_set[index];
                let slope = -1.0/p_value *(p_value*(1.0-p_value))*w;
                total_slope += slope;
            }
            // calculate the new weight
            let step_size = total_slope * learning_rate;
            let new_weight = weight - step_size;
            new_weights.push(new_weight);
        }

        let new_embedding = Embedding{
            token: embedding.token,
            input_layer: new_weights,
            output_layer: embedding.output_layer.clone(),
        };
        new_embeddings.push(new_embedding);
    }
    new_embeddings
}

//each embedding (e) maps to a linear activation (y) e->y
pub fn calculate_new_layer_2_embeddings(embeddings : Vec<Embedding>, predictions_set: Vec<Series>, linear_activations_set: Vec<Series>, expected_prediction_set: Vec<Series>) -> Vec<Embedding>{
    let mut new_embeddings = Vec::new();

    for embedding in embeddings {

        let weights = embedding.output_layer;
        let mut new_weights = Vec::new();

        for (_,weight) in weights.iter().enumerate(){
            let (_,y) = get_value(linear_activations_set[0].clone());

            let mut total_slope = 0.0;
            let learning_rate = 0.1;
            for (i, prediction) in predictions_set.iter().enumerate(){
                let (p_name,p_value) = get_value(prediction.clone());

                let expected_prediction = &expected_prediction_set[i];
                let (ep_name, ep_value) = get_value(expected_prediction.clone());

                // check if expected prediction is equal to the actual prediction
                let mut slope = 0.0;
                

                if p_name == ep_name{
                    // calculate the new weight
                    slope = -1.0/p_value *(p_value * (1.0-p_value)) * y;
                }else{
                    slope = -1.0/p_value *(-ep_value *p_value) * y;
                }
                total_slope += slope;
            }

            // calculate the new weight
            let step_size = total_slope * learning_rate;
            let new_weight = weight - step_size;
            new_weights.push(new_weight);
        }
        // update the weights
        let new_embedding = Embedding{
            token: embedding.token,
            input_layer: embedding.input_layer,
            output_layer: new_weights,
        };
        new_embeddings.push(new_embedding);
    }
    new_embeddings
}

// function to get the input and output data
pub async fn initialize_data() -> (Vec<Embedding>, Vec<Vec<f64>>, Vec<f64>){
    // using sample data for now
    // only 3 possible tokens
    // get data from database
    let mongo_client = MongoClient::new().await;
    let db_embeddings = mongo_client.get_embeddings().await;

    let collections = mongo_client.list_collections("aslan-tokens".to_string()).await;
    let mut input_set = Vec::new();
    let mut output_set = Vec::new();

    for collection in collections {
        let entries = mongo_client.get_collection(collection.clone(), "aslan-tokens".to_string()).await;
    
        for entry in entries {
            for e in entry.clone(){
                input_set.push(vec![e]);
            }
    
            // get the output by shifting the input by 1
            let mut output = entry.clone();
            output.remove(0);
            output_set.append(&mut output);
            break;
        }
        break;
    }

    input_set.remove(input_set.len()-1);

    return (db_embeddings, input_set, output_set);

}

// function to calculate the total cross entropy
pub fn calculate_total_cross_entropy(cross_entropy_set: Vec<Series>) -> f64{
    let mut total_cross_entropy = 0.0;
    for series in cross_entropy_set {
        let cross_entropy = match series.f64(){
            Ok(values) => values.into_no_null_iter().collect::<Vec<f64>>()[0],
            Err(_) => 0.0,
        };
        total_cross_entropy += cross_entropy;
    }
    total_cross_entropy
}

//function to calculate the cross entropy
pub fn calculate_cross_entropy(prediction: f64) -> f64{
    let cross_entropy = -1.0 * prediction.ln();
    cross_entropy
}

pub fn get_expected_prediction_probability(soft_max_set: Vec<Series>, expected_prediction: f64) -> Series{
    let mut probability = Series::new("probability", vec![0.0]);
    for series in soft_max_set {
        let prediction_name = series.name();
        let prediction_float = prediction_name.parse::<f64>().unwrap();

        if prediction_float == expected_prediction {
            probability = series;   
        }
    }
    probability
}


pub async fn make_prediction(input_values: Vec<f64>, embeddings : Vec<Embedding>) -> (Vec<Series>, Series){

    let (input_layer, output_layer) = get_inputs_and_output_vectors(input_values, embeddings);
    
    // sum the layers to get the
    let sum = vec![0.0; input_layer[0].len()];
    let mut sum_series = Series::new("sum", sum);

    for series in input_layer {
        sum_series = sum_series + series;
    }

    let linear_activations = linear_activation_function(sum_series);

    // multiply a the sum with each output layer and sum the results
    let soft_max_input = output_layer_activation(output_layer, linear_activations.clone());

    // pass the sum to the softmax activation function
    let soft_max = softmax(soft_max_input);
 
    (soft_max, linear_activations)
    
}


pub fn linear_activation_function(sum: Series) -> Series{
    // Skipped due to no need to implement REVISIT LATER
    // f(x) = x
    sum
}

pub fn get_inputs_and_output_vectors(input_values: Vec<f64>, embeddings : Vec<Embedding>)-> (Vec<Series>, Vec<Series>){
    let mut input_layer = Vec::new();
    let mut output_layer = Vec::new();

    for input in input_values {
        let input_embeddings = embeddings.clone();
        let input_vec =  input_embeddings.iter().filter(|&value|value.token == input).cloned().collect::<Vec<Embedding>>();
        let input_vec = input_vec[0].input_layer.clone();
        let series = Series::new(input.to_string().as_str(), input_vec);
        input_layer.push(series.clone());
    }

    let out_embeddings = embeddings.clone();
    for out in out_embeddings {
        let out_vec = out.output_layer.clone();
        let series = Series::new(out.token.to_string().as_str(), out_vec);
        output_layer.push(series.clone());
    }
    (input_layer, output_layer)
}

pub fn get_max_value(soft_max_set: Vec<Series>) -> Series{
    // get the highest value from the softmax array
    let mut max_value = soft_max_set[0].clone();
    for series in soft_max_set.iter(){
        let series_sum = series.sum::<f64>().unwrap();
        let max_value_sum = max_value.sum::<f64>().unwrap();
        if series_sum > max_value_sum {
            max_value = series.clone();
        }
    }
    return max_value;
}

pub fn output_layer_activation(output_layer: Vec<Series>, sum_series: Series) -> Vec<Series>{
    let mut soft_max_input = Vec::new();
    for series in output_layer {
        let result = sum_series.clone() * series.clone();
        let out_sum = result.sum::<f64>().unwrap();
        let out_sum = vec![out_sum];
        let series = Series::new(series.name(), out_sum);
        soft_max_input.push(series);
    }
    soft_max_input
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