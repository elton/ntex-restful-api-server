use std::sync::Arc;

use chrono::Local;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey};

use ntex::web::types::State;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde::{Deserialize, Serialize};

use base64::{engine::general_purpose, Engine as _};
use dotenv::dotenv;
use ulid::Ulid;

use crate::{models::user, AppState};

// 快速说明
//
// - 获取一个授权令牌:
//
// curl -X POST -H 'content-type:application/json' -d '{"client_id":"axum.rs","client_secret":"team@axum.rs"}' 127.0.0.1:9527/authorize
//
// - 使用授权令牌访问被保护的内容
//
// curl -H 'content-type:application/json' -H 'Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZWFtQGF4dW0ucnMiLCJjb21wYW55IjoiQVhVTS5SUyIsImV4cCI6MTAwMDAwMDAwMDB9.2jPYCuK6_nDrFdXS3HLAm43YvbFvrBBLYS6YkZ_z6zM' 127.0.0.1:9527/protected
//
// - 尝试使用非法的令牌使用授权令牌访问被保护的内容
//
// curl -H 'content-type:application/json' -H 'Authorization: Bearer foobar' 127.0.0.1:9527/protected

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub token_id: String, // token ID
    pub iss: String,      // 签发者
    pub sub: String,      // 主题
    pub iat: usize,       // 签发时间
    pub exp: usize,       // 过期时间
}

pub enum TokenType {
    AccessToken,
    RefreshToken,
}

impl Claims {
    pub fn new(sub: &str, iss: &str) -> Self {
        let now = Local::now();
        let iat: usize = now.timestamp().try_into().unwrap();
        let exp: usize = (now + chrono::Duration::hours(1))
            .timestamp()
            .try_into()
            .unwrap();

        Self {
            token_id: Ulid::new().to_string(),
            iss: iss.to_owned(),
            sub: sub.to_owned(),
            iat,
            exp,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
}

pub fn generate_token(
    kind: TokenType,
    claims: &Claims,
) -> Result<String, jsonwebtoken::errors::Error> {
    dotenv().ok();
    let private_key = match kind {
        TokenType::AccessToken => {
            std::env::var("ACCESS_TOKEN_PRIVATE_KEY").expect("ACCESS_TOKEN_PRIVATE_KEY must be set")
        }
        TokenType::RefreshToken => std::env::var("REFRESH_TOKEN_PRIVATE_KEY")
            .expect("REFRESH_TOKEN_PRIVATE_KEY must be set"),
    };
    let bytes_private_key = general_purpose::STANDARD.decode(private_key).unwrap();
    let decoded_private_key = String::from_utf8(bytes_private_key).unwrap();

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);

    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(decoded_private_key.as_bytes())?,
    )?;

    Ok(token)
}

pub fn decode_token(kind: TokenType, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    dotenv().ok();

    let public_key =
        match kind {
            TokenType::AccessToken => std::env::var("ACCESS_TOKEN_PUBLIC_KEY")
                .expect("ACCESS_TOKEN_PUBLIC_KEY must be set"),
            TokenType::RefreshToken => std::env::var("REFRESH_TOKEN_PUBLIC_KEY")
                .expect("REFRESH_TOKEN_PUBLIC_KEY must be set"),
        };

    let bytes_public_key = general_purpose::STANDARD.decode(public_key).unwrap();
    let decoded_public_key = String::from_utf8(bytes_public_key).unwrap();

    let validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);

    let token = token.replace("Bearer ", "");
    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_rsa_pem(decoded_public_key.as_bytes())?,
        &validation,
    )?;

    Ok(token_data.claims)
}

// save jwt to redis
pub async fn save_token_to_redis(
    data: &State<Arc<AppState>>,
    token_id: &str,
    user_id: usize,
    max_age: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut redis_client = data.redis_client.get_multiplexed_async_connection().await?;

    redis_client.set_ex(token_id, user_id, max_age).await?;
    Ok(())
}

/// get user_id from redis by jwt token
/// kind is the type of token, it can be AccessToken or RefreshToken
/// token is the jwt token

pub async fn get_user_id_from_redis(
    conn: &mut MultiplexedConnection,
    kind: TokenType,
    token: &str,
) -> Result<Option<usize>, Box<dyn std::error::Error>> {
    let token = token.replace("Bearer ", "");
    // decode token and get user_id from redis
    if let Ok(claims) = decode_token(kind, token.as_str()) {
        log::info!("claims in get_user_id_from_redis: {:?}", claims);
        let user_id = conn.get(claims.token_id).await.map_err(|e| {
            log::error!("Invalid token, no record in Redis: {}", e);
            e
        })?;
        Ok(Some(user_id))
    } else {
        Err("Invalid token".into())
    }
}

