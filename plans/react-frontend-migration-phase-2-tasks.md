# Tasks: React Frontend Migration Phase 2

## Scope
This task file covers only Phase 2 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- introduce the shared React shell and user-menu foundation
- add typed `GET /api/v1/iam` and `GET /api/v1/no-access`
- add a local React-backed `/na` route owned by `pushkind-todo`
- cut over only the no-access page to a built frontend document
- keep `GET /` and `GET /task/{task_id}` on the current Tera path

Do not start Phase 3, Phase 4, or Phase 5 in this file. Phase 2 is complete
only when `/na` is served from a Vite-built document with a React-owned shell,
while the main task list and task details pages still render through the
existing Tera templates.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Phase 1 task file:
  [../plans/react-frontend-migration-phase-1-tasks.md](../plans/react-frontend-migration-phase-1-tasks.md)
- Current backend asset helper:
  [../src/frontend.rs](../src/frontend.rs)
- Current server wiring:
  [../src/lib.rs](../src/lib.rs)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-todo`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 1 is already complete:
  `frontend/` exists,
  Vite builds to `assets/dist/`,
  and `src/frontend.rs` can open built HTML.
- Keep `GET /` and `GET /task/{task_id}` on the current Tera rendering path.
- Do not add task list page-data APIs, task details page-data APIs, or JSON
  mutation endpoints in this phase.
- Do not remove Tera templates, flash-message middleware, or HTMX in this
  phase.

## What You Will Change In Phase 2
You will change only these repository areas:

- create `src/dto/api.rs`
- create `src/services/api.rs`
- create `src/routes/aux.rs`
- edit `src/dto/mod.rs`
- edit `src/services/mod.rs`
- edit `src/routes/mod.rs`
- edit `src/routes/api.rs`
- edit `src/lib.rs`
- create shared shell files under `frontend/src/components/`
- create shared API and shell helper files under `frontend/src/lib/`
- replace only `frontend/src/entries/no-access.tsx`
- create `frontend/src/pages/NoAccessPage.tsx`
- append small shell styles to `frontend/src/styles/foundation.css`
- update the existing frontend toolchain section in `README.md`

If you find yourself editing `src/routes/main.rs`, `src/routes/task.rs`,
`templates/main/index.html`, or `templates/task/index.html`, stop. That is not
Phase 2.

## Deliverables
- `GET /api/v1/iam` exists and returns typed shell data for authenticated
  users.
- `GET /api/v1/no-access` exists and returns typed page data for the local
  no-access page.
- `GET /na` is served from `assets/dist/app/no-access.html` after auth.
- The no-access page uses a React shell with a shared navbar and dropdown.
- Auth menu loading happens after shell data loads, and failure falls back to
  `Домой` and logout.
- `GET /` and `GET /task/{task_id}` still render through Tera.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 2 changes:

```bash
pwd
git status --short
find frontend/src -maxdepth 3 -type f | sort
sed -n '1,220p' src/frontend.rs
sed -n '1,220p' src/routes/api.rs
sed -n '1,180p' README.md
```

Expected result before Phase 2 starts:
- `frontend/` exists from Phase 1
- `frontend/src/entries/no-access.tsx` is still the placeholder entry
- there is no `src/routes/aux.rs`
- there is no `src/dto/api.rs`
- there is no `src/services/api.rs`
- `/na` is still served by the shared `pushkind_common::routes::not_assigned`
  wiring in `src/lib.rs`

## Task 1: Add Typed Shell And No-Access DTOs

### 1.1 Create `src/dto/api.rs`
Create [../src/dto/api.rs](../src/dto/api.rs) with exactly this content:

```rust
//! DTOs exposed by React-owned ToDo API endpoints.

use pushkind_common::domain::auth::AuthenticatedUser;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CurrentUserDto {
    pub email: String,
    pub name: String,
    pub hub_id: i32,
    pub roles: Vec<String>,
}

