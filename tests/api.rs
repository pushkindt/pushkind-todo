//! Integration tests for React-facing API contracts.
use chrono::NaiveDate;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::services::errors::ServiceError;
use serde_json::json;

use pushkind_todo::domain::client::NewClient;
use pushkind_todo::domain::task::{NewTask, TaskPriority, TaskStatus};
use pushkind_todo::domain::task_event::{NewTaskEvent, TaskEventType};
use pushkind_todo::domain::types::{
    ClientName, ClientPublicId, HubId, TaskComment, TaskDescription, TaskTitle, TaskTrack,
    UserEmail, UserName,
};
use pushkind_todo::domain::user::NewUser;
use pushkind_todo::dto::api::{ClientLookupQueryDto, LookupQueryDto};
use pushkind_todo::dto::main::IndexQuery;
use pushkind_todo::forms::task::{QuickTaskStatusPayload, TaskCommentPayload, UpdateTaskPayload};
use pushkind_todo::repository::{
    ClientWriter, DieselRepository, TaskEventWriter, TaskReader, TaskWriter, UserWriter,
};
use pushkind_todo::services::api::{
    get_task_collection_data, get_task_details_data, list_clients, list_tracks, list_users,
};
use pushkind_todo::services::mock::MockZmqSender;
use pushkind_todo::services::task as task_service;

mod common;

fn authenticated_user(roles: &[&str]) -> AuthenticatedUser {
    AuthenticatedUser {
        sub: "user-1".to_string(),
        email: "viewer@example.com".to_string(),
        hub_id: 1,
        name: "Viewer".to_string(),
        roles: roles.iter().map(|role| (*role).to_string()).collect(),
        exp: 0,
    }
}

#[test]
fn task_collection_contract_includes_items_filters_and_lookups() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let hub_id = HubId::new(1).unwrap();

    let author = repo
        .create_or_update_user(&NewUser::new(
            hub_id,
            UserName::new("Author").unwrap(),
            UserEmail::new("author@example.com").unwrap(),
        ))
        .expect("create author");
    let assignee = repo
        .create_or_update_user(&NewUser::new(
            hub_id,
            UserName::new("Worker").unwrap(),
            UserEmail::new("worker@example.com").unwrap(),
        ))
        .expect("create assignee");
    let client = repo
        .create_or_update_client(&NewClient::new(
            hub_id,
            ClientName::new("ACME").unwrap(),
            ClientPublicId::new("client-7").unwrap(),
        ))
        .expect("create client");

    let active_task = repo
        .create_task(
            &NewTask::new(hub_id, author.id, TaskTitle::new("Alpha Task").unwrap())
                .description(TaskDescription::new("<p>Body</p>").unwrap())
                .track(TaskTrack::new("Support").unwrap())
                .assign_to(assignee.id)
                .client_id(client.id)
                .status(TaskStatus::InProgress)
                .due_date(NaiveDate::from_ymd_opt(2024, 2, 10).unwrap()),
        )
        .expect("create active task");
    repo.create_task(
        &NewTask::new(hub_id, author.id, TaskTitle::new("Archived Task").unwrap())
            .track(TaskTrack::new("Archive").unwrap())
            .status(TaskStatus::Completed),
    )
    .expect("create completed task");

    let dto = get_task_collection_data(
        IndexQuery::default(),
        &authenticated_user(&["todo"]),
        &repo,
        "https://files.example.com",
    )
    .expect("task collection should load");
    let payload = serde_json::to_value(dto).expect("serialize collection dto");

    assert_eq!(payload["items"].as_array().expect("items array").len(), 1);
    assert_eq!(payload["items"][0]["id"], json!(active_task.id.get()));
    assert_eq!(payload["items"][0]["title"], json!("Alpha Task"));
    assert_eq!(payload["items"][0]["status"], json!("InProgress"));
    assert_eq!(
        payload["items"][0]["client"]["public_id"],
        json!("client-7")
    );
    assert_eq!(payload["active_filters"]["status"], serde_json::Value::Null);
    assert_eq!(
        payload["files_service_url"],
        json!("https://files.example.com")
    );
    assert_eq!(
        payload["lookups"]["users"]["items"][0]["email"],
        json!("author@example.com")
    );
    assert_eq!(
        payload["lookups"]["tracks"]["items"][0]["value"],
        json!("Archive")
    );
}

