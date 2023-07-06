use aslan_data::DataNode;
use chrono::{DateTime, Utc};
use futures::{stream::TryStreamExt, StreamExt};
use log::{error, info, warn};
use mongodb::{
    bson::{doc, Document, self,oid::ObjectId, Bson},
    options::{ClientOptions, FindOptions, ResolverConfig, FindOneOptions},
    Client,
};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};

use crate::{types::app_state::LossBreakdown, transformer::embedding::Embedding, helpers::dataparser::TestData};

// implement copy trait for Mongo Client
impl Clone for MongoClient {
    fn clone(&self) -> Self {
        MongoClient {
            client: self.client.clone(),
        }
    }
}
#[derive(Debug)]
pub struct MongoClient {
    pub client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PredictEntry {
    pub _id: String,
    pub symbol: String,
    pub market: String,
    pub path: String,
    pub prediction: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolData {
    pub symbol: String,
    pub label: String,
    pub data: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelEntries{
    pub _id: String,
    pub symbol: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Symbol{
    pub symbol: String,
    pub name: String,
    pub class: String,
    pub exchange: String,
    pub shortable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetData {
    pub symbol: Option<String>,
    pub label: Option<String>,
    pub data: Vec<f64>,
}



impl MongoClient {
    pub async fn new() -> Self {
        let client_uri =
            env::var("MONGO_URL").expect("You must set the MONGO_URL environment var!");
        let client = Client::with_uri_str(client_uri).await.unwrap();
        MongoClient { client: client }
    }

    pub async fn get_symbol_data(&self, symbol: String, path: String, market: String) -> Vec<f64> {
        let database = self.client.database("aslan-data");
        info!("Querying for symbol: {} ", symbol);
        info!("Querying for path: {} ", path);
        info!("Querying for market: {} ", market);

        let collection_name = format!("{}_{}_DATA", symbol, market);
        let collection = database.collection::<SymbolData>(collection_name.as_str());
        info!("Querying for symbol: {} ", collection_name);

        // Query the books in the collection with a filter and an option.
        let label  = path.clone().to_lowercase();
        let filter = doc! { "label": label };
        let document = collection.find_one(filter, None).await.unwrap();
        
        match document {
            Some(document) => {
                info!("Found symbol data for {} and path {}", symbol, path);
                document.data
            }
            None => {
                error!("No symbol data found for {} and path {}", symbol, path);
                Vec::new()
            }
        }
    }

    pub async fn export_data(&self, symbol: String, data: Vec<DataNode>, label: String, market: String) {
        let database = self.client.database("aslan-model");
        let collection_name = format!("{}_{}_{}_MODEL", symbol,label,market);
        let collection = database.collection(&collection_name);
        let now: DateTime<Utc> = Utc::now();
        let mut nodes = Vec::new();
        for node in data {
            let mut edges = Vec::new();
            for edge in node.edges {
                let edge = doc! { 
                    "score": edge.score,
                    "weight": edge.weight,
                    "value": edge.value,
                 };
                edges.push(edge);
            };
            
            let node = doc! { 
                "symbol": &symbol,
                "label": &label,
                "timestamp": now.to_rfc3339(),
                "average": node.average,
                "members" : node.members,
                "edges" : edges
             };
            nodes.push(node);
        }
       
        collection.insert_many(nodes, None).await.unwrap();
    }

    pub async fn save_loss_breakdown (&self, symbol: String, loss_breakdown: LossBreakdown, label: String){
        let database = self.client.database("aslan-meta");
        let collection_name = format!("LOSS_BREAKDOWN");
        let collection = database.collection(collection_name.as_str());
        let now: DateTime<Utc> = Utc::now();
        let mut loss_items = Vec::new();
        for loss_item in loss_breakdown.loss_items {
            let loss_item = doc! { 
                "iteration": loss_item.iteration as i32,
                "loss": loss_item.loss,
             };
            loss_items.push(loss_item);
        };
        let id = format!("{}_{}_{}", symbol, label, loss_breakdown.stage);
        let loss_breakdown = doc! { 
            "_id": id,
            "symbol": &symbol,
            "label": &label,
            "stage": &loss_breakdown.stage,
            "timestamp": now.to_rfc3339(),
            "average_loss": loss_breakdown.average_loss,
            "average_price": loss_breakdown.average_price,
            "price_to_loss_ratio": loss_breakdown.price_to_loss_ratio,
            "loss_items" : loss_items
         };
        collection.insert_one(loss_breakdown, None).await.unwrap();
    }

    pub async fn get_node_data(&self, symbol: &str) -> Vec<DataNode> {
        return Vec::new();
    }

    //function to check if given a symbol and a path, if the data exists and return it if it does
    pub async fn get_model_metadata(&self, symbol: String, path: String) -> Option<ModelEntries> {
        let database = self.client.database("aslan-meta");
        let collection_name = format!("{}_MODELS", symbol);
        let collection = database.collection::<ModelEntries>(collection_name.as_str());
        info!("Querying for symbol: {} and path: {}", symbol, path);

        // Query the books in the collection with a filter and an option.
        let filter = doc! {"symbol": &symbol, "label": &path };
        let find_options = FindOneOptions::builder().build();
        let entry = collection.find_one(filter, find_options).await.unwrap();
        entry
    }

    // function to add model entry to the database
    pub async fn add_model_entry(&self, symbol: String, path: String, market: String) {
        let database = self.client.database("aslan-meta");
        let collection_name = format!("{}_MODELS", symbol);
        let collection = database.collection::<ModelEntries>(collection_name.as_str());
        let id = format!("{}_{}_MODEL", symbol, path);
        let entry = ModelEntries {
            _id: id,
            symbol: symbol,
            path: path,
        };
        collection.insert_one(entry, None).await.unwrap();
    }

    // function to find if a model entry exists
    pub async fn find_model_entry(&self, symbol: String, path: String) -> bool {
        let database = self.client.database("aslan-meta");
        let collection_name = format!("{}_MODELS", symbol);
        let collection = database.collection::<ModelEntries>(collection_name.as_str());
        let filter = doc! {"symbol": &symbol, "path": &path };
        let find_options = FindOneOptions::builder().build();
        let entry = collection.find_one(filter, find_options).await.unwrap();
        match entry {
            Some(_) => {
                info!("Found model entry for {} and path {}", symbol, path);
                true
            },
            None => {
                info!("No model entry found for {} and path {}", symbol, path);
                false
            },
        }
    }

    pub async fn find_node(&self, symbol: String, label: String, market:String ,parameter: f64) -> Vec<DataNode> {
        let database = self.client.database("aslan-model");
        let collection_name = format!("{}_{}_{}_MODEL", symbol,label,market);
        let collection = database.collection::<DataNode>(&collection_name);

        let add_feild_stage = doc!{"$addFields": { "distance": {"$abs":{"$subtract": ["$average" ,parameter ]}  } }};
        let sort_stage = doc!{"$sort": { "distance": 1 }};
        let limit_stage = doc!{"$limit": 1};
        let pipeline = vec![add_feild_stage, sort_stage, limit_stage];
        let mut cursor = collection.aggregate(pipeline, None).await.unwrap();
        
        let mut nodes = Vec::new();
        while let Some(data) = cursor.try_next().await.unwrap() {
            let data_node = bson::from_document::<DataNode>(data).unwrap();
            nodes.push(data_node);
        }

        nodes
        
    }

    pub async fn load_model(&self, symbol: String, label: String, market:String) -> Vec<DataNode> {
        let database = self.client.database("aslan-model");
        let collection_name = format!("{}_{}_{}_MODEL", symbol,label,market);
        let collection = database.collection::<DataNode>(&collection_name);

        let mut cursor = collection.find(None, None).await.unwrap();

        let mut nodes = Vec::new();
        while let Some(data) = cursor.try_next().await.unwrap() {
            nodes.push(data);
        }
        return nodes;
    }

    pub async fn get_symbols(&self,market: String) -> Vec<String> {
        let database = self.client.database("aslan-meta");
        let collection_name = format!("symbols_{}", market);
        let collection = database.collection::<Symbol>(&collection_name);

        let mut cursor = collection.find(None, None).await.unwrap();

        let mut symbols = Vec::new();
        while let Some(data) = cursor.try_next().await.unwrap() {
            symbols.push(data.symbol);
        }
        return symbols;
    }

    pub async fn insert_prediction(&self, id: String, symbol: String, market: String, path: String, prediction: Vec<f64>) {
        let database = self.client.database("aslan-predictions");
        let collection_name = format!("predictions_{}", market);
        let collection = database.collection::<PredictEntry>(collection_name.as_str());
        let entry = PredictEntry {
            _id: id,
            symbol: symbol,
            market: market,
            path: path,
            prediction: prediction,
        };
        collection.insert_one(entry, None).await.unwrap();
    }

    // function to list all collections in mongodb
    pub async fn list_collections(&self, database_name: String) -> Vec<String> {
        let database = self.client.database(database_name.as_str());
        let cursor = database.list_collection_names(None).await.unwrap();
        return cursor;
    }

    // given a collection name, return the documents in the collection
    pub async fn get_collection(&self, collection_name: String, database_name: String) -> Vec<Vec<f64>> {
        let database = self.client.database(database_name.as_str());
        let collection = database.collection::<AssetData>(&collection_name);
        let mut cursor = collection.find(None, None).await.unwrap();
        let mut documents = Vec::new();
        while let Some(document) = cursor.try_next().await.unwrap() {
            documents.push(document.data);
        }
        return documents;
    }

    // given a vec of floats and a collection name, insert the data into the collection
    pub async fn insert_tokens(&self, collection_name: String, data: Vec<f64>) {
        let database = self.client.database("aslan-tokens");
        let collection = database.collection::<Document>(&collection_name);
        let entry = doc!  {
            "_id": collection_name.clone(),
            "data": data,
        };
        collection.insert_one(entry, None).await.unwrap();
    }

    // given database and collection name check if a collection exists
    pub async fn check_collection(&self, collection_name: String, database: String) -> bool {
        let database = self.client.database(database.as_str());
        let filter = doc! {"name": collection_name};
        let cursor = database.list_collection_names(filter).await.unwrap();
        if cursor.len() == 0 {
            return false;
        } else {
            return true;
        }
    }

    pub async fn insert_embeddings(&self, embedings: Vec<Embedding>){
        let database = self.client.database("aslan-embeddings");
        let collection = database.collection::<Document>(&"embeddings");
        let mut entries = Vec::new();
        for embedding in embedings {
            let entry = doc!  {
                "_id": embedding.token.to_string(),
                "token": embedding.token,
                "input_layer": embedding.input_layer,
                "output_layer": embedding.output_layer,
            };
            entries.push(entry);
        }
        collection.insert_many(entries, None).await.unwrap();
    }

    pub async fn get_embeddings(&self) -> Vec<Embedding> {
        let database = self.client.database("aslan-embeddings");
        let collection = database.collection::<Embedding>(&"embeddings");
        let mut cursor = collection.find(None, None).await.unwrap();
        let mut embeddings = Vec::new();
        while let Some(embedding) = cursor.try_next().await.unwrap() {
            embeddings.push(embedding);
        }
        return embeddings;
    }


    pub async fn insert_test_data(&self, data: Vec<TestData>, database: String, collection: String){
        let database = self.client.database(&database);
        let collection = database.collection::<Document>(&collection);
        let mut entries = Vec::new();
        for test_data in data {
            let entry = doc!  {
                "input_data": test_data.input_data,
                "output_data": test_data.output_data,
            };
            entries.push(entry);
        }
        collection.insert_many(entries, None).await.unwrap();
    }

    pub async fn get_test_data(&self,database: String, collection: String) -> Vec<TestData> {
        let database = self.client.database(&database);
        let collection = database.collection::<TestData>(&collection);
        let mut cursor = collection.find(None, None).await.unwrap();
        let mut test_data = Vec::new();
        while let Some(data) = cursor.try_next().await.unwrap() {
            test_data.push(data);
        }
        return test_data;
    }
}