impl From<&AuthenticatedUser> for CurrentUserDto {
    fn from(user: &AuthenticatedUser) -> Self {
        Self {
            email: user.email.clone(),
            name: user.name.clone(),
            hub_id: user.hub_id,
            roles: user.roles.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct NavigationItemDto {
    pub name: &'static str,
    pub url: &'static str,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct IamDto {
    pub current_user: CurrentUserDto,
    pub home_url: String,
    pub navigation: Vec<NavigationItemDto>,
    pub local_menu_items: Vec<NavigationItemDto>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct NoAccessPageDto {
    pub current_user: CurrentUserDto,
    pub home_url: String,
    pub required_role: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_user_dto_can_be_built_from_authenticated_user() {
        let user = AuthenticatedUser {
            sub: "user-1".into(),
            email: "user@example.com".into(),
            hub_id: 42,
            name: "User".into(),
            roles: vec!["todo".into()],
            exp: 0,
        };

        let dto = CurrentUserDto::from(&user);

        assert_eq!(dto.email, "user@example.com");
        assert_eq!(dto.name, "User");
        assert_eq!(dto.hub_id, 42);
        assert_eq!(dto.roles, vec!["todo".to_string()]);
    }
}
```

### 1.2 Edit `src/dto/mod.rs`
Open [../src/dto/mod.rs](../src/dto/mod.rs).

Find:

```rust
//! DTO module exposing index and task payload helpers used by services and handlers.
#[cfg(feature = "server")]
pub mod main;
#[cfg(feature = "server")]
pub mod task;
pub mod zmq;
```

Change it to:

```rust
//! DTO module exposing API, index, task, and ZMQ payload helpers.
#[cfg(feature = "server")]
pub mod api;
#[cfg(feature = "server")]
pub mod main;
#[cfg(feature = "server")]
pub mod task;
pub mod zmq;
```

## Task 2: Add The Backend Service Layer For Shell And No-Access Data

### 2.1 Create `src/services/api.rs`
Create [../src/services/api.rs](../src/services/api.rs) with exactly this
content:

```rust
//! Service helpers serving shell and no-access data for React-owned pages.

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::check_role;

use crate::dto::api::{CurrentUserDto, IamDto, NavigationItemDto, NoAccessPageDto};
use crate::services::ServiceResult;
use crate::SERVICE_ACCESS_ROLE;

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
            name: "Задачи",
            url: "/",
        }]
    } else {
        Vec::new()
    };

    Ok(IamDto {
        current_user: CurrentUserDto::from(user),
        home_url: common_config.auth_service_url.clone(),
        navigation,
        local_menu_items: Vec::new(),
    })
}

/// Returns local page data for the ToDo no-access page.
pub fn get_no_access_data(
    user: &AuthenticatedUser,
    common_config: &CommonServerConfig,
) -> NoAccessPageDto {
    NoAccessPageDto {
        current_user: CurrentUserDto::from(user),
        home_url: common_config.auth_service_url.clone(),
        required_role: SERVICE_ACCESS_ROLE,
    }
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
        assert_eq!(response.required_role, "todo");
    }
}
```

### 2.2 Edit `src/services/mod.rs`
Open [../src/services/mod.rs](../src/services/mod.rs).

Find:

```rust
//! Service layer root re-exporting shared error helpers and service submodules.
pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

mod notifications;

pub mod main;
pub mod mock;
pub mod task;
```

Change it to:

```rust
//! Service layer root re-exporting shared error helpers and service submodules.
pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

mod notifications;

pub mod api;
pub mod main;
pub mod mock;
pub mod task;
```

## Task 3: Add The Backend Routes For `/na`, `/api/v1/iam`, And `/api/v1/no-access`

### 3.1 Replace `src/routes/api.rs`
Open [../src/routes/api.rs](../src/routes/api.rs).

Replace the entire file with exactly this content:

```rust
//! JSON API routes used for React-owned shell and task data.

use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;

use crate::dto::main::IndexQuery;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, api as api_service, main as main_service};

