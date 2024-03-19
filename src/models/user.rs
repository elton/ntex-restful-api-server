use ::r2d2::PooledConnection;
use chrono::Utc;
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::VarChar;
use diesel::BoolExpressionMethods;
use diesel::*;
use serde::{Deserialize, Serialize};
use std::io::Write;

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;

#[derive(Debug, PartialEq, FromSqlRow, AsExpression, Eq, Clone, Serialize, Deserialize, Copy)]
#[diesel(sql_type = diesel::sql_types::VarChar)]
#[serde(rename_all = "lowercase")]
// 定义Role枚举
pub enum Role {
    Admin,
    User,
}

// Implement the ToSql and FromSql traits for the Role enum
impl ToSql<VarChar, Pg> for Role {
    fn to_sql<'a>(&'a self, out: &mut Output<'a, '_, Pg>) -> serialize::Result {
        match *self {
            Role::Admin => out.write_all(b"admin")?,
            Role::User => out.write_all(b"user")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<VarChar, Pg> for Role {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"admin" => Ok(Role::Admin),
            b"user" => Ok(Role::User),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
// Queryable will generate all of the code needed to load a Post struct from a SQL query. Later, you can use User::as_select() to generate a select clause for your model type based on the table defined via the #[diesel(table_name = "your_table_name")] attribute.
// Selectable will generate code to construct a matching select clause based on your model type based on the table defined via the #[diesel(table_name = "your_table_name")] attribute.
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::models::schema::users)]
// checks to verify that all field types in your struct are compatible with the backend you are using.
#[diesel(check_for_backend(diesel::pg::Pg))]
// the order of the fields in the struct must match the order of the columns in the table and schema.

// [derive(Selectable)] + #[diesel(check_for_backend(YourBackendType))] to check for mismatching fields at compile time. This drastically improves the quality of the generated error messages by pointing to concrete type mismatches at field level.You need to specify the concrete database backend this specific struct is indented to be used with, as otherwise rustc cannot correctly identify the required deserialization implementation.

pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub avatar: Option<String>,
    pub password: String,
    pub role: Role,
    pub created_at: Option<chrono::DateTime<Utc>>,
    pub modified_at: Option<chrono::DateTime<Utc>>,
    pub deleted_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = crate::models::schema::users)]
pub struct NewUser {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub role: Option<Role>,
    pub password: Option<String>,
    pub created_at: Option<chrono::DateTime<Utc>>,
    pub modified_at: Option<chrono::DateTime<Utc>>,
    pub deleted_at: Option<chrono::DateTime<Utc>>,
}

// user login
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserLogin {
    pub email: String,
    pub password: String,
}

// search query
#[derive(Queryable, Deserialize, Serialize, Debug, Clone)]
pub struct SearchQuery {
    pub search_term: String,
    pub sort_by: String,
    pub order_by: String,
    pub page: i64,
    pub page_size: i64,
}

// get a user by email
pub fn get_user_by_email(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    email_add: &str,
) -> diesel::QueryResult<Option<User>> {
    use crate::models::schema::users::dsl::*;

    users
        .filter(email.eq(email_add).and(deleted_at.is_null()))
        .select(User::as_select())
        .first::<User>(conn)
        .optional()
}

// create a new user
pub fn create_user(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: NewUser,
) -> diesel::QueryResult<User> {
    use crate::models::schema::users::dsl::*;

    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(user.password.unwrap().as_bytes(), &salt)
        .map_err(|e| {
            log::error!("Error while hashing password: {:?}", e);
            diesel::result::Error::DeserializationError(
                format!("Error while hashing password: {}", e).into(),
            )
        })
        .map(|hash| hash.to_string())?;

    let user = NewUser {
        password: Some(hashed_password),
        created_at: Some(chrono::Utc::now()),
        modified_at: Some(chrono::Utc::now()),
        ..user
    };
    diesel::insert_into(users).values(&user).get_result(conn)
}

