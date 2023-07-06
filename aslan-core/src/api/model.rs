use actix_web::{post, Responder, HttpResponse};
use burn_autodiff::ADBackendDecorator;
use burn_ndarray::{NdArrayDevice, NdArrayBackend};

use crate::{helpers::dataparser::{tokenizer, generate_test_prediction}, transformer::embedding::{generate_unique_tokens, train_model}, core::training};
use crate::core;

#[post("/model")]
pub async fn model() -> impl Responder {
    tokio::spawn(async move {
        train_model().await;
    });
    HttpResponse::Ok().body("Aslan is creating the model")
}

// Generate tokens
#[post("/generateTokens")]
pub async fn generate_tokens() -> impl Responder {
    tokio::spawn(async move {
        // tokenize the data
        tokenizer().await;
        // generate embeddings based on the tokenized data
        generate_unique_tokens().await;
    });
    HttpResponse::Ok().body("Aslan is generating tokens")
}

#[post("/generateTestData")]
pub async fn generate_test_data() -> impl Responder {
    tokio::spawn(async move {
        
        generate_test_prediction().await;
    });
    HttpResponse::Ok().body("Aslan is generating test data")
}

#[post("/trainEmbeddings")]
pub async fn burn_generate() -> impl Responder {
        
    let device = NdArrayDevice::Cpu;
    training::run::<ADBackendDecorator<NdArrayBackend<f64>>>(device).await;
    
    HttpResponse::Ok().body("Aslan is training the embeddings")
}