#[test]
fn task_details_contract_includes_related_entities_and_events() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let hub_id = HubId::new(1).unwrap();

    let author = repo
        .create_or_update_user(&NewUser::new(
            hub_id,
            UserName::new("Author").unwrap(),
            UserEmail::new("author@example.com").unwrap(),
        ))
        .expect("create author");
    let assignee = repo
        .create_or_update_user(&NewUser::new(
            hub_id,
            UserName::new("Worker").unwrap(),
            UserEmail::new("worker@example.com").unwrap(),
        ))
        .expect("create assignee");
    let client = repo
        .create_or_update_client(&NewClient::new(
            hub_id,
            ClientName::new("ACME").unwrap(),
            ClientPublicId::new("client-7").unwrap(),
        ))
        .expect("create client");

    let task = repo
        .create_task(
            &NewTask::new(hub_id, author.id, TaskTitle::new("Detailed Task").unwrap())
                .assign_to(assignee.id)
                .client_id(client.id)
                .track(TaskTrack::new("Support").unwrap())
                .status(TaskStatus::InProgress),
        )
        .expect("create task");

    repo.record_event(&NewTaskEvent::new(
        task.id,
        Some(author.id),
        TaskEventType::Comment,
        json!({ "text": "Started" }),
    ))
    .expect("record event");

    let dto = get_task_details_data(
        task.id.get(),
        &authenticated_user(&["todo"]),
        &repo,
        "https://files.example.com",
    )
    .expect("task details should load");
    let payload = serde_json::to_value(dto).expect("serialize details dto");

    assert_eq!(payload["task"]["id"], json!(task.id.get()));
    assert_eq!(payload["author"]["email"], json!("author@example.com"));
    assert_eq!(payload["assignee"]["name"], json!("Worker"));
    assert_eq!(payload["client"]["public_id"], json!("client-7"));
    assert_eq!(
        payload["files_service_url"],
        json!("https://files.example.com")
    );
    assert_eq!(payload["events"][0]["event_type"], json!("Comment"));
    assert_eq!(
        payload["events"][0]["event_data"],
        json!({ "text": "Started" })
    );
}

#[test]
fn lookup_contracts_filter_by_query() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let hub_id = HubId::new(1).unwrap();

    repo.create_or_update_user(&NewUser::new(
        hub_id,
        UserName::new("Anna").unwrap(),
        UserEmail::new("anna@example.com").unwrap(),
    ))
    .expect("create anna");
    let author = repo
        .create_or_update_user(&NewUser::new(
            hub_id,
            UserName::new("Boris").unwrap(),
            UserEmail::new("boris@example.com").unwrap(),
        ))
        .expect("create boris");

    repo.create_or_update_client(&NewClient::new(
        hub_id,
        ClientName::new("ACME").unwrap(),
        ClientPublicId::new("ac-1").unwrap(),
    ))
    .expect("create acme");
    repo.create_or_update_client(&NewClient::new(
        hub_id,
        ClientName::new("Globex").unwrap(),
        ClientPublicId::new("gl-1").unwrap(),
    ))
    .expect("create globex");

    repo.create_task(
        &NewTask::new(hub_id, author.id, TaskTitle::new("Support Task").unwrap())
            .track(TaskTrack::new("Support").unwrap()),
    )
    .expect("create support task");
    repo.create_task(
        &NewTask::new(hub_id, author.id, TaskTitle::new("Sales Task").unwrap())
            .track(TaskTrack::new("Sales").unwrap()),
    )
    .expect("create sales task");

    let user_lookup = list_users(
        LookupQueryDto {
            query: Some("ann".to_string()),
        },
        &authenticated_user(&["todo"]),
        &repo,
    )
    .expect("users lookup");
    let client_lookup = list_clients(
        ClientLookupQueryDto {
            search: Some("ac".to_string()),
        },
        &authenticated_user(&["todo"]),
        &repo,
    )
    .expect("clients lookup");
    let track_lookup = list_tracks(
        LookupQueryDto {
            query: Some("sup".to_string()),
        },
        &authenticated_user(&["todo"]),
        &repo,
    )
    .expect("tracks lookup");

    assert_eq!(user_lookup.items.len(), 1);
    assert_eq!(user_lookup.items[0].email, "anna@example.com");
    assert_eq!(client_lookup.items.len(), 1);
    assert_eq!(client_lookup.items[0].public_id, "ac-1");
    assert_eq!(track_lookup.items.len(), 1);
    assert_eq!(track_lookup.items[0].value, "Support");
}