// verify a user by email and password
pub fn verify_user(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    email_add: &str,
    pwd: &str,
) -> diesel::QueryResult<Option<User>> {
    use crate::models::schema::users::dsl::*;

    let user = users
        .filter(email.eq(email_add).and(deleted_at.is_null()))
        .select(User::as_select())
        .first::<User>(conn)
        .optional()?;

    match user {
        Some(user) => {
            match PasswordHash::new(&user.password).map_err(|e| {
                log::error!("Error while verifying password: {:?}", e);
                diesel::result::Error::DeserializationError(
                    format!("Error while verifying password: {}", e).into(),
                )
            }) {
                Ok(hash) => argon2::Argon2::default()
                    .verify_password(pwd.as_bytes(), &hash)
                    .map_err(|e| {
                        log::error!("Error while verifying password: {:?}", e);
                        diesel::result::Error::DeserializationError(
                            format!("Error while verifying password: {}", e).into(),
                        )
                    })
                    .map_or(Ok(None), |_| Ok(Some(user))),
                Err(e) => Err(e),
            }
        }
        None => Ok(None),
    }
}

// get a user by id
pub fn get_users_by_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i32,
) -> diesel::QueryResult<Vec<User>> {
    use crate::models::schema::users::dsl::*;

    let user_vec = users
        .filter(id.eq(user_id).and(deleted_at.is_null()))
        .select(User::as_select())
        .load::<User>(conn)?;

    Ok(user_vec)
}

// get a users by name ignore case
pub fn get_users_by_name(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_name: &str,
) -> diesel::QueryResult<Vec<User>> {
    use crate::models::schema::users::dsl::*;

    users
        .filter(
            name.ilike(&format!("%{}%", user_name))
                .and(deleted_at.is_null()),
        )
        .select(User::as_select())
        .order_by(created_at.desc())
        .load(conn)
}

// search users by name or email with pagination and sorting
pub fn search_users(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    search_term: &str,
    sort_by: &str,
    order_by: &str,
    page: i64,
    page_size: i64,
) -> diesel::QueryResult<(Vec<User>, i64)> {
    use crate::models::schema::users::dsl::*;

    let offset = (page - 1) * page_size;

    let user_list = users
        .filter(
            name.ilike(&format!("%{}%", &search_term))
                .or(email.ilike(&format!("%{}%", &search_term)))
                .and(deleted_at.is_null()),
        )
        .select(User::as_select())
        .order_by(diesel::dsl::sql::<diesel::sql_types::Text>(&format!(
            "{} {}",
            sort_by, order_by
        )))
        .offset(offset)
        .limit(page_size)
        .load(conn)?;

    let total_count = users
        .filter(
            name.ilike(&format!("%{}%", &search_term))
                .or(email.ilike(&format!("%{}%", &search_term)))
                .and(deleted_at.is_null()),
        )
        .count()
        .get_result(conn)?;
    Ok((user_list, total_count))
}

// update a user by id
pub fn update_user_by_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i32,
    mut user: NewUser,
) -> diesel::QueryResult<User> {
    use crate::models::schema::users::dsl::*;

    user.modified_at = Some(chrono::Utc::now());

    diesel::update(users.find(user_id))
        .set(user)
        .get_result(conn)
}

// delete a user by id, soft delete by setting deleted_at
pub fn delete_user_by_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i32,
) -> diesel::QueryResult<User> {
    use crate::models::schema::users::dsl::*;

    let now = Some(chrono::Utc::now());

    diesel::update(users.find(user_id))
        .set(deleted_at.eq(now))
        .get_result(conn)
}

#[test]
fn test_verify_user() {
    let pwd = "123";
    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default()
        .hash_password(pwd.as_bytes(), &salt)
        .unwrap();
    assert!(argon2::Argon2::default()
        .verify_password(pwd.as_bytes(), &hashed_password)
        .is_ok());
}
