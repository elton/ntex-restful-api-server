use dotenv::dotenv;
use redis::{Client, RedisError};

pub fn new() -> Result<redis::Client, RedisError> {
    dotenv().ok();
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

    Client::open(redis_url)
}
