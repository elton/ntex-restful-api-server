use dotenv::dotenv;
use ntex::http;
use ntex::web::{self, Error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::utils;
use crate::{
    errors::AppError,
    handlers::Response,
    models::user::{self, NewUser, SearchQuery, User, UserLogin},
    utils::{jwt, jwt::Claims},
    AppState,
};

#[derive(Deserialize, Serialize)]
#[serde(untagged)] // 可以自动反序列化像 {id: 123} 或 {name: "Jason"} 这样的 JSON 数据
pub enum UserQuery {
    Id { id: i32 },
    Name { name: String },
}

#[derive(Deserialize, Serialize)]
pub struct Info {
    id: Option<i32>,
    name: Option<String>,
}

// create a new user
// #[web::post("/user")]
pub async fn create_user(
    data: web::types::State<Arc<AppState>>,
    user: web::types::Json<NewUser>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");
    let cloned_user = user.clone();

    // query the user by email to check if it already exists
    let existing_user =
        web::block(move || user::get_user_by_email(&mut conn, &user.email.as_ref().unwrap()))
            .await
            .map_err(|e| {
                log::error!("Failed to get user by email: {:?}", e);
                AppError::BadRequest(e.to_string())
            })?;

    if existing_user.is_some() {
        return Err(AppError::UserAlreadyExists(
            "User Email Already Exists".to_string(),
        ));
    }

    // the conn variable is moved into the web::block closure, so it's no longer available after the closure is executed. To use the conn variable after the closure, it needs to get another one.
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");

    let new_user = web::block(move || {
        // Obtaining a connection from the pool is also a potentially blocking operation. So, it should be called within the `web::block` closure, as well.
        user::create_user(&mut conn, cloned_user)
    })
    .await
    .map_err(|e| {
        log::error!("Failed to create new user: {:?}", e);
        AppError::BadRequest(e.to_string())
    })?;

    Ok(web::HttpResponse::Created().json(&Response::<&User> {
        status: "success".to_string(),
        message: format!(
            "User `{}` with id `{}` created successfully",
            new_user.name, new_user.id
        ),
        count: None,
        data: Some(&new_user),
    }))
}

// user login with email and password
pub async fn user_login(
    data: web::types::State<Arc<AppState>>,
    user: web::types::Json<UserLogin>,
) -> Result<web::HttpResponse, AppError> {
    // check if email and password are provided
    if (&user).email.is_empty() || (&user).password.is_empty() {
        return Err(AppError::BadRequest(
            "Email and password are required".to_string(),
        ));
    }

    // verify user by email and password from db
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");
    let user = web::block(move || user::verify_user(&mut conn, &user.email, &user.password))
        .await
        .map_err(|e| {
            log::error!("Failed to verify user: {:?}", e);
            AppError::BadRequest(e.to_string())
        })?;

    if let Some(user) = user {
        // if user is verified, generate jwt token
        let mut claims = Claims::new(&user.email, "pwr.ink");
        let access_token = jwt::generate_token(jwt::TokenType::AccessToken, &mut claims);
        let refresh_token = jwt::generate_token(jwt::TokenType::RefreshToken, &mut claims);
        let token = jwt::Token {
            access_token: access_token.unwrap(),
            refresh_token: refresh_token.unwrap(),
        };

        #[derive(Serialize)]
        struct LoginResponse<'a> {
            user: &'a User,
            token: &'a jwt::Token,
        }

        let access_claims =
            jwt::decode_token(jwt::TokenType::AccessToken, token.access_token.as_str()).unwrap();
        let refresh_claims =
            jwt::decode_token(jwt::TokenType::RefreshToken, token.refresh_token.as_str()).unwrap();

        dotenv().ok();
        let access_token_max_age =
            std::env::var("ACCESS_TOKEN_MAXAGE").expect("DATABASE_URL must be set");
        let refresh_token_max_age =
            std::env::var("REFRESH_TOKEN_MAXAGE").expect("DATABASE_URL must be set");

        // store tokens in cookies
        utils::cookie::store_cookie(
            "access_token",
            &token.access_token,
            access_token_max_age.parse::<i64>().unwrap(),
        );

        // save tokens data to redis with their expire time
        jwt::save_token_to_redis(
            &data,
            access_claims.token_id.as_str(),
            user.id as usize,
            access_token_max_age.parse::<u64>().unwrap() * 60,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to save access_token: {:?}", e);
            AppError::BadRequest(e.to_string())
        })?;
        jwt::save_token_to_redis(
            &data,
            refresh_claims.token_id.as_str(),
            user.id as usize,
            refresh_token_max_age.parse::<u64>().unwrap() * 60,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to save refresh_token: {:?}", e);
            AppError::BadRequest(e.to_string())
        })?;

        Ok(web::HttpResponse::Ok().json(&Response::<LoginResponse> {
            status: "success".to_string(),
            message: "User verified".to_string(),
            count: None,
            data: Some(LoginResponse {
                user: &user,
                token: &token,
            }),
        }))
    } else {
        // if user is not verified, return unauthorized
        Err(AppError::Unauthorized)
    }
}

