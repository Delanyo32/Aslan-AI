use lapin::{ConnectionProperties, Connection, options::{QueueDeclareOptions, BasicConsumeOptions, BasicPublishOptions, BasicAckOptions}, types::FieldTable, BasicProperties, Channel, message::DeliveryResult};
use log::{info, error};
use serde::{Serialize, Deserialize};

use crate::{types::app_state, api::predict, db::mongodb::MongoClient};

pub struct RabbitMQ{
    pub channel: Channel,
    pub model_queue: String,
    pub prediction_queue: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelParameter {
    symbol: String,
    market: String,
    path: String
}


// implement copy trait for RabbitMQ
impl Clone for RabbitMQ {
    fn clone(&self) -> Self {
        RabbitMQ {
            channel: self.channel.clone(),
            model_queue: self.model_queue.clone(),
            prediction_queue: self.prediction_queue.clone(),
        }
    }
}


impl RabbitMQ {
    pub async fn new()-> Self{
        let uri = std::env::var("AMQP_ADDR").expect("You must set the AMQP_ADDR environment var!");
        let options = ConnectionProperties::default()
            .with_executor(tokio_executor_trait::Tokio::current())
            .with_reactor(tokio_reactor_trait::Tokio);
    
        let connection = Connection::connect(&uri, options).await.unwrap();
        let channel = connection.create_channel().await.unwrap();

        let prediction_queue = "prediction_queue".to_string();
        let model_queue = "model_queue".to_string();

        let _model_queue = channel
        .queue_declare(
            &model_queue,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

        let _predict_queue = channel
        .queue_declare(
            &prediction_queue,
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    let model_consumer = channel
        .basic_consume(
            &model_queue,
            "model_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

        model_consumer.set_delegate(move |delivery: DeliveryResult| async move {
        let delivery = match delivery {
            // Carries the delivery alongside its channel
            Ok(Some(delivery)) => delivery,
            // The consumer got canceled
            Ok(None) => return,
            // Carries the error and is always followed by Ok(None)
            Err(error) => {
                dbg!("Failed to consume queue message {}", error);
                return;
            }
        };

        // TODO FIX error behavior
        // convert message from bytes to struct
        let model_parameter: ModelParameter = serde_json::from_slice(&delivery.data).unwrap();
        info!("Received message for model consumer: {}", model_parameter.symbol);
        info!("Building Model");
        app_state::build_model(model_parameter.symbol, model_parameter.path, model_parameter.market).await;
        info!("Model Built");

        delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("Failed to ack send_webhook_event message");

    });



    let predict_consumer = channel
    .basic_consume(
        &prediction_queue,
        "predict_consumer",
        BasicConsumeOptions::default(),
        FieldTable::default(),
    )
    .await
    .unwrap();

    predict_consumer.set_delegate(move |delivery: DeliveryResult| async move {
    let delivery = match delivery {
        // Carries the delivery alongside its channel
        Ok(Some(delivery)) => delivery,
        // The consumer got canceled
        Ok(None) => return,
        // Carries the error and is always followed by Ok(None)
        Err(error) => {
            dbg!("Failed to consume queue message {}", error);
            return;
        }
    };

    // convert message from bytes to struct
    let predict_parameter: predict::PredictParameters = serde_json::from_slice(&delivery.data).unwrap();
    info!("Received message for predict consumer: {}", predict_parameter.symbol);


    delivery
    .ack(BasicAckOptions::default())
    .await
    .expect("Failed to ack send_webhook_event message");

    let id = predict_parameter.id.clone();
    let symbol = predict_parameter.symbol.clone();
    let market = predict_parameter.market.clone();
    let path = predict_parameter.path.clone();

    //TODO FIX error behavior
    let prediction = predict::generate_results(predict_parameter).await;

    info!("Saving Prediction to MongoDB");
    let mongodb = MongoClient::new().await;
    mongodb.insert_prediction(id, symbol, market, path, prediction).await;
    info!("Saving Prediction to MongoDB completed");
});

        RabbitMQ{
            channel: channel,
            model_queue: model_queue.clone(),
            prediction_queue: prediction_queue.clone(),
        }
    }

    pub async fn send_model_message (self, message: String){
        let publish = self.channel
        .basic_publish(
            "",
            &self.model_queue,
            BasicPublishOptions::default(),
            message.as_bytes(),
            BasicProperties::default(),
        );

        let _publish_confirm = match publish.await{
            Ok(confirm) => {
                info!("Message sent to model queue");
                let _confirmation = match confirm.await{
                    Ok(_) => info!("Message confirmed from model queue"),
                    Err(e) => error!("Error confirming message: {}", e),
                };
            },
            Err(e) => {
                error!("Error sending message: {}", e);
            },
        };
    }

    pub async fn send_predicition_message (self, message: String){
        let publish = self.channel
        .basic_publish(
            "",
            &self.model_queue,
            BasicPublishOptions::default(),
            message.as_bytes(),
            BasicProperties::default(),
        );

        let _publish_confirm = match publish.await{
            Ok(confirm) => {
                info!("Message sent to prediction queue");
                let _confirmation = match confirm.await{
                    Ok(_) => info!("Message confirmed from prediction queue"),
                    Err(e) => error!("Error confirming message: {}", e),
                };
            },
            Err(e) => {
                error!("Error sending message: {}", e);
            },
        };
    }
}