use std::collections::{hash_map, HashMap};

//find the trend of the data
//iterate through the options
//randomly select a result
//keep doing that for a number of times
//average out the results
use rand::prelude::*;
use serde::{Deserialize, Serialize};
#[derive(Debug)]
pub struct Bootstrap{
    iterations: usize,
    data: Vec<Vec<f64>>,
}
#[derive(Debug,Serialize, Deserialize)]
pub struct BootstrapResult{
    pub generated_data: Vec<f64>,
    pub average_data: Vec<f64>,
}

impl BootstrapResult {
    pub fn new(generated: Vec<f64>,average_data:Vec<f64>) -> Self {
        
        BootstrapResult{
            generated_data: generated,
            average_data: average_data,
        }
    }
}

impl Bootstrap {
    pub fn new(iterations:usize,data:Vec<Vec<f64>>)->Self{
        //copy test data
        Bootstrap{
            iterations,
            data,
        }
    }

    pub fn denormalize(seed_data:f64,data:Vec<f64>)->Vec<f64>{
        let mut generated_data = Vec::new();
        generated_data.push(seed_data);
        for i in 1..data.len(){
            generated_data.push(generated_data[i-1] + data[i]);
        }
        generated_data
    }

    pub fn select_top_results(data: &Vec<Vec<f64>>, test_data:&Vec<f64>,number_to_select:usize)->HashMap<String, Vec<f64>>{
        let data = Bootstrap::calculate_std_dev(data,test_data);
        let keys = data.keys().collect::<Vec<&String>>();
        let mut keys_f64 = keys.iter().map(|x| x.parse::<f64>().unwrap()).collect::<Vec<f64>>();
        keys_f64.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut top_results = HashMap::new();
        for key in keys_f64.iter().take(number_to_select){
            top_results.insert(key.to_string(),data.get(&key.to_string()).unwrap().to_vec());
        }
        top_results
    }

    pub fn calculate_std_dev(data: &Vec<Vec<f64>>, test_data:&Vec<f64>)->HashMap<String, Vec<f64>>{
        let mut result  = HashMap::new();
    
        for row in data.iter(){
            let mut generated_data = Bootstrap::denormalize(test_data[0],row.to_vec());
            let std_dev = generated_data.iter().zip(test_data.iter()).map(|(a,b)| (a - b).powi(2)).sum::<f64>().sqrt();
            result.insert(std_dev.to_string(),generated_data.to_vec());
        }
        result
    }

    pub fn run(&self, seed_data:f64, slot_size:usize) -> BootstrapResult {
        let slots = slot_size;
        let mut current_data = seed_data;
        let mut generated_data = Vec::new();
        generated_data.push(current_data);
        let mut average_data = Vec::new();

        for selected_slot in 0..slots{
            let mut slot_choices = Vec::new();
            for _ in 0..self.iterations{
                //randomly generate index to select data from
                let mut rng = rand::thread_rng();
                let random_index = rng.gen_range(0..self.data.len());
                let data_slot = self.data[random_index][selected_slot];
                slot_choices.push(data_slot);
            }
            let average = slot_choices.iter().sum::<f64>() / slot_choices.len() as f64;
            let average = (average * 100.0).round() / 100.0;
            average_data.push(average);
            current_data = current_data + average;
            generated_data.push(current_data);
        }
        let result = BootstrapResult::new(generated_data,average_data);
        result
    }
}