#[get("/v1/iam")]
/// Return typed shell data for React-owned ToDo pages.
pub async fn api_v1_iam(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    match api_service::get_shell_data(&user, common_config.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(err) => {
            log::error!("Failed to load ToDo shell data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/no-access")]
/// Return typed page data for the React-owned ToDo no-access page.
pub async fn api_v1_no_access(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    HttpResponse::Ok().json(api_service::get_no_access_data(
        &user,
        common_config.get_ref(),
    ))
}

#[get("/v1/tasks")]
/// Return a JSON list of tasks with optional search and pagination.
pub async fn api_v1_tasks(
    params: web::Query<IndexQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match main_service::load_index_page(params.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response.tasks),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list tasks: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

### 3.2 Create `src/routes/aux.rs`
Create [../src/routes/aux.rs](../src/routes/aux.rs) with exactly this content:

```rust
//! Auxiliary routes for React-owned frontend documents.

use std::path::Path;

use actix_web::{HttpRequest, HttpResponse, get};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::frontend::{
    FRONTEND_DIST_DIR, FRONTEND_NO_ACCESS_DOCUMENT, FrontendAssetError, open_frontend_html,
};

#[get("/na")]
pub async fn not_assigned(request: HttpRequest, _user: AuthenticatedUser) -> HttpResponse {
    let no_access_document = Path::new(FRONTEND_DIST_DIR).join(FRONTEND_NO_ACCESS_DOCUMENT);

    match open_frontend_html(&no_access_document).await {
        Ok(file) => file.into_response(&request),
        Err(FrontendAssetError::Read(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::ServiceUnavailable()
                .body("ToDo frontend assets are not built yet. Run `cd frontend && npm run build`.")
        }
        Err(error) => {
            log::error!("Failed to open ToDo no-access frontend document: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
```

### 3.3 Edit `src/routes/mod.rs`
Open [../src/routes/mod.rs](../src/routes/mod.rs).

Find:

```rust
//! Route module combining HTTP handlers for both the UI and JSON API.
pub mod api;
pub mod main;
pub mod task;
```

Change it to:

```rust
//! Route module combining HTTP handlers for both the UI and JSON API.
pub mod api;
pub mod aux;
pub mod main;
pub mod task;
```

### 3.4 Edit `src/lib.rs`
Open [../src/lib.rs](../src/lib.rs).

Make these exact changes.

1. Replace this import:

```rust
#[cfg(feature = "server")]
use pushkind_common::routes::{logout, not_assigned};
```

with:

```rust
#[cfg(feature = "server")]
use pushkind_common::routes::logout;
```

2. Add these imports near the other route imports:

```rust
#[cfg(feature = "server")]
use crate::routes::api::{api_v1_iam, api_v1_no_access};
#[cfg(feature = "server")]
use crate::routes::aux::not_assigned;
```

3. Inside `HttpServer::new(move || { ... })`, delete this line:

```rust
        use crate::routes::api::api_v1_tasks;
```

4. Replace this block:

```rust
            .service(not_assigned)
            .service(web::scope("/api").service(api_v1_tasks))
```

with:

```rust
            .service(not_assigned)
            .service(
                web::scope("/api")
                    .service(api_v1_iam)
                    .service(api_v1_no_access)
                    .service(api_v1_tasks),
            )
```

Do not change anything else in `src/lib.rs` during Phase 2.

### 3.5 Verify The Backend API And `/na` Surface
Run these commands:

```bash
rg -n "api_v1_iam|api_v1_no_access|not_assigned" src
cargo test --all-features services::api::tests -- --nocapture
cargo build --all-features --verbose
```

Expected result:
- `/api/v1/iam` and `/api/v1/no-access` handlers exist
- `src/routes/aux.rs` exists and exports `/na`
- backend compiles before you touch the React no-access page

## Task 4: Add The Shared React Shell Foundation
This shell is introduced on `/na` first. `GET /` and `GET /task/{task_id}`
will reuse it in later phases.

### 4.1 Create `frontend/src/lib/models.ts`
Create [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts) with
exactly this content:

```ts
export type NavigationItem = {
  name: string;
  url: string;
};

export type UserMenuItem = {
  name: string;
  url: string;
  iconClass?: string;
};

export type CurrentUser = {
  email: string;
  name: string;
  hubId: number;
  roles: string[];
};

export type ShellData = {
  currentUser: CurrentUser;
  homeUrl: string;
  navigation: NavigationItem[];
  localMenuItems: UserMenuItem[];
};

export type NoAccessData = {
  currentUser: CurrentUser;
  homeUrl: string;
  requiredRole: string;
};
```

### 4.2 Create `frontend/src/lib/api.ts`
Create [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts) with exactly
this content:

```ts
import type { NoAccessData, NavigationItem, ShellData, UserMenuItem } from "./models";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Invalid API response: expected string at ${key}.`);
  }

  return value;
}

function readNumber(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`Invalid API response: expected number at ${key}.`);
  }

  return value;
}

function readStringArray(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`Invalid API response: expected string[] at ${key}.`);
  }

  return value;
}

function parseNavigationItems(payload: unknown): NavigationItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid navigation payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid navigation item payload.");
    }

    return {
      name: readString(item, "name"),
      url: readString(item, "url"),
    };
  });
}

function parseMenuItems(payload: unknown): UserMenuItem[] {
  if (!Array.isArray(payload)) {
    throw new Error("Invalid menu item payload.");
  }

  return payload.map((item) => {
    if (!isRecord(item)) {
      throw new Error("Invalid menu item payload.");
    }

    return {
      name: readString(item, "name"),
      url: readString(item, "url"),
    };
  });
}

function parseCurrentUser(payload: unknown) {
  if (!isRecord(payload)) {
    throw new Error("Invalid current user payload.");
  }

  return {
    email: readString(payload, "email"),
    name: readString(payload, "name"),
    hubId: readNumber(payload, "hub_id"),
    roles: readStringArray(payload, "roles"),
  };
}

