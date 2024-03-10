use ntex::web::{self, Error};

use crate::{
    errors::AppError,
    handlers::Response,
    models::user::{self, NewUser, SearchQuery, User, UserLogin},
    repository::database,
};

// create a new user
#[web::post("/user")]
async fn create_user(
    pool: web::types::State<database::DbPool>,
    user: web::types::Json<NewUser>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let cloned_user = user.clone();

    // query the user by email to check if it already exists
    let existing_user = web::block(move || user::get_user_by_email(&mut conn, &user.email))
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
    let mut conn = pool.get().expect("couldn't get db connection from pool");

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
    pool: web::types::State<database::DbPool>,
    user: web::types::Json<UserLogin>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
    let user = web::block(move || user::verify_user(&mut conn, &user.email, &user.password))
        .await
        .map_err(|e| {
            log::error!("Failed to verify user: {:?}", e);
            AppError::BadRequest(e.to_string())
        })?;

    match user {
        Some(user) => Ok(web::HttpResponse::Found().json(&Response::<&User> {
            status: "success".to_string(),
            message: "User verified".to_string(),
            count: None,
            data: Some(&user),
        })),
        None => Err(AppError::Unauthorized),
    }
}

// get a user by id
#[web::get("/user/{id}")]
async fn get_user_by_id(
    pool: web::types::State<database::DbPool>,
    user_id: web::types::Path<i32>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
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
    pool: web::types::State<database::DbPool>,
    user_name: web::types::Path<String>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
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
    pool: web::types::State<database::DbPool>,
    query: web::types::Json<SearchQuery>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
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
    pool: web::types::State<database::DbPool>,
) -> Result<web::HttpResponse, AppError> {
    let mut conn = pool.get().expect("couldn't get db connection from pool");
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
        count: None,
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
        count: None,
        data: Some(&deleted_user),
    }))
}
