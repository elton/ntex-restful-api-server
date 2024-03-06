use ntex::web::{self, Error};

use crate::{
    handlers::Response,
    models::user::{self, NewUser, User},
    repository::database,
};

// create a new user
#[web::post("/user")]
async fn create_user(
    pool: web::types::State<database::DbPool>,
    user: web::types::Json<NewUser>,
) -> Result<web::HttpResponse, web::Error> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let new_user = web::block(move || {
        // Obtaining a connection from the pool is also a potentially blocking operation. So, it should be called within the `web::block` closure, as well.
        user::create_user(&mut conn, user.into_inner())
    })
    .await
    .map_err(|e| {
        log::error!("Failed to create new user: {:?}", e);
        web::Error::from(e)
    })?;

    Ok(web::HttpResponse::Created().json(&Response::<&User> {
        status: "success".to_string(),
        message: format!(
            "User `{}` with id `{}` created successfully",
            new_user.name, new_user.id
        ),
        data: Some(&new_user),
    }))
}

// get a user by id
#[web::get("/user/{id}")]
async fn get_user_by_id(
    pool: web::types::State<database::DbPool>,
    user_id: web::types::Path<i32>,
) -> Result<web::HttpResponse, web::Error> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let user = web::block(move || user::get_user_by_id(&mut conn, user_id.into_inner()))
        .await
        .map_err(|e| {
            log::error!("Failed to get user by id: {:?}", e);
            web::Error::from(e)
        })?;

    match user {
        Some(user) => Ok(web::HttpResponse::Ok().json(&Response::<&User> {
            status: "success".to_string(),
            message: "User found".to_string(),
            data: Some(&user),
        })),
        None => Ok(web::HttpResponse::NotFound().json(&Response::<()> {
            status: "success".to_string(),
            message: "User not found".to_string(),
            data: None,
        })),
    }
}

// get users by name ignore case
#[web::get("/users/{name}")]
async fn get_users_by_name(
    pool: web::types::State<database::DbPool>,
    user_name: web::types::Path<String>,
) -> Result<web::HttpResponse, Error> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let users = web::block(move || user::get_users_by_name(&mut conn, &user_name.into_inner()))
        .await
        .map_err(|e| {
            log::error!("Failed to get users by name: {:?}", e);
            web::Error::from(e)
        })?;

    Ok(web::HttpResponse::Ok().json(&Response::<Vec<User>> {
        status: "success".to_string(),
        message: "Users found".to_string(),
        data: Some(users),
    }))
}

// get all users
#[web::get("/users")]
async fn get_all_users(
    pool: web::types::State<database::DbPool>,
) -> Result<web::HttpResponse, Error> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let users = web::block(move || user::get_all_users(&mut conn))
        .await
        .map_err(|e| {
            log::error!("Failed to get all users: {:?}", e);
            web::Error::from(e)
        })?;

    Ok(web::HttpResponse::Ok().json(&Response::<Vec<User>> {
        status: "success".to_string(),
        message: "Users found".to_string(),
        data: Some(users),
    }))
}

// update a user by id
#[web::put("/user/{id}")]
async fn update_user_by_id(
    pool: web::types::State<database::DbPool>,
    user_id: web::types::Path<i32>,
    user: web::types::Json<NewUser>,
) -> Result<web::HttpResponse, Error> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
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
        data: Some(&updated_user),
    }))
}

// delete a user by id, soft delete by setting deleted_at
#[web::delete("/user/{id}")]
async fn delete_user_by_id(
    pool: web::types::State<database::DbPool>,
    user_id: web::types::Path<i32>,
) -> Result<web::HttpResponse, Error> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
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
        data: Some(&deleted_user),
    }))
}
