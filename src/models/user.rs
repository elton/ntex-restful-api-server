use ::r2d2::PooledConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use serde::{Deserialize, Serialize};

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
    pub created_at: Option<chrono::NaiveDateTime>,
    pub modified_at: Option<chrono::NaiveDateTime>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = crate::models::schema::users)]
pub struct NewUser {
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub modified_at: Option<chrono::NaiveDateTime>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
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

// create a new user
pub fn create_user<'a>(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    user: NewUser,
) -> diesel::QueryResult<User> {
    use crate::models::schema::users::dsl::*;

    diesel::insert_into(users).values(&user).get_result(conn)
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
