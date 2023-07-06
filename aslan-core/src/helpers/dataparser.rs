

// - function that loads all the data collections from the database
// - finds the differences 
// - saves the differences to the database

use log::info;
use serde::{Serialize, Deserialize};

use crate::db::mongodb::MongoClient;
use rand::Rng;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestData {
    pub input_data: f64,
    pub output_data: f64,
}

pub async fn tokenizer() {
    let mongo_client = MongoClient::new().await;
    let collections = mongo_client.list_collections("aslan-data".to_string()).await;

    for collection in collections {
        // print collection
        info!("Collection: {}", collection);

        // check if collection exists in tokens collection
        let collection_found = mongo_client.check_collection(collection.clone(), "aslan-tokens".to_string()).await; 
        if collection_found {
            info!("Collection already tokenized");
            continue;
        }

        // for each collection, get the data
        let entries = mongo_client.get_collection(collection.clone(), "aslan-data".to_string()).await;
        let mut differences = Vec::new();
        for entry in entries {
            let vec_data = entry;
            info!("Normalizing data");
            let mut normalized_data = aslan_data::AslanDataChunks::normalize_data(&vec_data);
            differences.append(&mut normalized_data);
        }
        // save normalized data to database
        info!("Saving normalized data to database");
        mongo_client.insert_tokens(collection.clone(), differences).await;
    }
    info!("Tokenization complete");

    
}

pub async fn generate_test_prediction() {
    info!("Generating test data");
    let mongo_client = MongoClient::new().await;
    let collections = mongo_client.list_collections("aslan-tokens".to_string()).await;

    let mut test_data_set = Vec::new();
    let mut validation_data_set = Vec::new();

    for collection in collections {

        let entries = mongo_client.get_collection(collection.clone(), "aslan-tokens".to_string()).await;

        for entry in entries{
            let mut input_data = entry.clone();
            let mut output_data = input_data.clone();
            input_data.remove(input_data.len() - 1);
            output_data.remove(0);

            for (index, input) in input_data.iter().enumerate() {
                let output = output_data[index];
                let test_data = TestData {
                    input_data: *input,
                    output_data: output,
                };

                if index % 16 == 0 {
                    validation_data_set.push(test_data);
                }else {
                    test_data_set.push(test_data);
                }
            }
        }     
        break;   
    }
    info!("Saving Test data to database");
    mongo_client.insert_test_data(test_data_set, "test-dataset".to_owned(), "test-data".to_owned()).await;
    info!("Saving Validation data to database");
    mongo_client.insert_test_data(validation_data_set, "validation-dataset".to_owned(), "validation-data".to_owned()).await;
    info!("Test data saved to database");
}