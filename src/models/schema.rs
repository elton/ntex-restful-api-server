// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 255]
        avatar -> Nullable<Varchar>,
        created_at -> Nullable<Timestamp>,
        modified_at -> Nullable<Timestamp>,
        deleted_at -> Nullable<Timestamp>,
    }
}
