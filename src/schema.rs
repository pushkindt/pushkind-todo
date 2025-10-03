// @generated automatically by Diesel CLI.

diesel::table! {
    task_assignments (id) {
        id -> Integer,
        task_id -> Integer,
        hub_id -> Integer,
        assignee_id -> Integer,
        assigned_at -> Timestamp,
    }
}

diesel::table! {
    task_events (id) {
        id -> Integer,
        task_id -> Integer,
        user_id -> Nullable<Integer>,
        event_type -> Text,
        event_data -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    tasks (id) {
        id -> Integer,
        hub_id -> Integer,
        title -> Text,
        description -> Nullable<Text>,
        status -> Text,
        due_date -> Nullable<Date>,
        assigned_to -> Nullable<Integer>,
        author_id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        completed_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        email -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(task_assignments -> tasks (task_id));
diesel::joinable!(task_assignments -> users (assignee_id));
diesel::joinable!(task_events -> tasks (task_id));
diesel::joinable!(task_events -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(task_assignments, task_events, tasks, users,);
