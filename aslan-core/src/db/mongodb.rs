use aslan_data::DataNode;
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use log::{error, info, warn};
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, FindOptions, ResolverConfig},
    Client,
};
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};

use crate::types::app_state::LossBreakdown;

#[derive(Debug)]
pub struct MongoClient {
    pub client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolData {
    pub symbol: String,
    pub label: String,
    pub data: Vec<f64>,
}

impl MongoClient {
    pub async fn new() -> Self {
        let client_uri =
            env::var("MONGO_URL").expect("You must set the MONGO_URL environment var!");
        let client = Client::with_uri_str(client_uri).await.unwrap();
        MongoClient { client: client }
    }

    pub async fn get_symbol_data(&self, symbol: String, path: String) -> Vec<f64> {
        let database = self.client.database("aslan-data");
        let collection_name = format!("{}_data", symbol);
        let collection = database.collection::<SymbolData>(collection_name.as_str());
        info!("Querying for symbol: {} and path: {}", symbol, path);

        // Query the books in the collection with a filter and an option.
        let mut data = Vec::new();
        let filter = doc! {"label": &path };
        let find_options = FindOptions::builder().build();
        let mut cursor = collection.find(filter, find_options).await.unwrap();
        info!("Displaying results");
        // Iterate over the results of the cursor.
        while let Some(entry) = cursor.try_next().await.unwrap() {
            data = entry.data;
        }
        if data.len() == 0 {
            error!("No data found for symbol: {} and path: {}", symbol, path);
        }
        data
    }

    pub async fn export_data(&self, symbol: String, data: Vec<DataNode>, label: String) {
        let database = self.client.database("aslan-data");
        let collection_name = format!("{}_model", symbol);
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
        let database = self.client.database("aslan-data");
        let collection = database.collection("loss-breakdown");
        let now: DateTime<Utc> = Utc::now();
        let mut loss_items = Vec::new();
        for loss_item in loss_breakdown.loss_items {
            let loss_item = doc! { 
                "iteration": loss_item.iteration as i32,
                "loss": loss_item.loss,
             };
            loss_items.push(loss_item);
        };
        let loss_breakdown = doc! { 
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
}