/// refresh token
pub async fn refresh_token(
    data: &State<Arc<AppState>>,
    refresh_token: &str,
) -> Result<Token, Box<dyn std::error::Error>> {
    log::info!("refresh token in fn refresh_token: {}", refresh_token);
    let mut conn = data.redis_client.get_multiplexed_async_connection().await?;
    // decode refresh token and get user_id from redis
    if let Some(user_id) = get_user_id_from_redis(&mut conn, TokenType::RefreshToken, refresh_token)
        .await
        .map_err(|e| {
            log::error!("Invalid refresh token: {}", e);
            e
        })?
    {
        // get user name from postgresql database
        let mut conn = data
            .pool
            .get()
            .expect("couldn't get db connection from pool");
        let user = user::get_users_by_id(&mut conn, user_id as i32).map_err(|e| {
            log::error!("Error getting user from db: {}", e);
            e
        })?;

        if user.is_empty() {
            return Err("Invalid refresh token".into());
        }

        let access_claims = Claims::new(&user[0].name, "pwr.ink");
        let access_token = generate_token(TokenType::AccessToken, &access_claims)?;

        let refresh_claims = Claims::new(&user[0].name, "pwr.ink");
        let refresh_token = generate_token(TokenType::RefreshToken, &refresh_claims)?;

        dotenv().ok();
        let access_token_max_age =
            std::env::var("ACCESS_TOKEN_MAXAGE").expect("DATABASE_URL must be set");
        let refresh_token_max_age =
            std::env::var("REFRESH_TOKEN_MAXAGE").expect("DATABASE_URL must be set");

        // save new tokens to redis
        save_token_to_redis(
            &data,
            access_claims.token_id.as_str(),
            user_id as usize,
            access_token_max_age.parse::<u64>().unwrap() * 60,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to save access_token: {:?}", e);
            e
        })?;
        save_token_to_redis(
            &data,
            refresh_claims.token_id.as_str(),
            user_id as usize,
            refresh_token_max_age.parse::<u64>().unwrap() * 60,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to save refresh_token: {:?}", e);
            e
        })?;

        Ok(Token {
            access_token,
            refresh_token,
        })
    } else {
        Err("Invalid token".into())
    }
}

#[cfg(test)]
#[test]
fn test_jwt() {
    let mut claims = Claims::new("elton", "pwr.ink");
    let token = generate_token(TokenType::AccessToken, &mut claims).unwrap();
    println!("access token: {}", token);
    let claims = decode_token(TokenType::AccessToken, &token).unwrap();
    println!("claims: {:?}", claims);

    assert_eq!(claims.sub, "elton");

    let mut claims = Claims::new("elton", "refresh_claims");
    let token = generate_token(TokenType::RefreshToken, &mut claims).unwrap();
    println!("refresh token: {}", token);
    let claims = decode_token(TokenType::RefreshToken, &token).unwrap();
    println!("claims: {:?}", claims);

    assert_eq!(claims.iss, "refresh_claims");
}

#[test]
fn test_decode_token() {
    let refresh_token ="eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJ0b2tlbl9pZCI6IjAxSFNKQVJLWERBSDIzWjhTRjZaWTQ3NVRWIiwiaXNzIjoicHdyLmluayIsInN1YiI6IkVsdG9uIFpoZW5nIiwiaWF0IjoxNzExMDg1OTk3LCJleHAiOjE3MTEwODk1OTd9.dNaWF0NQfqyz58MW_YJEI5f3CFQNhh-jAKXCXht7IDUrXvKOfDezJoBpWUR7vF8bcWV--Ak1KSkemaqSVYuRJX9TBWzXTS_y5_42Q2F_HdJ8RZPycEDDQrCjHY-koGgSHMwRdyqDhzA1Z5ErtVdShxI5RVQd2WYjBPKlSE_dhpcWBJpI7x3VqrHh2QsDifebtfvVNAgzHKQe03YNdQc4EI6Xfeq8zVnOUak7Een5wdiRE_LdKaCYF7mgCIESS6pV8C-drCr6kpC8ovONIhESEy5G8FvjB-xH8vPegg7P0V54gT0-Bb0JoNlxGT0FHHZmQvPPHRS7WC7kGAP-xKp00PkRAbk9MPmdHzU7clmSLKByqBazfPh9jOiyIyNmOIpiqNKDPCqkxpKCbyZMlAr7dGz1odDDsKNJuJDhnayB7fujkEIkIW6AmWxs_A8eslJvFmXRU3zBNdQyPDKQUvId2v6b70OkndOpEn3_PUg8N0T20zkYKDGGf_CG_rdPAU4_1oZAHShArIb5AfZvlyujEIOHfiOdw9Q_CgGyiEefe8ORFX44uj-1SE08DDpEpBQk3_kZEnmfCqqgMFkb931bp9gMo0Wib2JBIxY1_saOywrTCRUFhARavW6cifgj_rK9w4SMHSUT-sFH25keKEVLi5wKu8KvdruTSW3bUx4kls8";
    let claims = decode_token(TokenType::RefreshToken, refresh_token).unwrap();
    println!("claims: {:?}", claims);
    assert_eq!(claims.sub, "Elton Zheng");
}
