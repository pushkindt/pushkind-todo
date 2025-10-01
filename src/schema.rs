// @generated automatically by Diesel CLI.

diesel::table! {
    templates (id) {
        id -> Integer,
        hub_id -> Integer,
        value -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}
