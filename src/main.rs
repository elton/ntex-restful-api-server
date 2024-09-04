#![recursion_limit = "256"]

mod errors;
mod handlers;
mod middleware;
mod models;
mod repository;
mod utils;

use dotenvy::dotenv;
use ntex::web::{self};
use ntex_cors::Cors;
use std::sync::Arc;

pub struct AppState {
    pool: repository::database::DbPool,
    redis_client: redis::Client,
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    // enable logger
    std::env::set_var("RUST_LOG", "ntex=info,diesel=debug");
    env_logger::init();

    // load environment variables from os
    dotenv().ok();
    let app_port = (std::env::var("SERVER_PORT").ok())
        .unwrap_or("8000".to_string())
        .parse::<u16>()
        .unwrap();

    // set up database connection pool
    let pool = match repository::database::new() {
        Ok(pool) => {
            log::info!("✅ Connection to the database is successful!");
            pool
        }
        Err(e) => {
            log::error!("🔥 Error connecting to the database: {}", e);
            std::process::exit(1);
        }
    };

    // set up redis connection
    let redis_client = match repository::redis::new() {
        Ok(client) => {
            log::info!("✅ Connection to the redis is successful!");
            client
        }
        Err(e) => {
            log::error!("🔥 Error connecting to Redis: {}", e);
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
            .wrap(web::middleware::Logger::default())
            .wrap(
                Cors::new() // <- Construct CORS middleware builder
                    .finish(),
            )
            // enable default headers
            .wrap(web::middleware::DefaultHeaders::new().header("content-type", "application/json"))
            // enable Compression, A response's Content-Encoding header defaults to ContentEncoding::Auto, which performs automatic content compression negotiation based on the request's Accept-Encoding header.
            // should add "compress" feature to the Cargo.toml
            .wrap(web::middleware::Compress::default())
            .wrap(middleware::auth::Auth)
            .configure(handlers::config)
    })
    .bind(("0.0.0.0", app_port))?
    .run()
    .await
}
