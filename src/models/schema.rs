// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 128]
        name -> Varchar,
        email -> Varchar,
        #[max_length = 128]
        avatar -> Nullable<Varchar>,
        #[max_length = 128]
        password -> Varchar,
        #[max_length = 48]
        role -> Varchar,
        created_at -> Nullable<Timestamptz>,
        modified_at -> Nullable<Timestamptz>,
        deleted_at -> Nullable<Timestamp>,
    }
}
