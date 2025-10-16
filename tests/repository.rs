use chrono::{Duration, NaiveDate};
use pushkind_common::repository::errors::RepositoryError;
use pushkind_todo::domain::task::{
    NewTask as DomainNewTask, TaskAssignment as DomainTaskAssignment, TaskListFilters, TaskStatus,
    UpdateTask as DomainUpdateTask,
};
use pushkind_todo::domain::task_event::{NewTaskEvent as DomainNewTaskEvent, TaskEventType};
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

    let alice_new = DomainNewUser::new(1, "Alice".to_string(), "ALICE@example.com".to_string());
    let bob_new = DomainNewUser::new(1, "Bob".to_string(), "bob@example.com".to_string());

    let alice = repo
        .create_or_update_user(&alice_new)
        .expect("create alice");
    let bob = repo.create_or_update_user(&bob_new).expect("create bob");

    assert_eq!(alice.email, "alice@example.com");
    assert_eq!(bob.email, "bob@example.com");

    let fetched_by_id = repo
        .get_user_by_id(alice.id, 1)
        .expect("get_user_by_id")
        .expect("alice should exist");
    assert_eq!(fetched_by_id.name, "Alice");

    let fetched_by_email = repo
        .get_user_by_email("ALICE@EXAMPLE.COM", 1)
        .expect("get_user_by_email")
        .expect("alice by email");
    assert_eq!(fetched_by_email.id, alice.id);

    let (total, mut users) = repo.list_users(UserListQuery::new(1)).expect("list users");
    assert_eq!(total, 2);
    assert_eq!(users.len(), 2);
    users.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");

    let (search_total, search_users) = repo
        .list_users(UserListQuery::new(1).search("lice"))
        .expect("search users");
    assert_eq!(search_total, 1);
    assert_eq!(search_users.first().expect("one user").id, alice.id);

    let updated = repo
        .update_user(
            alice.id,
            1,
            &DomainUpdateUser {
                name: "Alice Updated".to_string(),
            },
        )
        .expect("update alice");
    assert_eq!(updated.name, "Alice Updated");

    let err = repo
        .update_user(
            alice.id,
            2,
            &DomainUpdateUser {
                name: "Hacker".to_string(),
            },
        )
        .expect_err("cross-hub update should fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let err = repo
        .delete_user(alice.id, 2)
        .expect_err("cross-hub delete should fail");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_user(alice.id, 1).expect("delete alice");

    assert!(
        repo.get_user_by_id(alice.id, 1)
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

    let author = repo
        .create_or_update_user(&DomainNewUser::new(
            1,
            "Event Author".to_string(),
            "author@example.com".to_string(),
        ))
        .expect("create author user");

    let task = repo
        .create_task(&DomainNewTask::new(1, author.id, "Eventful Task"))
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
        .get_event_by_id(comment.id, 1)
        .expect("fetch event by id")
        .expect("comment should exist");
    assert_eq!(fetched.id, comment.id);

    assert!(
        repo.get_event_by_id(comment.id, 2)
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

    let events = repo.list_events_for_task(task.id, 1).expect("list events");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].id, status.id);
    assert_eq!(events[1].id, comment.id);

    let cross_events = repo
        .list_events_for_task(task.id, 2)
        .expect("cross hub list");
    assert!(cross_events.is_empty());

    let err = repo
        .delete_event(comment.id, 2)
        .expect_err("cross hub delete");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_event(comment.id, 1)
        .expect("delete comment event");

    assert!(
        repo.get_event_by_id(comment.id, 1)
            .expect("fetch after delete")
            .is_none()
    );

    let remaining = repo
        .list_events_for_task(task.id, 1)
        .expect("list remaining events");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, status.id);
}

