use actix_web::web::{route, Json};
use anyhow::Result;
use actix_web::{Responder, HttpResponse, post};
use apalis::prelude::{Monitor, WorkerBuilder, WorkerFactoryFn, JobStreamExt, JobState, Storage};
use apalis::layers::{TraceLayer};
use apalis::postgres::PostgresStorage;
use actix_web::{web, App, HttpServer, middleware::Logger};
use api::job::{list_jobs, get_workers, kill_job, get_job};
use futures::future;
mod api;
use api::task::{init};
use api::predict::{generate, add_predict_job, predict_job, PredictJob};

mod types;
use types::app_state::{TrainJob,train_model, JobList};

mod db;


#[tokio::main]
async fn main()-> std::io::Result<()>{

    let _guard = sentry::init(("https://54db31e0bb8a4a1296accdf0ee495df9@o987229.ingest.sentry.io/4504302999502848", sentry::ClientOptions {
        release: sentry::release_name!(),
        ..Default::default()
    }));

    let port = std::env::var("PORT").unwrap_or("8080".to_string()).parse::<u16>().unwrap();
    std::env::set_var("RUST_LOG", "debug,sqlx::query=error");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // RabbitMQ
    let rabbitmq = db::rabbitmq::RabbitMQ::new().await;

    let http = HttpServer::new(move || {
        let logger = Logger::default();
        App::new()
        .wrap(sentry_actix::Sentry::new())
        .wrap(logger)
            .service(init)
            .service(generate)
            .service(add_predict_job)
            .route("/", web::get().to(health))
            .route("/listJobs" ,web::get().to(list_jobs))
            .route("/listWorkers" ,web::get().to(get_workers))
            .route("/killJob/{job_id}" ,web::get().to(kill_job))
            .route("/getJob/{job_id}" ,web::get().to(get_job))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await;

    
    Ok(())
}

async fn health() -> impl Responder {
    HttpResponse::Ok().body("Aslan is searching For Truth!")
}