/// get a user by id or name
/// extract path info from "users?id={id}&name={name}" url
/// {id} - deserializes to a i32
/// {name} -  - deserializes to a String
pub async fn get_user_by_id_or_name<T>(
    data: web::types::State<Arc<AppState>>,
    web::types::Query(info): web::types::Query<Info>,
) -> Result<web::HttpResponse, AppError>
where
    T: Serialize + Send,
{
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");

    let Info { id, name } = info;

    let user_result = if let Some(id) = id {
        web::block(move || user::get_users_by_id(&mut conn, id))
            .await
            .map_err(|e| {
                log::error!("Failed to get user: {:?}", e);
                AppError::BadRequest(e.to_string())
            })?
    } else if let Some(name) = name {
        web::block(move || user::get_users_by_name(&mut conn, &name))
            .await
            .map_err(|e| {
                log::error!("Failed to get user: {:?}", e);
                AppError::BadRequest(e.to_string())
            })?
    } else {
        return Err(AppError::BadRequest(
            "Please provide either an id or a name".to_string(),
        ));
    };

    Ok(web::HttpResponse::Ok().json(&Response::<Vec<User>> {
        status: "success".to_string(),
        message: "User found".to_string(),
        count: None,
        data: Some(user_result),
    }))
}

// search users by name or email with pagination and sorting
// #[web::post("/users/search")]
pub async fn search_users(
    data: web::types::State<Arc<AppState>>,
    query: web::types::Json<SearchQuery>,
    req: ntex::web::HttpRequest,
) -> Result<web::HttpResponse, AppError> {
    // get headers from request
    let headers = req
        .headers()
        // get the authorization header from the request, which contains the jwt token.
        // This is the same as `.get("Authorization")`
        .get(http::header::AUTHORIZATION)
        .unwrap()
        .to_str()
        .unwrap();

    let token = headers.replace("Bearer ", "");
    log::info!("token: {:?}", &token);

    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");
    let (users, count) = web::block(move || {
        user::search_users(
            &mut conn,
            &query.search_term,
            &query.sort_by,
            &query.order_by,
            query.page,
            query.page_size,
        )
    })
    .await
    .map_err(|e| {
        log::error!("Failed to search users: {:?}", e);
        AppError::BadRequest(e.to_string())
    })?;

    // map_or_else 第一个闭包参数是没有元素时的处理，第二个闭包参数是有元素时的处理
    let message = users.iter().next().map_or_else(
        || "No user found".to_string(),
        |_| match count {
            0 => "No users found".to_string(),
            1 => "1 user found".to_string(),
            _ => format!("{} users found", count),
        },
    );

    Ok(web::HttpResponse::Ok().json(&Response::<Vec<User>> {
        status: "success".to_string(),
        message,
        count: Some(count),
        data: Some(users),
    }))
}

// update a user by id
pub async fn update_user_by_id(
    data: web::types::State<Arc<AppState>>,
    user: web::types::Json<NewUser>,
) -> Result<web::HttpResponse, Error> {
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");

    let updated_user = web::block(move || match user.id {
        Some(id) => user::update_user_by_id(&mut conn, id, user.into_inner()),
        None => Err(diesel::result::Error::DeserializationError(
            "User id is required".into(),
        )),
    })
    .await
    .map_err(|e| {
        log::error!("Failed to update user by id: {:?}", e);
        web::Error::from(e)
    })?;

    Ok(web::HttpResponse::Ok().json(&Response::<&User> {
        status: "success".to_string(),
        message: format!(
            "User `{}` with id `{}` updated successfully",
            updated_user.name, updated_user.id
        ),
        count: None,
        data: Some(&updated_user),
    }))
}

// delete a user by id, soft delete by setting deleted_at
pub async fn delete_user_by_id<T>(
    data: web::types::State<Arc<AppState>>,
    web::types::Query(info): web::types::Query<Info>,
) -> Result<web::HttpResponse, Error>
where
    T: Serialize + Send,
{
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");

    let deleted_user = web::block(move || match info.id {
        Some(id) => user::delete_user_by_id(&mut conn, id),
        _ => Err(diesel::result::Error::DeserializationError(
            "User id is required".into(),
        )),
    })
    .await
    .map_err(|e| {
        log::error!("Failed to delete user by id: {:?}", e);
        web::Error::from(e)
    })?;

    Ok(web::HttpResponse::Ok().json(&Response::<&User> {
        status: "success".to_string(),
        message: format!(
            "User `{}` with id `{}` deleted successfully",
            deleted_user.name, deleted_user.id
        ),
        count: None,
        data: Some(&deleted_user),
    }))
}