function parseShellData(payload: unknown): ShellData {
  if (!isRecord(payload)) {
    throw new Error("Invalid shell payload.");
  }

  return {
    currentUser: parseCurrentUser(payload.current_user),
    homeUrl: readString(payload, "home_url"),
    navigation: parseNavigationItems(payload.navigation),
    localMenuItems: parseMenuItems(payload.local_menu_items),
  };
}

function parseNoAccessData(payload: unknown): NoAccessData {
  if (!isRecord(payload)) {
    throw new Error("Invalid no-access payload.");
  }

  return {
    currentUser: parseCurrentUser(payload.current_user),
    homeUrl: readString(payload, "home_url"),
    requiredRole: readString(payload, "required_role"),
  };
}

function withBaseUrl(baseUrl: string, path: string) {
  return new URL(path, baseUrl).toString();
}

function isJsonResponse(response: Response): boolean {
  return (
    response.headers.get("content-type")?.includes("application/json") ?? false
  );
}

export const browserLocation = {
  assign(url: string) {
    window.location.assign(url);
  },
};

function handleAuthRedirectResponse(response: Response): never {
  browserLocation.assign(response.url);
  throw new Error("Сессия истекла. Выполняется переход на страницу входа.");
}

function ensureResponseIsNotAuthRedirect(response: Response) {
  if (response.redirected && !isJsonResponse(response)) {
    handleAuthRedirectResponse(response);
  }
}

async function readJsonResponse<T>(response: Response, endpoint: string) {
  if (!isJsonResponse(response)) {
    throw new Error(
      `Expected JSON response from ${endpoint} with status ${response.status}.`,
    );
  }

  return (await response.json()) as T;
}

async function fetchJson(url: string) {
  const response = await fetch(url, {
    headers: {
      Accept: "application/json",
    },
    cache: "no-store",
    credentials: "include",
  });

  if (!response.ok) {
    if (response.status === 401) {
      throw new Error("Недостаточно прав для доступа к ToDo.");
    }

    throw new Error(`Request failed with status ${response.status}.`);
  }

  ensureResponseIsNotAuthRedirect(response);
  return readJsonResponse(response, url);
}

export async function fetchShellData(): Promise<ShellData> {
  const payload = await fetchJson("/api/v1/iam");
  return parseShellData(payload);
}

export async function fetchNoAccessData(): Promise<NoAccessData> {
  const payload = await fetchJson("/api/v1/no-access");
  return parseNoAccessData(payload);
}

export async function fetchHubMenuItems(
  authBaseUrl: string,
  hubId: number,
): Promise<UserMenuItem[]> {
  const payload = await fetchJson(
    withBaseUrl(authBaseUrl, `/api/v1/hubs/${hubId}/menu-items`),
  );
  return parseMenuItems(payload);
}
```

### 4.3 Create `frontend/src/lib/useTodoShell.ts`
Create [../frontend/src/lib/useTodoShell.ts](../frontend/src/lib/useTodoShell.ts)
with exactly this content:

```ts
import { useEffect, useState } from "react";

import { fetchHubMenuItems, fetchShellData } from "./api";
import type { ShellData, UserMenuItem } from "./models";

type TodoShellState =
  | { status: "loading" }
  | {
      status: "ready";
      shell: ShellData;
      authMenuItems: UserMenuItem[];
      authMenuLoaded: boolean;
    }
  | { status: "error"; message: string };

export function useTodoShell(errorMessage: string) {
  const [state, setState] = useState<TodoShellState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    void fetchShellData()
      .then((shell) => {
        if (!active) {
          return;
        }

        setState({
          status: "ready",
          shell,
          authMenuItems: [],
          authMenuLoaded: false,
        });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setState({
          status: "error",
          message: error instanceof Error ? error.message : errorMessage,
        });
      });

    return () => {
      active = false;
    };
  }, [errorMessage]);

  useEffect(() => {
    if (state.status !== "ready" || state.authMenuLoaded) {
      return;
    }

    let active = true;

    void fetchHubMenuItems(state.shell.homeUrl, state.shell.currentUser.hubId)
      .then((authMenuItems) => {
        if (!active) {
          return;
        }

        setState((currentState) => {
          if (currentState.status !== "ready") {
            return currentState;
          }

          return {
            status: "ready",
            shell: currentState.shell,
            authMenuItems,
            authMenuLoaded: true,
          };
        });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        console.warn(
          "Failed to load auth navigation menu. Falling back to local ToDo menu only.",
          error,
        );

        setState((currentState) => {
          if (currentState.status !== "ready") {
            return currentState;
          }

          return {
            status: "ready",
            shell: currentState.shell,
            authMenuItems: currentState.authMenuItems,
            authMenuLoaded: true,
          };
        });
      });

    return () => {
      active = false;
    };
  }, [state]);

  return state;
}
```

### 4.4 Create `frontend/src/components/UserMenuDropdown.tsx`
Create [../frontend/src/components/UserMenuDropdown.tsx](../frontend/src/components/UserMenuDropdown.tsx)
with exactly this content:

```tsx
import type { UserMenuItem } from "../lib/models";

