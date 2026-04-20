//! Service helpers serving React-owned shell, page-data, and lookup contracts.
use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::dto::shell::{CurrentUserDto, IamDto, NavigationItemDto, NoAccessPageDto};
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{check_role, ensure_role};
use pushkind_common::services::errors::ServiceResult;

use crate::SERVICE_ACCESS_ROLE;
use crate::dto::api::{
    ClientLookupCollectionDto, ClientLookupItemDto, ClientLookupQueryDto, LookupQueryDto,
    TaskCollectionDto, TaskCollectionFiltersDto, TaskCollectionLookupsDto, TaskDetailsDto,
    TaskDetailsTaskDto, TaskEventItemDto, TaskListItemDto, TaskPaginationDto, TaskUserSummaryDto,
    TrackLookupCollectionDto, TrackLookupItemDto, UserLookupCollectionDto, UserLookupItemDto,
};
use crate::dto::main::IndexQuery;
use crate::repository::{
    ClientReader, TaskEventReader, TaskReader, UserListQuery, UserReader, UserWriter,
};
use crate::services::{main as main_service, task as task_service};

/// Returns shell data for authenticated users.
///
/// This endpoint intentionally does not require the `todo` role because the
/// React-owned `/na` page also needs shell data.
pub fn get_shell_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> ServiceResult<IamDto> {
    let navigation = if check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        vec![NavigationItemDto {
            name: "Задачи".to_string(),
            url: "/".to_string(),
        }]
    } else {
        Vec::new()
    };

    Ok(IamDto {
        current_user: CurrentUserDto::from(user.clone()),
        home_url: common_config.auth_service_url.clone(),
        navigation,
        local_menu_items: Vec::new(),
        hub_name: "ToDo".to_string(),
    })
}

/// Returns local page data for the ToDo no-access page.
pub fn get_no_access_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> NoAccessPageDto {
    NoAccessPageDto {
        current_user: CurrentUserDto::from(user.clone()),
        home_url: common_config.auth_service_url.clone(),
        required_role: Some(SERVICE_ACCESS_ROLE.to_string()),
    }
}

/// Build the canonical task-collection API payload for the React list page.
pub fn get_task_collection_data<R>(
    query: IndexQuery,
    user: &AuthenticatedUser,
    repo: &R,
    files_service_url: &str,
) -> ServiceResult<TaskCollectionDto>
where
    R: TaskReader + UserReader + UserWriter + ClientReader + ?Sized,
{
    let collection = main_service::load_task_collection(query, user, repo)?;
    let clients = collection.clients;
    let clients_by_id = clients
        .iter()
        .map(|client| (client.id, client))
        .collect::<HashMap<_, _>>();

    let items = collection
        .items
        .iter()
        .map(|item| {
            let client = item
                .task
                .client_id
                .and_then(|client_id| clients_by_id.get(&client_id).copied());

            TaskListItemDto::from_parts(item, client)
        })
        .collect();

    Ok(TaskCollectionDto {
        items,
        pagination: TaskPaginationDto {
            page: collection.page,
            total_pages: collection.total_pages,
        },
        active_filters: TaskCollectionFiltersDto::from(&collection.filters),
        recently_updated_task_ids: collection
            .recently_updated_task_ids
            .into_iter()
            .map(|id| id.get())
            .collect(),
        lookups: TaskCollectionLookupsDto {
            users: UserLookupCollectionDto {
                items: collection
                    .users
                    .iter()
                    .map(UserLookupItemDto::from)
                    .collect(),
            },
            clients: ClientLookupCollectionDto {
                items: clients.iter().map(ClientLookupItemDto::from).collect(),
            },
            tracks: TrackLookupCollectionDto {
                items: collection
                    .tracks
                    .iter()
                    .map(TrackLookupItemDto::from)
                    .collect(),
            },
        },
        files_service_url: files_service_url.to_string(),
    })
}