#[test]
fn task_details_contract_reflects_update_status_and_comment_mutations() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let hub_id = HubId::new(1).unwrap();
    let user = authenticated_user(&["todo"]);
    let zmq = MockZmqSender;

    let author = repo
        .create_or_update_user(&NewUser::new(
            hub_id,
            UserName::new("Author").unwrap(),
            UserEmail::new("author@example.com").unwrap(),
        ))
        .expect("create author");

    let task = repo
        .create_task(&NewTask::new(
            hub_id,
            author.id,
            TaskTitle::new("Original Task").unwrap(),
        ))
        .expect("create task");

    task_service::update_task(
        task.id.get(),
        UpdateTaskPayload {
            title: TaskTitle::new("Updated Task").unwrap(),
            description: Some(TaskDescription::new("<p>Updated body</p>").unwrap()),
            track: Some(TaskTrack::new("Escalation").unwrap()),
            priority: TaskPriority::High,
            status: TaskStatus::InProgress,
            due_date: Some(NaiveDate::from_ymd_opt(2024, 3, 12).unwrap()),
            assignee: None,
            client: None,
        },
        &user,
        &repo,
        &zmq,
        &zmq,
    )
    .expect("update task");

    task_service::transition_task_status(
        task.id.get(),
        QuickTaskStatusPayload {
            status: TaskStatus::Completed,
            comment: Some(TaskComment::new("<p>Готово</p>").unwrap()),
            assign_self: false,
        },
        &user,
        &repo,
        &zmq,
        &zmq,
    )
    .expect("complete task");

    task_service::add_task_comment(
        task.id.get(),
        TaskCommentPayload {
            message: TaskComment::new("<p>Новый комментарий</p>").unwrap(),
        },
        &user,
        &repo,
        &zmq,
    )
    .expect("add comment");

    let dto = get_task_details_data(task.id.get(), &user, &repo, "https://files.example.com")
        .expect("task details");
    let payload = serde_json::to_value(dto).expect("serialize details dto");

    assert_eq!(payload["task"]["title"], json!("Updated Task"));
    assert_eq!(payload["task"]["status"], json!("Completed"));
    assert_eq!(payload["task"]["track"], json!("Escalation"));
    assert_eq!(payload["task"]["priority"], json!("High"));
    assert_eq!(payload["task"]["due_date"], json!("2024-03-12"));
    assert!(
        payload["events"]
            .as_array()
            .expect("events array")
            .iter()
            .any(|event| event["event_type"] == json!("MetadataUpdated"))
    );
    assert!(
        payload["events"]
            .as_array()
            .expect("events array")
            .iter()
            .any(|event| event["event_type"] == json!("StatusChanged"))
    );
    assert!(
        payload["events"]
            .as_array()
            .expect("events array")
            .iter()
            .any(|event| event["event_type"] == json!("Comment"))
    );
}

#[test]
fn deleted_task_disappears_from_task_details_contract() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());
    let hub_id = HubId::new(1).unwrap();

    let author = repo
        .create_or_update_user(&NewUser::new(
            hub_id,
            UserName::new("Author").unwrap(),
            UserEmail::new("author@example.com").unwrap(),
        ))
        .expect("create author");

    let task = repo
        .create_task(&NewTask::new(
            hub_id,
            author.id,
            TaskTitle::new("Disposable Task").unwrap(),
        ))
        .expect("create task");

    task_service::delete_task(task.id.get(), &authenticated_user(&["todo"]), &repo)
        .expect("delete task");

    assert!(
        repo.get_task_by_id(task.id, hub_id)
            .expect("query task")
            .is_none()
    );

    let result = get_task_details_data(
        task.id.get(),
        &authenticated_user(&["todo"]),
        &repo,
        "https://files.example.com",
    );
    assert!(matches!(result, Err(ServiceError::NotFound)));
}
