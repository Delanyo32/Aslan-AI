use actix_web::{
    post,
    web::{self, Json},
};
use apalis::{postgres::PostgresStorage, prelude::Storage};
use serde::{Deserialize, Serialize};
use log::{info, warn,error};

use crate::types::app_state::{TrainJob,Status};

#[derive(Debug, Serialize, Deserialize)]
struct DataRequest {
    symbol: String,
    start_date: DataRange,
    end_date: DataRange,
}

#[derive(Debug, Serialize, Deserialize)]
struct DataRange {
    year: i32,
    month: u32,
    day: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataResponse {
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataFileParam {
    symbol: String,
    path: String
}

#[post("/data")]
pub async fn init(body: web::Json<DataFileParam>, storage: web::Data<PostgresStorage<TrainJob>>) -> Json<DataResponse> {

    let new_job = TrainJob {
        symbol: body.symbol.clone(),
        path: body.path.clone(),
        status: Status::Pending,
    };
    let storage = &*storage.into_inner();
    let mut storage = storage.clone();
    let res = storage.push(new_job).await;

    info!("Adding modeling to queue: {}", body.symbol);  
    match res {
        Ok(()) => {
            info!("Model request added to queue");
            let response = DataResponse {
                message: "Data file added to queue".to_string(),
            };
            Json(response)
        },
        Err(e) => {
            error!("Error adding job to queue: {}", e);
            let response = DataResponse {
                message: format!("Error adding data file to queue: {}", e),
            };
            Json(response)
        },
    }   
}



