// @generated automatically by Diesel CLI.

diesel::table! {
    hosts (id) {
        id -> BigInt,
        name -> Text,
        ip -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}