/// Build the task-details API payload for the React task page.
pub fn get_task_details_data<R>(
    task_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
    files_service_url: &str,
) -> ServiceResult<TaskDetailsDto>
where
    R: TaskReader + TaskEventReader + UserReader + ClientReader + ?Sized,
{
    let details = task_service::load_task_details(task_id, user, repo)?;

    Ok(TaskDetailsDto {
        task: TaskDetailsTaskDto::from(&details.task),
        author: TaskUserSummaryDto::from(&details.author),
        assignee: details.assignee.as_ref().map(TaskUserSummaryDto::from),
        client: details
            .client
            .as_ref()
            .map(crate::dto::api::TaskClientSummaryDto::from),
        events: details.events.iter().map(TaskEventItemDto::from).collect(),
        files_service_url: files_service_url.to_string(),
    })
}

/// Return minimal user lookup items for React-owned assignee selectors.
pub fn list_users<R>(
    params: LookupQueryDto,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<UserLookupCollectionDto>
where
    R: UserReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = crate::domain::types::HubId::new(user.hub_id)?;
    let query = params
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let list_query = if let Some(query) = query {
        UserListQuery::new(hub_id).search(query.to_string())
    } else {
        UserListQuery::new(hub_id)
    };

    let (_, users) = repo.list_users(list_query)?;

    Ok(UserLookupCollectionDto {
        items: users.iter().map(UserLookupItemDto::from).collect(),
    })
}

/// Return minimal client lookup items for React-owned client selectors.
pub fn list_clients<R>(
    params: ClientLookupQueryDto,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<ClientLookupCollectionDto>
where
    R: ClientReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = crate::domain::types::HubId::new(user.hub_id)?;
    let query = params
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);

    let clients = repo.list_clients(hub_id)?;
    let items = clients
        .iter()
        .filter(|client| {
            query.as_ref().is_none_or(|query| {
                contains_case_insensitive(client.name.as_str(), query)
                    || contains_case_insensitive(client.public_id.as_str(), query)
            })
        })
        .map(ClientLookupItemDto::from)
        .collect();

    Ok(ClientLookupCollectionDto { items })
}

/// Return distinct task tracks for React-owned selectors.
pub fn list_tracks<R>(
    params: LookupQueryDto,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<TrackLookupCollectionDto>
where
    R: TaskReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = crate::domain::types::HubId::new(user.hub_id)?;
    let query = params
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);

    let tracks = repo.list_task_tracks(hub_id)?;
    let items = tracks
        .iter()
        .filter(|track| {
            query
                .as_ref()
                .is_none_or(|query| contains_case_insensitive(track.as_str(), query))
        })
        .map(TrackLookupItemDto::from)
        .collect();

    Ok(TrackLookupCollectionDto { items })
}

fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 7,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    fn common_config() -> CommonServerConfig {
        CommonServerConfig {
            auth_service_url: "https://auth.example.com".to_string(),
            secret: "supersecret".repeat(8),
        }
    }

    #[test]
    fn shell_data_includes_navigation_for_todo_users() {
        let response = get_shell_data(&sample_user(&["todo"]), &common_config())
            .expect("shell data should succeed");

        assert_eq!(response.current_user.email, "user@example.com");
        assert_eq!(response.home_url, "https://auth.example.com");
        assert_eq!(response.hub_name, "ToDo");
        assert_eq!(response.navigation.len(), 1);
        assert_eq!(response.navigation[0].name, "Задачи");
        assert_eq!(response.navigation[0].url, "/");
    }

    #[test]
    fn shell_data_keeps_working_without_todo_role() {
        let response = get_shell_data(&sample_user(&[]), &common_config())
            .expect("shell data should still succeed");

        assert_eq!(response.navigation, Vec::<NavigationItemDto>::new());
        assert_eq!(response.local_menu_items, Vec::<NavigationItemDto>::new());
    }

    #[test]
    fn no_access_data_exposes_required_role() {
        let response = get_no_access_data(&sample_user(&[]), &common_config());

        assert_eq!(response.current_user.name, "Tester");
        assert_eq!(response.home_url, "https://auth.example.com");
        assert_eq!(response.required_role.as_deref(), Some("todo"));
    }
}