function menuItemIconClass(item: UserMenuItem) {
  if (item.iconClass) {
    return item.iconClass;
  }

  if (item.name === "Главная" || item.name === "Домой") {
    return "bi bi-house";
  }

  if (item.name === "Настройки") {
    return "bi bi-gear";
  }

  return "bi bi-grid";
}

const LOGOUT_ITEM_NAMES = new Set([
  "logout",
  "log out",
  "sign out",
  "signout",
  "выйти",
]);

function normalizedPath(url: string) {
  try {
    return new URL(url, "https://pushkind.local").pathname.replace(/\/+$/, "");
  } catch {
    return undefined;
  }
}

function isLogoutItem(item: UserMenuItem, logoutAction: string) {
  const normalizedName = item.name.trim().toLowerCase();
  if (LOGOUT_ITEM_NAMES.has(normalizedName)) {
    return true;
  }

  return normalizedPath(item.url) === normalizedPath(logoutAction);
}

type UserMenuDropdownProps = {
  currentUserEmail: string;
  localItems: UserMenuItem[];
  fetchedItems: UserMenuItem[];
  logoutAction: string;
};

export function UserMenuDropdown({
  currentUserEmail,
  localItems,
  fetchedItems,
  logoutAction,
}: UserMenuDropdownProps) {
  const visibleLocalItems = localItems.filter(
    (item) => !isLogoutItem(item, logoutAction),
  );
  const visibleFetchedItems = fetchedItems.filter(
    (item) => !isLogoutItem(item, logoutAction),
  );
  const hasNavigationItems =
    visibleLocalItems.length > 0 || visibleFetchedItems.length > 0;

  return (
    <div className="dropdown-center">
      <button
        className="btn btn-link nav-link align-items-center text-muted dropdown-toggle"
        type="button"
        data-bs-toggle="dropdown"
        aria-expanded="false"
      >
        <i className="bi bi-person-circle fs-4" />
      </button>
      <ul className="dropdown-menu dropdown-menu-end">
        <li>
          <h6 className="dropdown-header">{currentUserEmail}</h6>
        </li>
        {hasNavigationItems ? (
          <li>
            <hr className="dropdown-divider" />
          </li>
        ) : null}
        {visibleLocalItems.map((item) => (
          <li key={`local-${item.url}-${item.name}`}>
            <a className="dropdown-item icon-link" href={item.url}>
              <i className={`${menuItemIconClass(item)} mb-2`} />
              {item.name}
            </a>
          </li>
        ))}
        {visibleFetchedItems.map((item) => (
          <li key={`fetched-${item.url}-${item.name}`}>
            <a className="dropdown-item icon-link" href={item.url}>
              <i className={`${menuItemIconClass(item)} mb-2`} />
              {item.name}
            </a>
          </li>
        ))}
        <li>
          <form method="POST" action={logoutAction}>
            <button type="submit" className="dropdown-item icon-link">
              <i className="bi bi-box-arrow-right mb-2" />
              Выйти
            </button>
          </form>
        </li>
      </ul>
    </div>
  );
}
```

### 4.5 Create `frontend/src/components/UserMenuDropdown.test.tsx`
Create [../frontend/src/components/UserMenuDropdown.test.tsx](../frontend/src/components/UserMenuDropdown.test.tsx)
with exactly this content:

```tsx
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { UserMenuDropdown } from "./UserMenuDropdown";

describe("UserMenuDropdown", () => {
  it("renders local items before fetched items and keeps logout last", () => {
    const markup = renderToStaticMarkup(
      <UserMenuDropdown
        currentUserEmail="user@example.com"
        localItems={[
          { name: "Домой", url: "https://auth.example.com" },
          { name: "Локальный пункт", url: "/local" },
        ]}
        fetchedItems={[
          { name: "Внешний пункт", url: "/remote" },
          { name: "Выйти", url: "/logout" },
        ]}
        logoutAction="/logout"
      />,
    );

    expect(markup.indexOf("Домой")).toBeLessThan(
      markup.indexOf("Локальный пункт"),
    );
    expect(markup.indexOf("Локальный пункт")).toBeLessThan(
      markup.indexOf("Внешний пункт"),
    );
    expect(markup.lastIndexOf("Выйти")).toBeGreaterThan(
      markup.indexOf("Внешний пункт"),
    );
  });
});
```

### 4.6 Create `frontend/src/components/TodoNavbar.tsx`
Create [../frontend/src/components/TodoNavbar.tsx](../frontend/src/components/TodoNavbar.tsx)
with exactly this content:

```tsx
import type { ReactNode } from "react";

