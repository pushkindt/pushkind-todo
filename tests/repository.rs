use chrono::{Duration, NaiveDate};
use pushkind_common::repository::errors::RepositoryError;
use pushkind_todo::domain::task::{
    NewTask as DomainNewTask, TaskAssignment as DomainTaskAssignment, TaskListFilters,
    TaskPriority, TaskStatus, UpdateTask as DomainUpdateTask,
};
use pushkind_todo::domain::task_event::{NewTaskEvent as DomainNewTaskEvent, TaskEventType};
use pushkind_todo::domain::types::{
    HubId, SearchTerm, TaskDescription, TaskTitle, TaskTrack, UserEmail, UserName,
};
use pushkind_todo::domain::user::{NewUser as DomainNewUser, UpdateUser as DomainUpdateUser};
use pushkind_todo::repository::DieselRepository;
use pushkind_todo::repository::{
    TaskEventReader, TaskEventWriter, TaskListQuery, TaskReader, TaskWriter, UserListQuery,
    UserReader, UserWriter,
};
use serde_json::json;

mod common;

#[test]
fn test_user_repository_crud() {
    let test_db = common::TestDb::new("test_user_repository_crud.db");
    let repo = DieselRepository::new(test_db.pool());

    let hub_id = HubId::new(1).unwrap();
    let alice_new = DomainNewUser::new(
        hub_id,
        UserName::new("Alice").unwrap(),
        UserEmail::new("ALICE@example.com").unwrap(),
    );
    let bob_new = DomainNewUser::new(
        hub_id,
        UserName::new("Bob").unwrap(),
        UserEmail::new("bob@example.com").unwrap(),
    );

    let alice = repo
        .create_or_update_user(&alice_new)
        .expect("create alice");
    let bob = repo.create_or_update_user(&bob_new).expect("create bob");

    assert_eq!(alice.email.as_str(), "alice@example.com");
    assert_eq!(bob.email.as_str(), "bob@example.com");

    let fetched_by_id = repo
        .get_user_by_id(alice.id.get(), hub_id.get())
        .expect("get_user_by_id")
        .expect("alice should exist");
    assert_eq!(fetched_by_id.name.as_str(), "Alice");

    let fetched_by_email = repo
        .get_user_by_email("ALICE@EXAMPLE.COM", hub_id.get())
        .expect("get_user_by_email")
        .expect("alice by email");
    assert_eq!(fetched_by_email.id, alice.id);

    let (total, mut users) = repo.list_users(UserListQuery::new(1)).expect("list users");
    assert_eq!(total, 2);
    assert_eq!(users.len(), 2);
    users.sort_by_key(|user| user.name.as_str().to_owned());
    assert_eq!(users[0].name.as_str(), "Alice");
    assert_eq!(users[1].name.as_str(), "Bob");

    let (search_total, search_users) = repo
        .list_users(UserListQuery::new(1).search("lice"))
        .expect("search users");
    assert_eq!(search_total, 1);
    assert_eq!(search_users.first().expect("one user").id, alice.id);

    let updated = repo
        .update_user(
            alice.id.get(),
            hub_id.get(),
            &DomainUpdateUser {
                name: UserName::new("Alice Updated").unwrap(),
            },
        )
        .expect("update alice");
    assert_eq!(updated.name.as_str(), "Alice Updated");

    let err = repo
        .update_user(
            alice.id.get(),
            2,
            &DomainUpdateUser {
                name: UserName::new("Hacker").unwrap(),
            },
        )
        .expect_err("cross-hub update should fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let err = repo
        .delete_user(alice.id.get(), 2)
        .expect_err("cross-hub delete should fail");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_user(alice.id.get(), hub_id.get())
        .expect("delete alice");

    assert!(
        repo.get_user_by_id(alice.id.get(), hub_id.get())
            .expect("get after delete")
            .is_none()
    );

    let (total_after, users_after) = repo
        .list_users(UserListQuery::new(1))
        .expect("list after delete");
    assert_eq!(total_after, 1);
    assert_eq!(users_after[0].id, bob.id);
}

#[test]
fn test_task_event_repository_crud() {
    let test_db = common::TestDb::new("test_task_event_repository_crud.db");
    let repo = DieselRepository::new(test_db.pool());
    let hub_id = HubId::new(1).unwrap();

    let author = repo
        .create_or_update_user(&DomainNewUser::new(
            hub_id,
            UserName::new("Event Author").unwrap(),
            UserEmail::new("author@example.com").unwrap(),
        ))
        .expect("create author user");

    let task = repo
        .create_task(&DomainNewTask::new(
            hub_id,
            author.id,
            TaskTitle::new("Eventful Task").unwrap(),
        ))
        .expect("create task for events");
    assert_eq!(task.author_id, author.id);

    let mut comment_event = DomainNewTaskEvent::new(
        task.id,
        Some(author.id),
        TaskEventType::Comment,
        json!({"text": "Initial comment"}),
    );
    comment_event.created_at -= Duration::seconds(5);

    let comment = repo
        .record_event(&comment_event)
        .expect("record comment event");
    assert_eq!(comment.task_id, task.id);

    let fetched = repo
        .get_event_by_id(comment.id.get(), hub_id.get())
        .expect("fetch event by id")
        .expect("comment should exist");
    assert_eq!(fetched.id, comment.id);

    assert!(
        repo.get_event_by_id(comment.id.get(), 2)
            .expect("cross hub fetch")
            .is_none()
    );

    let mut status_event = DomainNewTaskEvent::new(
        task.id,
        None,
        TaskEventType::StatusChanged,
        json!({"from": "pending", "to": "in_progress"}),
    );
    status_event.created_at = comment_event.created_at + Duration::seconds(10);

    let status = repo
        .record_event(&status_event)
        .expect("record status change");

    let events = repo
        .list_events_for_task(task.id.get(), hub_id.get())
        .expect("list events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, status.id);
    assert_eq!(events[1].id, comment.id);

    let cross_events = repo
        .list_events_for_task(task.id.get(), 2)
        .expect("cross hub list");
    assert!(cross_events.is_empty());

    let err = repo
        .delete_event(comment.id.get(), 2)
        .expect_err("cross hub delete");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_event(comment.id.get(), hub_id.get())
        .expect("delete comment event");

    assert!(
        repo.get_event_by_id(comment.id.get(), hub_id.get())
            .expect("fetch after delete")
            .is_none()
    );

    let remaining = repo
        .list_events_for_task(task.id.get(), hub_id.get())
        .expect("list remaining events");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, status.id);
}

#[test]
fn test_task_repository_crud() {
    let test_db = common::TestDb::new("test_task_repository_crud.db");
    let repo = DieselRepository::new(test_db.pool());
    let hub_id = HubId::new(1).unwrap();

    let due_alpha = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
    let due_beta = NaiveDate::from_ymd_opt(2024, 2, 1).expect("valid date");

    let author = repo
        .create_or_update_user(&DomainNewUser::new(
            hub_id,
            UserName::new("Task Author").unwrap(),
            UserEmail::new("author@example.com").unwrap(),
        ))
        .expect("create author user");

    let assignee = repo
        .create_or_update_user(&DomainNewUser::new(
            hub_id,
            UserName::new("Task Owner").unwrap(),
            UserEmail::new("task-owner@example.com").unwrap(),
        ))
        .expect("create assignee user");

    let alpha_new = DomainNewTask::new(hub_id, author.id, TaskTitle::new("Alpha Task").unwrap())
        .description(TaskDescription::from("first task"))
        .track(TaskTrack::new("Activation").unwrap())
        .due_date(due_alpha);
    let beta_new = DomainNewTask::new(hub_id, author.id, TaskTitle::new("Beta Task").unwrap())
        .description(TaskDescription::from("second task"))
        .track(TaskTrack::new("Retention").unwrap())
        .status(TaskStatus::InProgress)
        .assign_to(assignee.id)
        .due_date(due_beta);

    let mut alpha = repo.create_task(&alpha_new).expect("create alpha task");
    let beta = repo.create_task(&beta_new).expect("create beta task");

    assert_eq!(alpha.author_id, author.id);
    assert_eq!(beta.author_id, author.id);
    assert_eq!(alpha.title.as_str(), "Alpha Task");
    assert_eq!(beta.status, TaskStatus::InProgress);
    assert_eq!(beta.assigned_to, Some(assignee.id));
    assert_eq!(alpha.track.as_ref().map(|t| t.as_str()), Some("Activation"));
    assert_eq!(beta.track.as_ref().map(|t| t.as_str()), Some("Retention"));
    assert_eq!(alpha.priority, TaskPriority::Middle);
    assert_eq!(beta.priority, TaskPriority::Middle);

    assert!(
        repo.get_task_by_id(alpha.id.get(), 2)
            .expect("cross hub get")
            .is_none()
    );

    let tracks = repo
        .list_task_tracks(hub_id.get())
        .expect("list distinct task tracks");
    assert_eq!(
        tracks,
        vec!["Activation".to_string(), "Retention".to_string()]
    );

    let (total, tasks) = repo
        .list_tasks(TaskListQuery::new(hub_id.get()).unwrap())
        .expect("list tasks");
    assert_eq!(total, 2);
    assert_eq!(tasks.len(), 2);

    let filters = TaskListFilters::new(hub_id).search(SearchTerm::new("Alpha").unwrap());
    let (search_total, search_results) = repo
        .list_tasks(
            TaskListQuery::new(hub_id.get())
                .unwrap()
                .with_filters(filters),
        )
        .expect("search tasks");
    assert_eq!(search_total, 1);
    assert_eq!(search_results[0].id, alpha.id);

    let status_filters = TaskListFilters::new(hub_id).with_status(TaskStatus::InProgress);
    let (status_total, status_results) = repo
        .list_tasks(
            TaskListQuery::new(hub_id.get())
                .unwrap()
                .with_filters(status_filters),
        )
        .expect("status filter");
    assert_eq!(status_total, 1);
    assert_eq!(status_results[0].id, beta.id);

    let assignee_filters = TaskListFilters::new(hub_id).for_assignee(assignee.id);
    let (assignee_total, assignee_results) = repo
        .list_tasks(
            TaskListQuery::new(hub_id.get())
                .unwrap()
                .with_filters(assignee_filters),
        )
        .expect("assignee filter");
    assert_eq!(assignee_total, 1);
    assert_eq!(assignee_results[0].id, beta.id);

    let due_filters = TaskListFilters::new(hub_id)
        .due_after(NaiveDate::from_ymd_opt(2024, 1, 15).expect("valid date"));
    let (due_total, due_results) = repo
        .list_tasks(
            TaskListQuery::new(hub_id.get())
                .unwrap()
                .with_filters(due_filters),
        )
        .expect("due filter");
    assert_eq!(due_total, 1);
    assert_eq!(due_results[0].id, beta.id);

    let update = DomainUpdateTask::from_task(&alpha)
        .title(TaskTitle::new("Alpha Updated").unwrap())
        .status(TaskStatus::Completed);
    alpha = repo
        .update_task(alpha.id.get(), hub_id.get(), &update)
        .expect("update alpha");
    assert_eq!(alpha.title.as_str(), "Alpha Updated");
    assert_eq!(alpha.status, TaskStatus::Completed);

    let cross_update =
        DomainUpdateTask::from_task(&alpha).title(TaskTitle::new("Intruder").unwrap());
    let err = repo
        .update_task(alpha.id.get(), 2, &cross_update)
        .expect_err("cross hub update should fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let assignment = DomainTaskAssignment::new(beta.id, hub_id, assignee.id);
    repo.record_assignment(&assignment)
        .expect("record assignment");

    let assignments = repo
        .list_assignments_for_task(beta.id.get(), hub_id.get())
        .expect("list assignments");
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].assignee_id, assignee.id);

    let err = repo
        .remove_assignment(beta.id.get(), 2, assignee.id.get())
        .expect_err("cross hub assignment removal");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.remove_assignment(beta.id.get(), hub_id.get(), assignee.id.get())
        .expect("remove assignment");
    assert!(
        repo.list_assignments_for_task(beta.id.get(), hub_id.get())
            .expect("assignments after remove")
            .is_empty()
    );

    let err = repo
        .delete_task(beta.id.get(), 2)
        .expect_err("cross hub delete should fail");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_task(beta.id.get(), hub_id.get())
        .expect("delete beta");
    repo.delete_task(alpha.id.get(), hub_id.get())
        .expect("delete alpha");

    assert!(
        repo.get_task_by_id(alpha.id.get(), hub_id.get())
            .expect("get alpha after delete")
            .is_none()
    );

    let (total_after, tasks_after) = repo
        .list_tasks(TaskListQuery::new(1).unwrap())
        .expect("list after deletes");
    assert_eq!(total_after, 0);
    assert!(tasks_after.is_empty());
}
