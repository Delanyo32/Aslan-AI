

// - function that loads all the data collections from the database
// - finds the differences 
// - saves the differences to the database

use log::info;

use crate::db::mongodb::MongoClient;

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