use ntex::web;
use ntex_cors::Cors;

pub mod errors;
pub mod handlers;
pub mod models;
pub mod repository;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    // enable logger
    std::env::set_var("RUST_LOG", "ntex=info,diesel=debug");
    env_logger::init();

    // set up database connection pool
    let pool = repository::database::new();
    // web::HttpServer can be shutdown gracefully.
    web::HttpServer::new(move || {
        let logger = web::middleware::Logger::default();

        web::App::new()
            .wrap(
                Cors::new() // <- Construct CORS middleware builder
                    .finish(),
            )
            // set up DB pool to be used with web::State<Pool> extractor
            .state(pool.clone())
            // enable logger
            .wrap(logger)
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
