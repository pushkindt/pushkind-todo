# pushkind-todo: Specification

## Goals

- Provide a hub-scoped task tracker (“todo”) for Pushkind hubs.
- Offer server-rendered UI for browsing, creating, and updating tasks.
- Record an auditable task event stream (comments, status/assignment/metadata changes).
- Support bulk task creation via CSV upload.
- Publish outbound email notifications (via ZeroMQ) for task creation, updates, and comments.
- Enforce authentication + authorization via `pushkind_common` with role gating (`SERVICE_ACCESS_ROLE = "todo"`).

## Non-goals

- Public, unauthenticated access to data.
- Cross-hub data access; all reads/writes are scoped to a hub.
- Rich workflow engine (e.g., configurable state machines beyond the fixed status enum).
- Full “assignment history” feature: the `task_assignments` table and repository APIs exist, but the current service layer does not populate it.
- Hard guarantees about `completed_at`:
  - Quick status transitions set/clear `completed_at`.
  - Full “edit task” updates can set status to `Completed` without setting `completed_at` (current behaviour).

## Domain model

### Core entities

- `User` (`src/domain/user.rs`)
  - Fields: `id`, `hub_id`, `name`, `email`, `visited_at`
  - `NewUser` is derived from `pushkind_common::domain::auth::AuthenticatedUser`.
- `Task` (`src/domain/task.rs`)
  - Fields: `id`, `hub_id`, `title`, `description?`, `track?`, `priority`, `status`, `due_date?`,
    `assigned_to?`, `author_id`, `created_at`, `updated_at`, `completed_at?`
  - `TaskStatus`: `Pending | InProgress | Blocked | Completed | Archived`
  - `TaskPriority`: `Low | Middle | High`
- `TaskEvent` (`src/domain/task_event.rs`)
  - Fields: `id`, `task_id`, `user_id?`, `event_type`, `event_data` (JSON), `created_at`
  - `TaskEventType`: `Comment | StatusChanged | AssignmentChanged | MetadataUpdated`

### Value objects (type constraints)

Defined in `src/domain/types.rs`:

- Positive identifiers: `UserId`, `HubId`, `TaskId`, `TaskEventId` (`i32 > 0`).
- `UserEmail`: trimmed + lowercased and validated as an email.
- Non-empty strings (trimmed): `UserName`, `TaskTitle`, `TaskTrack`, `SearchTerm`.
- `TaskDescription`: HTML content (sanitized at the form boundary; domain wrapper itself does not validate).

## Invariants

### Hub scoping

- Every query is scoped by `hub_id` (from `AuthenticatedUser`).
- Database tables carry `hub_id` columns (`users`, `tasks`, `task_assignments`), and repository queries filter by hub.

### Authorization

- All service entrypoints call `pushkind_common::routes::ensure_role(user, SERVICE_ACCESS_ROLE)`.
- `SERVICE_ACCESS_ROLE` is `"todo"` (`src/lib.rs`).

### Persistence constraints

- `tasks.status` has a DB check constraint for the known statuses (`migrations/2025-10-01-094600_create-tasks/up.sql`).
- `task_events.event_type` has a DB check constraint for known event types (`migrations/2025-10-01-094700_create-task-events/up.sql`).
- `users` is unique on `(hub_id, email)` (`migrations/2025-10-01-094500_create-users/up.sql`).
- `tasks.author_id` is a foreign key referencing `users(id)` with `ON DELETE RESTRICT`.
- `tasks.assigned_to` references `users(id)` with `ON DELETE SET NULL`.
- When a task event is recorded, repository logic updates the parent task’s `updated_at` to the event timestamp (`src/repository/task_event.rs`).

### Sanitization and validation boundaries

- Forms validate “presence” constraints with `validator` (e.g., title/comment length ≥ 1).
- User-supplied HTML is sanitized using `ammonia`:
  - Task descriptions on create/update.
  - Task comments.
  - Quick status comment.

### Task list filtering defaults

- The index list hides “terminal” tasks by default at service level:
  - `load_index_page` sets `hide_terminal_statuses = true` initially (`src/services/main.rs`).
  - Repository interprets “hide terminal” as excluding `Completed`, `Archived`, and `Blocked` (`src/repository/task.rs`).

## API contracts

### Authentication / identity

- Handlers rely on `AuthenticatedUser` extractor from `pushkind_common` and a `RedirectUnauthorized` middleware is applied to the mounted UI scope (`src/lib.rs`).
- Session, identity, and flash messages are cookie-backed (`src/lib.rs`).

### UI (server-rendered) endpoints

All mounted under `web::scope("").wrap(RedirectUnauthorized)` (`src/lib.rs`).

- `GET /` (`src/routes/main.rs:show_index`)
  - Query params: `IndexQuery` (`src/dto/main.rs`) — `search`, `page`, `status`, `track`, `assignee`, `priority`, `updated_after`, `updated_before`.
  - Response: HTML (`templates/main/index.html`), or redirect to `/na` on authorization failure.
