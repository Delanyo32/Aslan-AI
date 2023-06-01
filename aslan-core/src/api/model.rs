use actix_web::{post, Responder, HttpResponse};

use crate::{helpers::dataparser::tokenizer, transformer::embedding::{generate_unique_tokens, train_model}};


#[post("/model")]
pub async fn model() -> impl Responder {
    tokio::spawn(async move {
        // tokenize the data
        //tokenizer().await;
        // generate embeddings based on the tokenized data
        //generate_unique_tokens().await;
        //train the model
        train_model().await;
    });
    HttpResponse::Ok().body("Aslan is creating the model")
}