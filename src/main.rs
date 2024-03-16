mod errors;
mod handlers;
mod models;
mod repository;
mod utils;

use ntex::web::{self, middleware};
use ntex_cors::Cors;
use ntex_identity::{CookieIdentityPolicy, IdentityService};

#[ntex::main]
async fn main() -> std::io::Result<()> {
    // enable logger
    std::env::set_var("RUST_LOG", "ntex=info,diesel=debug");
    env_logger::init();

    // set up database connection pool
    let pool = repository::database::new();

    let domain: String = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    // web::HttpServer can be shutdown gracefully.
    web::HttpServer::new(move || {
        web::App::new()
            // set up DB pool to be used with web::State<Pool> extractor
            .state(pool.clone())
            // enable logger
            .wrap(middleware::Logger::default())
            .wrap(IdentityService::new(
                // <- create identity middleware
                CookieIdentityPolicy::new(&[0; 32]) // <- create cookie identity policy
                    .name("pwr-auth")
                    .path("/")
                    .domain(domain.as_str())
                    .secure(false),
            ))
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
