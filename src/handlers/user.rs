use std::sync::Arc;

use ntex::web::{self, Error};
use serde::Serialize;

use crate::{
    errors::AppError,
    handlers::Response,
    models::user::{self, NewUser, SearchQuery, User, UserLogin},
    utils::{jwt, jwt::Claims},
    AppState,
};

// create a new user
#[web::post("/user")]
async fn create_user(
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
#[web::post("/user/login")]
async fn user_login(
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
        let claims = Claims::new(&user.email, "pwr.ink");
        let access_token = jwt::generate_token(&claims);
        let refresh_token = jwt::generate_token(&claims);
        let token = jwt::Token {
            access_token: access_token.unwrap(),
            refresh_token: refresh_token.unwrap(),
        };

        #[derive(Serialize)]
        struct LoginResponse<'a> {
            user: &'a User,
            token: &'a jwt::Token,
        }

        // write token to redis

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

// get a user by id
#[web::get("/user/{id}")]
async fn get_user_by_id(
    data: web::types::State<Arc<AppState>>,
    user_id: web::types::Path<i32>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");
    let user = web::block(move || user::get_user_by_id(&mut conn, user_id.into_inner()))
        .await
        .map_err(|e| {
            log::error!("Failed to get user by id: {:?}", e);
            AppError::BadRequest(e.to_string())
        })?;

    match user {
        Some(user) => Ok(web::HttpResponse::Ok().json(&Response::<&User> {
            status: "success".to_string(),
            message: "User found".to_string(),
            count: None,
            data: Some(&user),
        })),
        None => Err(AppError::Unauthorized),
    }
}

// get users by name ignore case
#[web::get("/users/{name}")]
async fn get_users_by_name(
    data: web::types::State<Arc<AppState>>,
    user_name: web::types::Path<String>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");
    let users = web::block(move || user::get_users_by_name(&mut conn, &user_name.into_inner()))
        .await
        .map_err(|e| {
            log::error!("Failed to get users by name: {:?}", e);
            AppError::BadRequest(e.to_string())
        })?;

    Ok(web::HttpResponse::Ok().json(&Response::<Vec<User>> {
        status: "success".to_string(),
        message: "Users found".to_string(),
        count: None,
        data: Some(users),
    }))
}

// search users by name or email with pagination and sorting
#[web::post("/users/search")]
async fn search_users(
    data: web::types::State<Arc<AppState>>,
    query: web::types::Json<SearchQuery>,
) -> Result<web::HttpResponse, AppError> {
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

// get all users
#[web::get("/users")]
async fn get_all_users(
    data: web::types::State<Arc<AppState>>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");
    let users = web::block(move || user::get_all_users(&mut conn))
        .await
        .map_err(|e| {
            log::error!("Failed to get all users: {:?}", e);
            AppError::BadRequest(e.to_string())
        })?;

    Ok(web::HttpResponse::Ok().json(&Response::<Vec<User>> {
        status: "success".to_string(),
        message: "Users found".to_string(),
        count: Some(users.len() as i64),
        data: Some(users),
    }))
}

// update a user by id
#[web::put("/user/{id}")]
async fn update_user_by_id(
    data: web::types::State<Arc<AppState>>,
    user_id: web::types::Path<i32>,
    user: web::types::Json<NewUser>,
) -> Result<web::HttpResponse, Error> {
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");
    let updated_user = web::block(move || {
        user::update_user_by_id(&mut conn, user_id.into_inner(), user.into_inner())
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
#[web::delete("/user/{id}")]
async fn delete_user_by_id(
    data: web::types::State<Arc<AppState>>,
    user_id: web::types::Path<i32>,
) -> Result<web::HttpResponse, Error> {
    let mut conn = data
        .pool
        .get()
        .expect("couldn't get db connection from pool");
    let deleted_user = web::block(move || user::delete_user_by_id(&mut conn, user_id.into_inner()))
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
