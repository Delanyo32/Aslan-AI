use actix_web::{Responder, HttpResponse};
use actix_web::{web, App, HttpServer, middleware::Logger};
mod api;
use api::model::{model,generate_tokens};
use api::task::{init};
use api::predict::{generate, add_predict_job};

mod types;

mod db;
mod helpers;
mod transformer;


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


    let _http = HttpServer::new(move || {
        let logger = Logger::default();
        App::new()
        .wrap(sentry_actix::Sentry::new())
        .wrap(logger)
            .service(model)
            .service(generate_tokens)
            .service(init)
            .service(generate)
            .service(add_predict_job)
            .route("/", web::get().to(health))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await;

    
    Ok(())
}

async fn health() -> impl Responder {
    HttpResponse::Ok().body("Aslan is searching For Truth!")
}



