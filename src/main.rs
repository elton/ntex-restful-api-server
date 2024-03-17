mod errors;
mod handlers;
mod models;
mod repository;
mod utils;

use std::sync::Arc;

use ntex::web::{self, middleware};
use ntex_cors::Cors;

pub struct AppState {
    pool: repository::database::DbPool,
    redis_client: redis::Client,
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    // enable logger
    std::env::set_var("RUST_LOG", "ntex=info,diesel=debug");
    env_logger::init();

    // set up database connection pool
    let pool = repository::database::new();

    // set up redis connection
    let redis_client = match repository::redis::new().await {
        Ok(client) => {
            println!("✅ Connection to the redis is successful!");
            client
        }
        Err(e) => {
            println!("🔥 Error connecting to Redis: {}", e);
            std::process::exit(1);
        }
    };

    // web::HttpServer can be shutdown gracefully.
    web::HttpServer::new(move || {
        web::App::new()
            // set up DB pool to be used with web::State<Pool> extractor
            .state(Arc::new(AppState {
                pool: pool.clone(),
                redis_client: redis_client.clone(),
            }))
            // enable logger
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::new() // <- Construct CORS middleware builder
                    .finish(),
            )
            // enable default headers
            .wrap(web::middleware::DefaultHeaders::new().header("content-type", "application/json"))
            // enable Compression, A response's Content-Encoding header defaults to ContentEncoding::Auto, which performs automatic content compression negotiation based on the request's Accept-Encoding header.
            // should add "compress" feature to the Cargo.toml
            .wrap(web::middleware::Compress::default())
            .configure(handlers::config)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
