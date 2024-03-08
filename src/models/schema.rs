// @generated automatically by Diesel CLI.

diesel::table! {

    users (id) {
        id -> Int4,
        name -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        avatar -> Nullable<Varchar>,
        #[max_length = 255]
        role -> Varchar,
        #[max_length = 255]
        password -> Varchar,
        created_at -> Nullable<Timestamp>,
        modified_at -> Nullable<Timestamp>,
        deleted_at -> Nullable<Timestamp>,
    }
}