- `POST /task/add` (`src/routes/main.rs:add_task`)
  - Form body: `AddTaskForm` (`src/forms/main.rs`) — `title`, `message`, `track`, `priority`, optional assignee fields (`name`, `email`).
  - Response: redirect to `/` with flash message.
- `POST /tasks/upload` (`src/routes/main.rs:tasks_upload`)
  - Multipart body: `UploadTasksForm` with `csv` file up to 10MB.
  - CSV schema: headers `title,description` (both optional; empty/missing title rows are skipped).
  - Response: redirect to `/` with flash message.
- `GET /task/{task_id}` (`src/routes/task.rs:show_task`)
  - Response: HTML (`templates/task/index.html`), `404` if task not found (within hub).
- `POST /task/{task_id}/modal` (`src/routes/task.rs:task_modal`)
  - Response: HTML fragment (`templates/task/modal_body.html`) intended for AJAX modal rendering.
  - Error mapping: `401` for unauthorized; “not found” currently returns `500` (route-level behaviour).
- `POST /task/{task_id}/update` (`src/routes/task.rs:update_task`)
  - Form body: `UpdateTaskForm` (`src/forms/task.rs`) — `title`, `message`, `due_date` (YYYY-MM-DD), `status`, `track`, `priority`, optional assignee fields.
  - Response: redirect to `/task/{id}` with flash message.
  - Side effects:
    - Updates task fields.
    - Records `TaskEvent` entries for `StatusChanged`, `AssignmentChanged`, and/or `MetadataUpdated` when applicable (`src/services/task.rs`).
- `POST /task/{task_id}/status` (`src/routes/task.rs:quick_update_task_status`)
  - Form body: `QuickTaskStatusForm` (`src/forms/task.rs`) — `status`, `comment?`, `assign_self` (bool).
  - Response: redirect to `/task/{id}` with flash message.
  - Side effects:
    - Changes status and, when `status == Completed`, sets `completed_at`; otherwise clears it.
    - When `assign_self == true` and `status == InProgress`, assigns the current user.
    - Records `StatusChanged` / `AssignmentChanged` events; optionally records a `Comment` event if `comment` is non-empty after sanitization.
- `POST /task/{task_id}/comments` (`src/routes/task.rs:add_task_comment`)
  - Form body: `NewTaskCommentForm` (`src/forms/task.rs`) — `message`.
  - Response: redirect to `/task/{id}` with flash message.
  - Side effects:
    - Records a `Comment` event.
    - Notifies task author, assignee, and prior event actors (excluding the current actor).
- `POST /task/{task_id}/delete` (`src/routes/task.rs:delete_task`)
  - Response: redirect to `/` with flash message.
  - Semantics: deletes the task row; cascades to `task_events` and `task_assignments` via DB foreign keys.

### JSON API endpoints (present in code)

The codebase defines a JSON API handler but it is not currently mounted in `src/lib.rs`.

- `GET /v1/tasks` (`src/routes/api.rs:api_v1_tasks`)
  - Query params: `IndexQuery` (same as `GET /`).
  - Response: `200` JSON serialization of `pushkind_common::pagination::Paginated<IndexTask>`.
  - Error mapping: `401` if missing role; `500` on unexpected failures.

## Error semantics

### Service layer (`ServiceError`)

Services return `pushkind_common::services::errors::ServiceResult<T>`.
Observed behaviour in this crate:

- `Unauthorized`: user lacks `SERVICE_ACCESS_ROLE`.
- `NotFound`: requested task is absent (hub-scoped lookup).
- `Form(String)`: validation or parsing failure for user input (message is user-facing and typically in Russian).
- `Internal`: reserved for unexpected invariants (e.g., authenticated user cannot be mapped into a valid domain user).
- `Repository(RepositoryError)` and `TypeConstraint(String)` may be produced via conversions (`src/error_conversions.rs`) and repository mapping.

### Repository layer (`RepositoryError`)

Repository operations return `pushkind_common::repository::errors::RepositoryResult<T>`.
Observed mappings:

- `RepositoryError::NotFound` is translated to `ServiceError::NotFound` for task update/delete flows.
- `RepositoryError::ValidationError` can be produced when a persisted `task_events.event_data` cannot be serialized or violates model constraints (`src/repository/task_event.rs`).
- Other repository errors are treated as internal/unexpected at the route layer and typically surface as `500`.

### HTTP layer mapping (routes)

Routes translate `ServiceError` to HTTP/UX outcomes:

- UI routes:
  - `Unauthorized` → flash “Недостаточно прав.” + redirect to `/na`.
  - `Form(msg)` → flash `msg` + redirect back (usually `/` or `/task/{id}`).
  - `NotFound` → either `404` (task page) or flash + redirect (update/delete/status/comment routes).
  - Other errors → log + `500` or flash generic error + redirect.
- JSON route:
  - `Unauthorized` → `401`.
  - Other errors → `500`.