#[test]
fn test_task_repository_crud() {
    let test_db = common::TestDb::new("test_task_repository_crud.db");
    let repo = DieselRepository::new(test_db.pool());

    let due_alpha = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid date");
    let due_beta = NaiveDate::from_ymd_opt(2024, 2, 1).expect("valid date");

    let author = repo
        .create_or_update_user(&DomainNewUser::new(
            1,
            "Task Author".to_string(),
            "author@example.com".to_string(),
        ))
        .expect("create author user");

    let assignee = repo
        .create_or_update_user(&DomainNewUser::new(
            1,
            "Task Owner".to_string(),
            "task-owner@example.com".to_string(),
        ))
        .expect("create assignee user");

    let alpha_new = DomainNewTask::new(1, author.id, "Alpha Task")
        .description("first task")
        .due_date(due_alpha);
    let beta_new = DomainNewTask::new(1, author.id, "Beta Task")
        .description("second task")
        .status(TaskStatus::InProgress)
        .assign_to(assignee.id)
        .due_date(due_beta);

    let mut alpha = repo.create_task(&alpha_new).expect("create alpha task");
    let beta = repo.create_task(&beta_new).expect("create beta task");

    assert_eq!(alpha.author_id, author.id);
    assert_eq!(beta.author_id, author.id);
    assert_eq!(alpha.title, "Alpha Task");
    assert_eq!(beta.status, TaskStatus::InProgress);
    assert_eq!(beta.assigned_to, Some(assignee.id));

    assert!(
        repo.get_task_by_id(alpha.id, 2)
            .expect("cross hub get")
            .is_none()
    );

    let (total, tasks) = repo.list_tasks(TaskListQuery::new(1)).expect("list tasks");
    assert_eq!(total, 2);
    assert_eq!(tasks.len(), 2);

    let filters = TaskListFilters::new(1).search("Alpha");
    let (search_total, search_results) = repo
        .list_tasks(TaskListQuery::new(1).with_filters(filters))
        .expect("search tasks");
    assert_eq!(search_total, 1);
    assert_eq!(search_results[0].id, alpha.id);

    let status_filters = TaskListFilters::new(1).with_status(TaskStatus::InProgress);
    let (status_total, status_results) = repo
        .list_tasks(TaskListQuery::new(1).with_filters(status_filters))
        .expect("status filter");
    assert_eq!(status_total, 1);
    assert_eq!(status_results[0].id, beta.id);

    let assignee_filters = TaskListFilters::new(1).for_assignee(assignee.id);
    let (assignee_total, assignee_results) = repo
        .list_tasks(TaskListQuery::new(1).with_filters(assignee_filters))
        .expect("assignee filter");
    assert_eq!(assignee_total, 1);
    assert_eq!(assignee_results[0].id, beta.id);

    let due_filters = TaskListFilters::new(1)
        .due_after(NaiveDate::from_ymd_opt(2024, 1, 15).expect("valid date"));
    let (due_total, due_results) = repo
        .list_tasks(TaskListQuery::new(1).with_filters(due_filters))
        .expect("due filter");
    assert_eq!(due_total, 1);
    assert_eq!(due_results[0].id, beta.id);

    let update = DomainUpdateTask::from_task(&alpha)
        .title("Alpha Updated")
        .status(TaskStatus::Completed);
    alpha = repo
        .update_task(alpha.id, 1, &update)
        .expect("update alpha");
    assert_eq!(alpha.title, "Alpha Updated");
    assert_eq!(alpha.status, TaskStatus::Completed);

    let cross_update = DomainUpdateTask::from_task(&alpha).title("Intruder");
    let err = repo
        .update_task(alpha.id, 2, &cross_update)
        .expect_err("cross hub update should fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let assignment = DomainTaskAssignment::new(beta.id, 1, assignee.id);
    repo.record_assignment(&assignment)
        .expect("record assignment");

    let assignments = repo
        .list_assignments_for_task(beta.id, 1)
        .expect("list assignments");
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].assignee_id, assignee.id);

    let err = repo
        .remove_assignment(beta.id, 2, assignee.id)
        .expect_err("cross hub assignment removal");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.remove_assignment(beta.id, 1, assignee.id)
        .expect("remove assignment");
    assert!(
        repo.list_assignments_for_task(beta.id, 1)
            .expect("assignments after remove")
            .is_empty()
    );

    let err = repo
        .delete_task(beta.id, 2)
        .expect_err("cross hub delete should fail");
    assert!(matches!(err, RepositoryError::NotFound));

    repo.delete_task(beta.id, 1).expect("delete beta");
    repo.delete_task(alpha.id, 1).expect("delete alpha");

    assert!(
        repo.get_task_by_id(alpha.id, 1)
            .expect("get alpha after delete")
            .is_none()
    );

    let (total_after, tasks_after) = repo
        .list_tasks(TaskListQuery::new(1))
        .expect("list after deletes");
    assert_eq!(total_after, 0);
    assert!(tasks_after.is_empty());
}
