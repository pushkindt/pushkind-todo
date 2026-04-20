//! DTOs exposed by React-owned ToDo API endpoints.

use pushkind_common::routes::empty_string_as_none_fromstr;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{
    client::Client,
    task::{Task, TaskPriority, TaskStatus},
    task_event::TaskEventType,
    types::{TaskPublicId, TaskTrack, UserId},
    user::User,
};
use crate::dto::main::{IndexPageFilters, IndexTask};
use crate::dto::task::TaskEventWithAuthor;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskUserSummaryDto {
    pub id: i32,
    pub name: String,
    pub email: String,
}

impl From<&User> for TaskUserSummaryDto {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.get(),
            name: user.name.to_string(),
            email: user.email.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskClientSummaryDto {
    pub id: i32,
    pub name: String,
    pub public_id: String,
    pub url: Option<String>,
}

impl From<&Client> for TaskClientSummaryDto {
    fn from(client: &Client) -> Self {
        Self {
            id: client.id.get(),
            name: client.name.to_string(),
            public_id: client.public_id.to_string(),
            url: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskListItemDto {
    pub id: i32,
    pub public_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub track: Option<String>,
    pub priority: String,
    pub status: String,
    pub due_date: Option<String>,
    pub assignee: Option<TaskUserSummaryDto>,
    pub client: Option<TaskClientSummaryDto>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

impl TaskListItemDto {
    pub fn from_parts(index_task: &IndexTask, client: Option<&Client>) -> Self {
        Self {
            id: index_task.task.id.get(),
            public_id: index_task.task.public_id.map(task_public_id_to_string),
            title: index_task.task.title.to_string(),
            description: index_task
                .task
                .description
                .as_ref()
                .map(ToString::to_string),
            track: index_task.task.track.as_ref().map(ToString::to_string),
            priority: task_priority_to_string(index_task.task.priority).to_string(),
            status: task_status_to_string(index_task.task.status).to_string(),
            due_date: index_task.task.due_date.map(date_to_string),
            assignee: index_task.assignee.as_ref().map(TaskUserSummaryDto::from),
            client: client.map(TaskClientSummaryDto::from),
            created_at: datetime_to_string(index_task.task.created_at),
            updated_at: datetime_to_string(index_task.task.updated_at),
            completed_at: index_task.task.completed_at.map(datetime_to_string),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskPaginationDto {
    pub page: usize,
    pub total_pages: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskCollectionFiltersDto {
    pub search: Option<String>,
    pub status: Option<String>,
    pub track: Option<String>,
    pub assignee_id: Option<i32>,
    pub client_id: Option<i32>,
    pub priority: Option<String>,
    pub updated_after: Option<String>,
    pub updated_before: Option<String>,
    pub public_id: Option<String>,
}

impl From<&IndexPageFilters> for TaskCollectionFiltersDto {
    fn from(filters: &IndexPageFilters) -> Self {
        Self {
            search: filters.search.clone(),
            status: filters
                .status
                .map(task_status_to_string)
                .map(str::to_string),
            track: filters.track.as_ref().map(ToString::to_string),
            assignee_id: filters.assignee.map(UserId::get),
            client_id: filters.client.map(|id| id.get()),
            priority: filters
                .priority
                .map(task_priority_to_string)
                .map(str::to_string),
            updated_after: filters.updated_after.map(date_to_string),
            updated_before: filters.updated_before.map(date_to_string),
            public_id: filters.public_id.map(task_public_id_to_string),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserLookupItemDto {
    pub id: i32,
    pub name: String,
    pub email: String,
}

impl From<&User> for UserLookupItemDto {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.get(),
            name: user.name.to_string(),
            email: user.email.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserLookupCollectionDto {
    pub items: Vec<UserLookupItemDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientLookupItemDto {
    pub id: i32,
    pub name: String,
    pub public_id: String,
}

impl From<&Client> for ClientLookupItemDto {
    fn from(client: &Client) -> Self {
        Self {
            id: client.id.get(),
            name: client.name.to_string(),
            public_id: client.public_id.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientLookupCollectionDto {
    pub items: Vec<ClientLookupItemDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackLookupItemDto {
    pub value: String,
}

impl From<&TaskTrack> for TrackLookupItemDto {
    fn from(track: &TaskTrack) -> Self {
        Self {
            value: track.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackLookupCollectionDto {
    pub items: Vec<TrackLookupItemDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskCollectionLookupsDto {
    pub users: UserLookupCollectionDto,
    pub clients: ClientLookupCollectionDto,
    pub tracks: TrackLookupCollectionDto,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskCollectionDto {
    pub items: Vec<TaskListItemDto>,
    pub pagination: TaskPaginationDto,
    pub active_filters: TaskCollectionFiltersDto,
    pub recently_updated_task_ids: Vec<i32>,
    pub lookups: TaskCollectionLookupsDto,
    pub files_service_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskDetailsTaskDto {
    pub id: i32,
    pub public_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub track: Option<String>,
    pub priority: String,
    pub status: String,
    pub due_date: Option<String>,
    pub author_id: i32,
    pub assignee_id: Option<i32>,
    pub client_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

impl From<&Task> for TaskDetailsTaskDto {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id.get(),
            public_id: task.public_id.map(task_public_id_to_string),
            title: task.title.to_string(),
            description: task.description.as_ref().map(ToString::to_string),
            track: task.track.as_ref().map(ToString::to_string),
            priority: task_priority_to_string(task.priority).to_string(),
            status: task_status_to_string(task.status).to_string(),
            due_date: task.due_date.map(date_to_string),
            author_id: task.author_id.get(),
            assignee_id: task.assigned_to.map(|id| id.get()),
            client_id: task.client_id.map(|id| id.get()),
            created_at: datetime_to_string(task.created_at),
            updated_at: datetime_to_string(task.updated_at),
            completed_at: task.completed_at.map(datetime_to_string),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskEventItemDto {
    pub id: i32,
    pub event_type: String,
    pub event_data: Value,
    pub created_at: String,
    pub author: Option<TaskUserSummaryDto>,
}

impl From<&TaskEventWithAuthor> for TaskEventItemDto {
    fn from(event: &TaskEventWithAuthor) -> Self {
        Self {
            id: event.event.id.get(),
            event_type: task_event_type_to_string(event.event.event_type).to_string(),
            event_data: event.event.event_data.clone(),
            created_at: datetime_to_string(event.event.created_at),
            author: event.author.as_ref().map(TaskUserSummaryDto::from),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TaskDetailsDto {
    pub task: TaskDetailsTaskDto,
    pub author: TaskUserSummaryDto,
    pub assignee: Option<TaskUserSummaryDto>,
    pub client: Option<TaskClientSummaryDto>,
    pub events: Vec<TaskEventItemDto>,
    pub files_service_url: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct LookupQueryDto {
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub query: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ClientLookupQueryDto {
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub search: Option<String>,
}

fn task_priority_to_string(priority: TaskPriority) -> &'static str {
    priority.into()
}

fn task_status_to_string(status: TaskStatus) -> &'static str {
    status.into()
}

fn task_event_type_to_string(event_type: TaskEventType) -> &'static str {
    event_type.into()
}

fn date_to_string(date: chrono::NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn datetime_to_string(datetime: chrono::NaiveDateTime) -> String {
    datetime.format("%Y-%m-%dT%H:%M:%S%.f").to_string()
}

fn task_public_id_to_string(public_id: TaskPublicId) -> String {
    public_id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;
    use std::str::FromStr;

    use crate::domain::{
        client::Client,
        task::{Task, TaskPriority, TaskStatus},
        task_event::{TaskEvent, TaskEventType},
        types::{
            ClientId, ClientName, ClientPublicId, HubId, TaskDescription, TaskEventId, TaskId,
            TaskTitle, TaskTrack, UserEmail, UserId, UserName,
        },
        user::User,
    };

    fn sample_user(id: i32, name: &str, email: &str) -> User {
        User {
            id: UserId::new(id).unwrap(),
            hub_id: HubId::new(42).unwrap(),
            name: UserName::new(name).unwrap(),
            email: UserEmail::new(email).unwrap(),
            visited_at: None,
        }
    }

    fn sample_client() -> Client {
        let now = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();

        Client::new(
            ClientId::new(7).unwrap(),
            ClientPublicId::new("client-7").unwrap(),
            HubId::new(42).unwrap(),
            ClientName::new("ACME").unwrap(),
            now,
            now,
        )
    }

    fn sample_task() -> Task {
        let created_at = NaiveDate::from_ymd_opt(2024, 2, 1)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap();
        let updated_at = NaiveDate::from_ymd_opt(2024, 2, 2)
            .unwrap()
            .and_hms_opt(10, 45, 0)
            .unwrap();

        Task {
            id: TaskId::new(5).unwrap(),
            hub_id: HubId::new(42).unwrap(),
            title: TaskTitle::new("Подготовить отчёт").unwrap(),
            description: Some(TaskDescription::new("<p>Описание</p>").unwrap()),
            track: Some(TaskTrack::new("Support").unwrap()),
            priority: TaskPriority::High,
            status: TaskStatus::InProgress,
            due_date: Some(NaiveDate::from_ymd_opt(2024, 2, 10).unwrap()),
            assigned_to: Some(UserId::new(9).unwrap()),
            author_id: UserId::new(1).unwrap(),
            created_at,
            updated_at,
            completed_at: None,
            client_id: Some(ClientId::new(7).unwrap()),
            public_id: Some(
                TaskPublicId::from_str("7d3a1f0d-9960-4ed5-b1b6-8ced31ff5f87").unwrap(),
            ),
        }
    }

    #[test]
    fn task_list_item_conversion_preserves_nested_entities() {
        let assignee = sample_user(9, "Исполнитель", "worker@example.com");
        let client = sample_client();
        let dto = TaskListItemDto::from_parts(
            &IndexTask {
                task: sample_task(),
                assignee: Some(assignee),
            },
            Some(&client),
        );

        assert_eq!(dto.id, 5);
        assert_eq!(dto.priority, "High");
        assert_eq!(dto.status, "InProgress");
        assert_eq!(dto.assignee.unwrap().email, "worker@example.com");
        assert_eq!(dto.client.unwrap().public_id, "client-7");
        assert_eq!(dto.due_date.as_deref(), Some("2024-02-10"));
    }

    #[test]
    fn task_details_conversion_exposes_event_timeline() {
        let author = sample_user(1, "Автор", "author@example.com");
        let event = TaskEventWithAuthor {
            event: TaskEvent {
                id: TaskEventId::new(3).unwrap(),
                task_id: TaskId::new(5).unwrap(),
                user_id: Some(UserId::new(1).unwrap()),
                event_type: TaskEventType::Comment,
                event_data: json!({ "text": "Принято в работу" }),
                created_at: NaiveDate::from_ymd_opt(2024, 2, 2)
                    .unwrap()
                    .and_hms_opt(12, 0, 0)
                    .unwrap(),
            },
            author: Some(author.clone()),
        };

        let dto = TaskDetailsDto {
            task: TaskDetailsTaskDto::from(&sample_task()),
            author: TaskUserSummaryDto::from(&author),
            assignee: None,
            client: Some(TaskClientSummaryDto::from(&sample_client())),
            events: vec![TaskEventItemDto::from(&event)],
            files_service_url: "https://files.example.com".to_string(),
        };

        assert_eq!(dto.task.title, "Подготовить отчёт");
        assert_eq!(dto.author.email, "author@example.com");
        assert_eq!(dto.files_service_url, "https://files.example.com");
        assert_eq!(dto.events[0].event_type, "Comment");
        assert_eq!(
            dto.events[0].event_data,
            json!({ "text": "Принято в работу" })
        );
    }

    #[test]
    fn lookup_dto_conversion_keeps_user_identity() {
        let dto = UserLookupItemDto::from(&sample_user(9, "Исполнитель", "worker@example.com"));

        assert_eq!(dto.id, 9);
        assert_eq!(dto.name, "Исполнитель");
        assert_eq!(dto.email, "worker@example.com");
    }
}
