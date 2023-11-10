// @generated automatically by Diesel CLI.

diesel::table! {
    app_versions (hash) {
        hash -> Text,
        app_id -> BigInt,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    applications (id) {
        id -> BigInt,
        name -> Text,
        git_url -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    hosts (id) {
        id -> BigInt,
        name -> Text,
        ip -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    app_versions,
    applications,
    hosts,
);
