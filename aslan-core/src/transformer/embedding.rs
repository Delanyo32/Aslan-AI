use log::info;
use rand::Rng;
use sentry::Data;
use serde::{Serialize, Deserialize};

use polars::prelude::*;

use crate::{db::mongodb::MongoClient, api::predict};
use std::time::Instant;

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
    let start = Instant::now();
    // initialize embeddings and the data
    let (embeddings, input_set, output_set) = initialize_data().await;

    let elapsed = start.elapsed();
    info!("Time elapsed in initialize_data() is: {:?}", elapsed);
    info!("Initialized embeddings and data");
    // make 15 iterations
    for iteration in 0..1 {
        let start = Instant::now();
        info!("Starting Itteration: {}", iteration);

        // version prediction version 2 (with dataframes)
        for (_,input) in input_set.iter().enumerate() {
            let _ = generate_predictions_v2(vec![*input], output_set.clone(), embeddings.clone());
        }
        let elapsed = start.elapsed();
        info!("Time elapsed in generate_predictions_v2() is: {:?}", elapsed);
        break;

        // v1
        let mut cross_entropy_set = Vec::new();
        let mut predictions = Vec::new();
        let mut linear_activations_set = Vec::new();
        let mut expected_prediction_probability_set = Vec::new();
    
        info!("Making predictions");
        for (index,input) in input_set.iter().enumerate() {
            // make prediction
            let (softmax,linear_activations) = make_prediction(vec![*input], embeddings.clone()).await;
    
            linear_activations_set.push(linear_activations);
            
            // get the series with the expected output and calculate cross entropy
            let expected_prediction = output_set[index];
            let expected_prediction= get_expected_prediction_probability(softmax.clone(), expected_prediction);
    
            expected_prediction_probability_set.push(expected_prediction.clone());
            let (epp_name,epp_value) = get_value(expected_prediction);
    
            // calculate cross entropy
            let cross_entropy = calculate_cross_entropy(epp_value[0]);
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
    info!("Training complete");
}

pub fn generate_predictions_v2 (input_values: Vec<f64>,output_set: Vec<f64>, embeddings: Vec<Embedding>){
    info!("Making predictions");

    let (input_layer, output_layer) = get_inputs_and_output_vectors(input_values.clone(), embeddings);
    let mut prediction_df = DataFrame::new(input_layer.clone()).unwrap();

    // constructing the expression
    let input_labels = input_values.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    let mut init = lit(0.0);
    for label in input_labels{
        init = init + col(label.as_str());
    }

    // running the expression
    let total_sum = prediction_df.clone().lazy().with_columns([
        init.alias("sum"),
    ]).collect().unwrap();

    info!("total_sum: {:?}", total_sum);

    // do a linear sum of the input layers 
    // run through the linear activations
    // revisit with map or apply later
    let linear_activations_df = total_sum.clone().lazy().with_columns([
        col("sum").alias("linear_activations"),
    ]).collect().unwrap();

    info!("linear_activations_df: {:?}", linear_activations_df);
    // let linear_activations = linear_activation_function(total_sum);

    // let linear_activation_step = sum_step.with_column(linear_activations.clone()).unwrap();
    // let mut out_step =Vec::new();

    // for series in output_layer {
    //     let mut new_value = linear_activations.clone() * series.clone();
    //     new_value.rename(series.name());
    //     out_step.push(new_value);
    // }
    // let out_df = DataFrame::new(out_step).unwrap();

    // // get sum of each column
    // let sum_out = out_df.sum();

    // // get the vec series and run soft max
    // let set =  sum_out.get_columns().to_vec();

    // let softmax = softmax(set);

    // // convert softmax to dataframe
    // let softmax_df = DataFrame::new(softmax).unwrap();

    // let header = softmax_df.slice(0, 1);
    // let row = softmax_df.slice(1, 2);

    // info!("Prediction: {:?}", row);
    // info!("headers: {:?}", header);
}

//given a series, returns the name and value as a tuple
pub fn get_value(series: Series) -> (String, Vec<f64>) {
    let name = series.name().parse::<f64>().unwrap_or(0.0);
    let value = match series.f64(){
        Ok(values) => values.into_no_null_iter().collect(),
        Err(_) => Vec::new(),
    };
    (name.to_string(), value)
}

pub fn calculate_new_layer_1_embeddings(embeddings : Vec<Embedding>,predictions_set: Vec<Series>)->Vec<Embedding>{
    let mut new_embeddings = Vec::new();
    let embeddings_clone = embeddings.clone();

    let mut predictions_set_vec = Vec::new();
    for prediction in predictions_set{
        let (_p_name,p_value) = get_value(prediction);
        predictions_set_vec.push(p_value[0]);
    }

    let p_series = Series::new("predictions", predictions_set_vec.clone());
    let p_lenght = predictions_set_vec.len();
    let x = Series::new("neg_ones", vec![-1.0; p_lenght]);

    // num embeddings(~6000) *(num predictions(~560) * num weights(~100))
    for (e_index,embedding) in embeddings.iter().enumerate(){
        let weights = embedding.input_layer.clone();
        let w_set = &embeddings_clone[e_index].output_layer;
        
        let mut new_weights = Vec::new();
        let learning_rate = 0.1;

        // num predictions(~560) * num weights(~100)
        for (index,weight) in weights.iter().enumerate(){

            //version 2 
            // -1.0/p_value * (p_value*(1.0-p_value))*w;
            let w = w_set[index];
            
            let step_one = x.f64().unwrap() / p_series.f64().unwrap(); // -1.0/p_value
            info!("step_one: {:?}", step_one);
            let step_two = x.f64().unwrap() - p_series.f64().unwrap(); // (1.0-p_value)
            info!("step_two: {:?}", step_two);
            let step_three = p_series.f64().unwrap() * &step_two; // p_value*(1.0-p_value)
            info!("step_three: {:?}", step_three);
            let step_four = step_three * w; // p_value*(1.0-p_value)*w
            info!("step_four: {:?}", step_four);
            let step_five = step_one * step_four; // -1.0/p_value * (p_value*(1.0-p_value))*w;
            info!("step_five: {:?}", step_five);
            let total_slope = step_five.sum().unwrap();

            // calculate the new weight
            let step_size = total_slope * learning_rate;
            let new_weight = weight - step_size;
            new_weights.push(new_weight);

            break;
        }

        let new_embedding = Embedding{
            token: embedding.token,
            input_layer: new_weights,
            output_layer: embedding.output_layer.clone(),
        };
        new_embeddings.push(new_embedding);
        break;
    }
    new_embeddings
}

//each embedding (e) maps to a linear activation (y) e->y
pub fn calculate_new_layer_2_embeddings(embeddings : Vec<Embedding>, predictions_set: Vec<Series>, linear_activations_set: Vec<Series>, expected_prediction_set: Vec<Series>) -> Vec<Embedding>{
    let mut new_embeddings = Vec::new();

    let mut predictions_set_vec = Vec::new();
    let mut expected_e_value = Vec::new();
    let mut expected_p_values = Vec::new();

    for (index,prediction) in predictions_set.iter().enumerate(){

        let (p_name,p_value) = get_value(prediction.clone());
        predictions_set_vec.push(p_value[0]);

        let expected_prediction = &expected_prediction_set[index];
        let (ep_name, ep_value) = get_value(expected_prediction.clone());

        if p_name == ep_name{
            expected_e_value.push(p_value[0]);
            expected_p_values.push(ep_value[0]);
        }else{
            predictions_set_vec.push(p_value[0]) 
        }
    }

    // something is wrong with the p series
    let p_series = Series::new("predictions", predictions_set_vec.clone());
    let p_lenght = predictions_set_vec.len();
    let x = Series::new("neg_ones", vec![-1.0; p_lenght]);

    let e_length = expected_e_value.len();
    let a = Series::new("neg_ones", vec![-1.0; e_length]);
    let e_p_series = Series::new("expected_predictions", expected_p_values.clone());
    let e_value_series = Series::new("expected_e_value", expected_e_value.clone());

    for embedding in embeddings {

        let weights = embedding.output_layer;
        let mut new_weights = Vec::new();

        for (_,weight) in weights.iter().enumerate(){
            let (_,y) = get_value(linear_activations_set[0].clone());

            let learning_rate = 0.1;

            //version 2 
            // -1.0/p_value * (p_value*(1.0-p_value)) * y;
            // -1.0/e_p_series * (-ep_value * e_p_series) * y;

            let step_one = x.f64().unwrap() / p_series.f64().unwrap(); // -1.0/p_value
            info!("step_one: {:?}", step_one);
            let step_two = x.f64().unwrap() - p_series.f64().unwrap(); // (1.0-p_value)
            info!("step_two: {:?}", step_two);
            let step_three = p_series.f64().unwrap() * &step_two; // p_value*(1.0-p_value)
            info!("step_three: {:?}", step_three);
            let step_four = step_three * y[0]; // p_value*(1.0-p_value)*y
            info!("step_four: {:?}", step_four);
            let step_five = step_one * step_four; // -1.0/p_value * (p_value*(1.0-p_value))*y;
            info!("step_five: {:?}", step_five);
            let total_slope_1 = step_five.sum().unwrap();

            let mut total_slope_2 = 0.0;
            if e_length > 0{
                let step_one = a.f64().unwrap() / e_p_series.f64().unwrap(); // -1.0/e_p_series
                let step_two = a.f64().unwrap() * e_value_series.f64().unwrap(); //- 1.0 * ep_value = -e_value_series
                let step_three = &step_two * e_p_series.f64().unwrap(); // (-e_value_series * e_p_series)
                let step_four = step_one * step_three; // -1.0/e_p_series * (-e_value_series * e_p_series)
                let step_five = step_four * y[0]; // -1.0/e_p_series * (-e_value_series * e_p_series) * y;
                total_slope_2 = step_five.sum().unwrap();
            }
            
            let total_slope = total_slope_1 + total_slope_2;

            // calculate the new weight
            let step_size = total_slope * learning_rate;
            let new_weight = weight - step_size;
            new_weights.push(new_weight);
            break;
        }
        // update the weights
        let new_embedding = Embedding{
            token: embedding.token,
            input_layer: embedding.input_layer,
            output_layer: new_weights,
        };
        new_embeddings.push(new_embedding);
        break;
    }
    new_embeddings
}

// function to get the input and output data
pub async fn initialize_data() -> (Vec<Embedding>, Vec<f64>, Vec<f64>){
    let mongo_client = MongoClient::new().await;
    let db_embeddings = mongo_client.get_embeddings().await;

    let mut input_set = Vec::new();
    let mut output_set = Vec::new();

    let data = mongo_client.get_test_data("test-dataset".to_owned(), "test-data".to_owned()).await;
    for test_data in data {
        input_set.push(test_data.input_data);
        output_set.push(test_data.output_data);
        break;
    }

    return (db_embeddings, input_set.to_owned(), output_set.to_owned());
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


pub fn linear_activation_function_v2(sum: Series) -> Result<Series, PolarsError>{
    // Skipped due to no need to implement REVISIT LATER
    // f(x) = x
    let (_name, values)  = get_value(sum.clone());
    let la = Series::new("linear_activations", values);
    Ok(la)
}

pub fn linear_activation_function(sum: Series) -> Series{
    // Skipped due to no need to implement REVISIT LATER
    // f(x) = x
    let (_name, values)  = get_value(sum.clone());
    let la = Series::new("linear_activations", values);
    la
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