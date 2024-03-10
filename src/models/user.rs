use ::r2d2::PooledConnection;
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::VarChar;
use diesel::*;
use serde::{Deserialize, Serialize};
use std::io::Write;

use bcrypt::{hash, verify, DEFAULT_COST};

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
// Queryable will generate all of the code needed to load a Post struct from a SQL query.
// Selectable will generate code to construct a matching select clause based on your model type based on the table defined via the #[diesel(table_name = "your_table_name")] attribute.
#[derive(Queryable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::models::schema::users)]
// checks to verify that all field types in your struct are compatible with the backend you are using.
#[diesel(check_for_backend(diesel::pg::Pg))]
// the order of the fields in the struct must match the order of the columns in the table.
// [derive(Selectable)] + #[diesel(check_for_backend(YourBackendType))] to check for mismatching fields at compile time. This drastically improves the quality of the generated error messages by pointing to concrete type mismatches at field level.You need to specify the concrete database backend this specific struct is indented to be used with, as otherwise rustc cannot correctly identify the required deserialization implementation.

pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub avatar: Option<String>,
    pub role: Role,
    pub password: String,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub modified_at: Option<chrono::NaiveDateTime>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = crate::models::schema::users)]
pub struct NewUser {
    pub name: Option<String>,
    pub email: String,
    pub avatar: Option<String>,
    pub role: Role,
    pub password: String,
    pub modified_at: Option<chrono::NaiveDateTime>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
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

    let hashed_password = hash(&user.password, DEFAULT_COST).unwrap();
    let user = NewUser {
        password: hashed_password,
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
            if verify::<&str>(pwd, &user.password).unwrap() {
                Ok(Some(user))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

// get a user by id
pub fn get_user_by_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i32,
) -> diesel::QueryResult<Option<User>> {
    use crate::models::schema::users::dsl::*;

    users
        .filter(id.eq(user_id).and(deleted_at.is_null()))
        .select(User::as_select())
        .first::<User>(conn)
        .optional()
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

// get all users
pub fn get_all_users(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
) -> diesel::QueryResult<Vec<User>> {
    use crate::models::schema::users::dsl::*;

    users
        .filter(deleted_at.is_null())
        .select(User::as_select())
        .order_by(created_at.desc())
        .load(conn)
}

// update a user by id
pub fn update_user_by_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i32,
    mut user: NewUser,
) -> diesel::QueryResult<User> {
    use crate::models::schema::users::dsl::*;

    user.modified_at = Some(chrono::Local::now().naive_local());

    diesel::update(users.find(user_id))
        .set(&user)
        .get_result(conn)
}

// delete a user by id, soft delete by setting deleted_at
pub fn delete_user_by_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user_id: i32,
) -> diesel::QueryResult<User> {
    use crate::models::schema::users::dsl::*;

    let now = Some(chrono::Local::now().naive_local());

    diesel::update(users.find(user_id))
        .set(deleted_at.eq(now))
        .get_result(conn)
}

#[test]
fn test_verify_user() {
    let pwd = "123";
    let hashed_pwd = hash(pwd, DEFAULT_COST).unwrap();
    assert!(verify(pwd, &hashed_pwd).unwrap());
}
