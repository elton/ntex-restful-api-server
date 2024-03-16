use std::error::Error;

use chrono::Local;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

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

const JWT_SECRET: &[u8] = b"https://pwr.ink";

#[derive(Serialize, Deserialize)]
pub struct Claims {
    iss: String, // 签发者
    sub: String, // 主题
    iat: usize,  // 签发时间
    exp: usize,  // 过期时间
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
            iss: iss.to_owned(),
            sub: sub.to_owned(),
            iat,
            exp,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
}

pub fn create_token(claims: &Claims) -> Result<String, Box<dyn Error>> {
    let jwt = encode(
        &Header::default(), // using the default Algorithm HS256
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .unwrap();

    Ok(jwt)
}

pub fn verify_token(token: &str) -> Result<Claims, Box<dyn Error>> {
    let token = token.replace("Bearer ", "");
    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

#[test]
fn test_jwt() {
    let claims = Claims::new("elton", "pwr.ink");
    let token = create_token(&claims).unwrap();
    let claims = verify_token(&token).unwrap();

    assert_eq!(claims.sub, "elton");
}