import { UserMenuDropdown } from "./UserMenuDropdown";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type TodoNavbarProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
};

export function TodoNavbar({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
}: TodoNavbarProps) {
  return (
    <div className="container pt-2">
      <nav className="navbar navbar-expand-sm bg-body-tertiary">
        <div className="container-fluid">
          <a className="navbar-brand" href="/">
            ToDo
          </a>
          <button
            className="navbar-toggler"
            type="button"
            data-bs-toggle="collapse"
            data-bs-target="#todo-foundation-navbar"
            aria-controls="todo-foundation-navbar"
            aria-expanded="false"
            aria-label="Toggle navigation"
          >
            <span className="navbar-toggler-icon" />
          </button>
          <div className="collapse navbar-collapse" id="todo-foundation-navbar">
            <ul className="navbar-nav me-auto">
              {navigation.map((item) => (
                <li className="nav-item" key={item.url}>
                  <a className="nav-link" href={item.url}>
                    {item.name}
                  </a>
                </li>
              ))}
            </ul>
            {search ? <div className="todo-navbar-search">{search}</div> : null}
          </div>
          <div className="ms-sm-2">
            <UserMenuDropdown
              currentUserEmail={currentUserEmail}
              localItems={[{ name: "Домой", url: homeUrl }, ...localMenuItems]}
              fetchedItems={fetchedMenuItems}
              logoutAction="/logout"
            />
          </div>
        </div>
      </nav>
    </div>
  );
}
```

### 4.7 Create `frontend/src/components/TodoShell.tsx`
Create [../frontend/src/components/TodoShell.tsx](../frontend/src/components/TodoShell.tsx)
with exactly this content:

```tsx
import type { ReactNode } from "react";

import { TodoNavbar } from "./TodoNavbar";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type TodoShellProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
  children: ReactNode;
};

export function TodoShell({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
  children,
}: TodoShellProps) {
  return (
    <>
      <TodoNavbar
        navigation={navigation}
        currentUserEmail={currentUserEmail}
        homeUrl={homeUrl}
        localMenuItems={localMenuItems}
        fetchedMenuItems={fetchedMenuItems}
        search={search}
      />
      {children}
    </>
  );
}
```

### 4.8 Create `frontend/src/components/TodoShellFatalState.tsx`
Create [../frontend/src/components/TodoShellFatalState.tsx](../frontend/src/components/TodoShellFatalState.tsx)
with exactly this content:

```tsx
type TodoShellFatalStateProps = {
  message: string;
};

export function TodoShellFatalState({ message }: TodoShellFatalStateProps) {
  return (
    <main className="container py-5">
      <div className="alert alert-danger mb-0" role="alert">
        {message}
      </div>
    </main>
  );
}
```

## Task 5: Replace The No-Access Placeholder With The Real React Page

### 5.1 Create `frontend/src/pages/NoAccessPage.tsx`
Create [../frontend/src/pages/NoAccessPage.tsx](../frontend/src/pages/NoAccessPage.tsx)
with exactly this content:

```tsx
import { useEffect, useState } from "react";

import { fetchNoAccessData } from "../lib/api";
import type { NoAccessData } from "../lib/models";
import { useTodoShell } from "../lib/useTodoShell";
import { TodoShell } from "../components/TodoShell";
import { TodoShellFatalState } from "../components/TodoShellFatalState";

type NoAccessState =
  | { status: "loading" }
  | { status: "ready"; data: NoAccessData }
  | { status: "error"; message: string };

export function NoAccessPage() {
  const shellState = useTodoShell("Не удалось загрузить оболочку ToDo.");
  const [noAccessState, setNoAccessState] = useState<NoAccessState>({
    status: "loading",
  });

  useEffect(() => {
    let active = true;

    void fetchNoAccessData()
      .then((data) => {
        if (!active) {
          return;
        }

        setNoAccessState({ status: "ready", data });
      })
      .catch((error) => {
        if (!active) {
          return;
        }

        setNoAccessState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Не удалось загрузить страницу.",
        });
      });

    return () => {
      active = false;
    };
  }, []);

  if (shellState.status === "loading" || noAccessState.status === "loading") {
    return (
      <main className="container py-5">
        <div className="alert alert-secondary mb-0" role="status">
          Загрузка...
        </div>
      </main>
    );
  }

  if (shellState.status === "error") {
    return <TodoShellFatalState message={shellState.message} />;
  }

  if (noAccessState.status === "error") {
    return (
      <TodoShell
        navigation={shellState.shell.navigation}
        currentUserEmail={shellState.shell.currentUser.email}
        homeUrl={shellState.shell.homeUrl}
        localMenuItems={shellState.shell.localMenuItems}
        fetchedMenuItems={shellState.authMenuItems}
      >
        <main className="container py-5 todo-shell-content">
          <div className="alert alert-danger mb-0" role="alert">
            {noAccessState.message}
          </div>
        </main>
      </TodoShell>
    );
  }

  return (
    <TodoShell
      navigation={shellState.shell.navigation}
      currentUserEmail={shellState.shell.currentUser.email}
      homeUrl={shellState.shell.homeUrl}
      localMenuItems={shellState.shell.localMenuItems}
      fetchedMenuItems={shellState.authMenuItems}
    >
      <main className="container py-5 todo-shell-content">
        <div className="card shadow-sm">
          <div className="card-body p-4">
            <p className="text-uppercase text-secondary small mb-2">ToDo</p>
            <h1 className="h3 mb-3">Недостаточно прав для доступа к сервису</h1>
            <p className="text-secondary mb-3">
              Пользователь <strong>{noAccessState.data.currentUser.name}</strong>{" "}
              не имеет роли <code>{noAccessState.data.requiredRole}</code>.
            </p>
            <p className="text-secondary mb-4">
              Текущий email: <strong>{noAccessState.data.currentUser.email}</strong>
            </p>
            <div className="d-flex flex-column flex-sm-row gap-2">
              <a className="btn btn-primary" href={noAccessState.data.homeUrl}>
                Домой
              </a>
              <form method="POST" action="/logout">
                <button className="btn btn-outline-secondary" type="submit">
                  Выйти
                </button>
              </form>
            </div>
          </div>
        </div>
      </main>
    </TodoShell>
  );
}
```

### 5.2 Replace `frontend/src/entries/no-access.tsx`
Open [../frontend/src/entries/no-access.tsx](../frontend/src/entries/no-access.tsx).

Replace the entire file with exactly this content:

```tsx
import { mountPage } from "../lib/mount";
import { NoAccessPage } from "../pages/NoAccessPage";

mountPage("react-root", <NoAccessPage />);
```

### 5.3 Append Shared Shell Styles
Open [../frontend/src/styles/foundation.css](../frontend/src/styles/foundation.css).

Append these lines to the end of the file:

```css
.todo-navbar-search {
  width: min(100%, 24rem);
}

.todo-shell-content {
  min-height: calc(100vh - 5rem);
}
```

After the edit, the full file should look like this:

```css
:root {
  color-scheme: light;
}

body {
  min-height: 100vh;
}

.phase-one-placeholder {
  min-height: 100vh;
}

.phase-one-code {
  font-family: var(--bs-font-monospace, monospace);
}

.todo-navbar-search {
  width: min(100%, 24rem);
}

.todo-shell-content {
  min-height: calc(100vh - 5rem);
}
```

### 5.4 Keep The Other Frontend Entries Unchanged
Do not edit these files in Phase 2:

- [../frontend/src/entries/index.tsx](../frontend/src/entries/index.tsx)
- [../frontend/src/entries/task.tsx](../frontend/src/entries/task.tsx)

Those stay as Phase 1 placeholders until later phases.

### 5.5 Verify The Frontend Shell And `/na` Entry
Run:

```bash
cd frontend
npm run typecheck
npm run test
npm run build
cd ..
find assets/dist/app -maxdepth 1 -type f | sort
sed -n '1,120p' assets/dist/app/no-access.html
```

Expected result:
- the new dropdown test passes
- the build still emits `assets/dist/app/no-access.html`
- the no-access entry is now backed by the real React page

## Task 6: Update `README.md` For The `/na` Cutover
Phase 2 changes runtime behavior for `/na`, so the frontend toolchain section
must be updated.

### 6.1 Replace The Existing Frontend Toolchain Section
Open [../README.md](../README.md).

Replace the entire existing section that starts with
`### Frontend Toolchain (Phase 1)` and ends immediately before
`### Configuration` with exactly this content:

````md
### Frontend Toolchain (Phase 2)

The React migration now includes a local no-access page served from the
frontend build.

Install frontend dependencies with:

```bash
cd frontend
npm install
```

Build frontend assets with:

```bash
cd frontend
npm run build
```

The frontend build writes compiled HTML, JavaScript, CSS, and
`manifest.json` into `assets/dist/`.

Phase 2 note:

- built frontend assets are required for `/na` and frontend verification
  commands
- `GET /` and `GET /task/{task_id}` still use the current Tera templates
- if `assets/dist/app/no-access.html` is missing, `/na` returns
  `503 Service Unavailable` until you run `cd frontend && npm run build`
- later phases will move the main task pages to built frontend documents too
````

### 6.2 Verify The README Change
Run:

```bash
rg -n "Frontend Toolchain \\(Phase 2\\)|/na|503 Service Unavailable|GET /task" README.md
sed -n '100,150p' README.md
```

Expected result:
- the section title is now `Frontend Toolchain (Phase 2)`
- the README clearly documents that only `/na` has switched to built frontend
  HTML so far

## Task 7: Confirm You Did Not Accidentally Start Later Phases

### 7.1 Confirm Only `/na` Uses Built Frontend HTML
Run:

```bash
rg -n "open_frontend_html|FRONTEND_NO_ACCESS_DOCUMENT|/na" src
```

Expected result:
- `/na` is the only user-facing route using `open_frontend_html`
- there is no built-HTML cutover for `/` or `/task/{task_id}` yet

### 7.2 Confirm The Main Pages Still Use Tera
Run:

```bash
rg -n "render_template\\(|IncomingFlashMessages|Tera" src/routes/main.rs src/routes/task.rs src/lib.rs
```

Expected result:
- the main page and task page still render through Tera
- flash-message middleware is still in place

### 7.3 Confirm No New Task APIs Or JSON Mutations Were Added
Run:

```bash
git diff -- src/routes/api.rs src/routes/main.rs src/routes/task.rs src/services src/dto
```

Expected result:
- only shell/no-access API additions appear
- there is still no `/api/v1/tasks/{task_id}`
- there are still no JSON mutation endpoints in this phase

## Task 8: Full Phase 2 Verification
Run all verification commands in this exact order from the repo root:

```bash
cd frontend
npm run typecheck
npm run test
npm run build
cd ..
cargo build --all-features --verbose
cargo test --all-features --verbose
cargo clippy --all-features --tests -- -Dwarnings
cargo fmt --all -- --check
git status --short
```

### Manual Verification
After the automated commands pass, do this manual check:

1. Run `cargo run`.
2. Sign in as an authenticated user who does **not** have the `todo` role.
3. Open `http://127.0.0.1:8080/na`.
4. Confirm the page is rendered by React rather than the old Tera
   `templates/main/not_assigned.html` output.
5. Confirm the shell navbar renders.
6. Confirm the user dropdown includes `Домой` before any fetched auth menu
   items.
7. Confirm `Выйти` is always the last dropdown action.
8. If the auth menu API fails, confirm the page still works with only `Домой`
   and `Выйти`.
9. Confirm visiting `/` as the same user still follows the current
   authorization behavior and is **not** yet React-backed.

## Expected Repository State After Phase 2
If Phase 2 is done correctly, these new files should exist:

```text
frontend/src/components/
  TodoNavbar.tsx
  TodoShell.tsx
  TodoShellFatalState.tsx
  UserMenuDropdown.test.tsx
  UserMenuDropdown.tsx
frontend/src/lib/
  api.ts
  models.ts
  useTodoShell.ts
frontend/src/pages/
  NoAccessPage.tsx
src/dto/
  api.rs
src/routes/
  aux.rs
src/services/
  api.rs
```

These existing files should now be updated:

```text
frontend/src/entries/no-access.tsx
frontend/src/styles/foundation.css
README.md
src/dto/mod.rs
src/lib.rs
src/routes/api.rs
src/routes/mod.rs
src/services/mod.rs
```

## Phase 2 Exit Checklist
Mark Phase 2 done only if all of the following are true:

- `GET /api/v1/iam` exists and returns typed shell data
- `GET /api/v1/no-access` exists and returns typed no-access page data
- `GET /na` serves `assets/dist/app/no-access.html`
- `/na` works even for authenticated users without the `todo` role
- the React no-access page uses the shared shell
- auth menu loading happens after shell data and fails safely
- `GET /` still renders through Tera
- `GET /task/{task_id}` still renders through Tera
- `README.md` documents that `/na` now depends on built frontend assets

## Explicit Non-Goals For This Task File
Do not do any of the following here:

- switch `GET /` to built frontend HTML
- switch `GET /task/{task_id}` to built frontend HTML
- add `/api/v1/tasks/{task_id}`
- add `/api/v1/users`
- add `/api/v1/clients`
- add `/api/v1/tracks`
- add JSON mutation endpoints
- migrate the task list page to React
- migrate the task details page to React
- remove `tera`
- remove `actix-web-flash-messages`
- remove HTMX
- delete `templates/main/not_assigned.html`
