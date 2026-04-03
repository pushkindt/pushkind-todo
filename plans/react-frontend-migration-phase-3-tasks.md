# Tasks: React Frontend Migration Phase 3

## Scope
This task file covers only Phase 3 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- expand the typed ToDo API surface for task pages and lookup data
- turn `GET /api/v1/tasks` into the canonical React task-collection contract
- add typed `GET /api/v1/tasks/{task_id}`, `GET /api/v1/users`,
  `GET /api/v1/clients`, and `GET /api/v1/tracks`
- add shared JSON mutation response DTOs and route helpers for later React
  mutations
- move Russian validation-copy ownership and field-error extraction into
  `src/forms`
- move `Form -> Payload` conversion to route boundaries for task mutations
- extend the shared frontend API helpers so later phases can consume the new
  contracts

Do not start Phase 4, Phase 5, or Phase 6 in this file. Phase 3 is complete
only when the backend exposes the typed page-data and lookup contracts React
needs, while the live `GET /` and `GET /task/{task_id}` routes still render
through the current Tera templates.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Phase 1 task file:
  [../plans/react-frontend-migration-phase-1-tasks.md](../plans/react-frontend-migration-phase-1-tasks.md)
- Phase 2 task file:
  [../plans/react-frontend-migration-phase-2-tasks.md](../plans/react-frontend-migration-phase-2-tasks.md)
- Current API DTO module:
  [../src/dto/api.rs](../src/dto/api.rs)
- Current forms root:
  [../src/forms/mod.rs](../src/forms/mod.rs)
- Current API routes:
  [../src/routes/api.rs](../src/routes/api.rs)
- Current task-page routes:
  [../src/routes/main.rs](../src/routes/main.rs)
  [../src/routes/task.rs](../src/routes/task.rs)
- Current frontend API helpers:
  [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
  [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-todo`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 2 is already complete:
  `/na` is React-backed,
  `src/dto/api.rs` exists,
  `src/services/api.rs` exists,
  and the shared frontend shell lives under `frontend/src/lib/` and
  `frontend/src/components/`.
- Keep `GET /` and `GET /task/{task_id}` on the current Tera rendering path.
- Keep `/task/{task_id}/modal` as the existing HTML fragment route in this
  phase.
- Do not cut over any new page route to a built frontend document in this
  phase.
- Do not remove Tera, flash-message middleware, or HTMX in this phase.
- Do not add React page components for `/` or `/task/{task_id}` in this phase.
- Prefer ToDo-owned repository data for the new lookup APIs in this phase.
  If you believe a cross-service auth or CRM proxy is required, stop and write
  down that dependency choice explicitly instead of adding it implicitly.

## What You Will Change In Phase 3
You will change only these repository areas:

- edit `src/dto/api.rs`
- edit `src/forms/mod.rs`
- edit `src/forms/main.rs`
- edit `src/forms/task.rs`
- edit `src/services/api.rs`
- edit `src/services/main.rs`
- edit `src/services/task.rs`
- edit `src/routes/mod.rs`
- edit `src/routes/api.rs`
- edit `src/routes/main.rs`
- edit `src/routes/task.rs`
- edit `src/lib.rs`
- edit `frontend/src/lib/models.ts`
- edit `frontend/src/lib/api.ts`
- create `frontend/src/lib/api.test.ts`
- create `tests/api.rs`

If you find yourself editing `frontend/src/entries/*.tsx`,
`frontend/src/pages/*.tsx`, `templates/main/index.html`,
`templates/task/index.html`, or `README.md`, stop. That is not Phase 3.

## Deliverables
- `GET /api/v1/tasks` returns a typed task-collection DTO rather than a raw
  paginated domain list.
- `GET /api/v1/tasks/{task_id}` returns typed task-details data for React.
- `GET /api/v1/users`, `GET /api/v1/clients`, and `GET /api/v1/tracks` return
  typed lookup collections for React-owned forms and selectors.
- `src/dto/api.rs` exposes shared JSON mutation envelope DTOs:
  field errors,
  mutation success,
  mutation failure.
- `src/forms` owns Russian validation-copy and field-level error extraction.
- `src/routes/main.rs` and `src/routes/task.rs` convert forms into payloads
  before calling services.
- `frontend/src/lib/models.ts` and `frontend/src/lib/api.ts` understand the new
  task page-data, lookup, and mutation-envelope contracts.
- `GET /`, `GET /task/{task_id}`, and `/task/{task_id}/modal` still run
  through the current Tera and HTMX flow.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 3 changes:

```bash
pwd
git status --short
sed -n '1,260p' src/dto/api.rs
sed -n '1,260p' src/forms/mod.rs
sed -n '1,260p' src/routes/api.rs
sed -n '1,220p' frontend/src/lib/models.ts
sed -n '1,260p' frontend/src/lib/api.ts
rg -n "#\\[validate|message = \\\"" src/forms
rg -n "api/v1/users|api/v1/clients" templates src/routes
```

Expected result before Phase 3 starts:
- `src/dto/api.rs` only contains shell and no-access DTOs
- `src/forms` does not yet expose a `FormFieldError` helper
- validator annotations do not yet carry Russian `message = "..."`
- `GET /api/v1/tasks` still returns `response.tasks`
- there is no local `GET /api/v1/tasks/{task_id}`
- there are no local `GET /api/v1/users`, `GET /api/v1/clients`, or
  `GET /api/v1/tracks`
- `templates/main/index.html` and `templates/task/index.html` still reference
  auth or CRM lookup endpoints directly

## Task 1: Expand The Shared API DTO Module
Keep React-facing API DTOs in `src/dto/api.rs`. Do not keep growing
`src/dto/main.rs` or `src/dto/task.rs` with React API shapes; those remain the
legacy DTOs used by Tera handlers and existing services.

### 1.1 Edit `src/dto/api.rs`
Expand [../src/dto/api.rs](../src/dto/api.rs) so it contains all React-owned
API DTOs for this crate:

- keep the existing shell DTOs:
  `CurrentUserDto`,
  `NavigationItemDto`,
  `IamDto`,
  `NoAccessPageDto`
- add shared JSON mutation envelope DTOs:
  `ApiFieldErrorDto`,
  `ApiMutationSuccessDto`,
  `ApiMutationErrorDto`
- add `Default` and `From<&FormError>` for `ApiMutationErrorDto`
- add task collection DTOs:
  a task-list item DTO,
  a paginated task-list DTO,
  a task-collection DTO carrying:
  items,
  pagination,
  active filters,
  recently updated task ids,
  lookup collections needed by the list page
- add task-details DTOs:
  task summary/details,
  author summary,
  client summary,
  event item,
  event author,
  full task-details payload
- add lookup DTOs for:
  users,
  clients,
  tracks
- add small query DTOs for the new lookup endpoints if existing query structs
  do not fit cleanly

### 1.2 DTO Shape Rules
Apply these rules to every new DTO:

- do not serialize domain structs directly in the React API surface
- expose enums as stable strings rather than leaking Rust enum internals
- expose date and datetime values as explicit strings
- use plain string or numeric ids that are easy for React to consume
- keep lookup DTOs minimal
- keep page-data DTOs UI-ready and predictable

### 1.3 DTO Tests
Add focused tests in `src/dto/api.rs` that cover:

- `CurrentUserDto` conversion
- `ApiMutationErrorDto::from(&FormError)`
- at least one task-list DTO conversion
- at least one task-details DTO conversion
- at least one lookup DTO conversion

## Task 2: Move Validation Copy And Field Errors Into `src/forms`
Phase 3 is where form-owned validation becomes explicit. Existing user-facing
validation strings must stop living in route branches or implicit validator
defaults.

### 2.1 Edit `src/forms/mod.rs`
Update [../src/forms/mod.rs](../src/forms/mod.rs) to match the field-error
pattern already used by the migrated services:

- add a `FormFieldError` struct containing `field` and `message`
- make `FormError` display localized Russian messages
- add a `field_errors()` helper on `FormError`
- map conversion errors to stable field names
- add tests proving field errors come from validator annotations and
  `FormError` annotations, not route code

### 2.2 Edit `src/forms/main.rs`
Update [../src/forms/main.rs](../src/forms/main.rs):

- add Russian `message = "..."` annotations for every validator-backed field
- keep trimming, sanitization, and typed conversion inside the forms layer
- keep `AddTaskPayload` as the normalized route-to-service boundary object
- introduce a parsed upload payload if needed so the service layer no longer
  depends on multipart form internals

### 2.3 Edit `src/forms/task.rs`
Update [../src/forms/task.rs](../src/forms/task.rs):

- add Russian `message = "..."` annotations for task title, assignee fields,
  client fields, and comment fields
- keep `UpdateTaskPayload`, `TaskCommentPayload`, and
  `QuickTaskStatusPayload` as the normalized boundary payloads
- ensure conversion failures map to stable fields and Russian messages
- add tests for representative invalid forms

### 2.4 Message Ownership Rules
Follow these rules while editing the forms:

- validator messages belong on `#[validate(..., message = "...")]`
- conversion and normalization failures belong on `#[error("...")]`
  annotations in `FormError`
- routes may still choose HTTP status and flash destination, but they must not
  assemble field-level validation text

## Task 3: Move `Form -> Payload` Conversion To Route Boundaries
The current services still accept raw form structs. That blocks the later JSON
mutation work.

### 3.1 Edit `src/services/main.rs`
Refactor [../src/services/main.rs](../src/services/main.rs) so the service layer
accepts normalized payloads instead of Actix forms:

- `add_task` should accept `AddTaskPayload`
- `upload_tasks` should accept a normalized upload payload or parsed task list
- the service must no longer depend on `AddTaskForm` or `UploadTasksForm`

Keep the service layer focused on authorization, repository work,
notifications, and ZeroMQ side effects.

### 3.2 Edit `src/services/task.rs`
Refactor [../src/services/task.rs](../src/services/task.rs) so task mutation
services accept payloads rather than form structs:

- `update_task` should accept `UpdateTaskPayload`
- `transition_task_status` should accept `QuickTaskStatusPayload`
- `add_task_comment` should accept `TaskCommentPayload`

Keep service-owned user-facing messages only for non-field cases that truly
come from business rules, for example:
- duplicate status transitions
- not-found cases
- internal failures

### 3.3 Edit `src/routes/main.rs` And `src/routes/task.rs`
Update the route handlers so they perform `Form -> Payload` conversion before
calling services:

- the HTML routes must keep their current flash and redirect behavior
- form conversion errors should become flash messages using the now-localized
  `FormError`
- no JSON mutation endpoints should be introduced yet

## Task 4: Expand The API Service Layer
The backend already has shell and no-access helpers under `src/services/api.rs`.
Phase 3 expands that module into the React page-data and lookup contract layer.

### 4.1 Edit `src/services/api.rs`
Expand [../src/services/api.rs](../src/services/api.rs) with API-oriented
helpers for:

- shell data
- no-access data
- task collection data
- task details data
- user lookup data
- client lookup data
- track lookup data

### 4.2 Reuse Existing Business Logic
Do not duplicate task business rules in the API service layer.

Prefer this shape:

- reuse `main_service::load_index_page(...)` for collection data
- reuse `task_service::load_task_details(...)` for details data
- reuse repository readers for lookups
- convert legacy service results into the new React API DTOs in
  `services/api.rs`

### 4.3 Lookup Scope
For `/api/v1/users`, `/api/v1/clients`, and `/api/v1/tracks`:

- keep the contract local to `pushkind-todo`
- use Todo-owned repository data by default
- do not silently introduce new outbound HTTP proxy behavior to auth or CRM
- if lookup search is needed, apply repository-backed filtering where available
  and small in-memory filtering only where the repository API does not yet
  justify a new query type

### 4.4 Service Tests
Add tests in `src/services/api.rs` covering:

- authorized task collection serialization
- authorized task details serialization
- unauthorized task collection/details behavior
- user lookup filtering
- client lookup filtering
- track lookup filtering

## Task 5: Expand The API Routes And Shared Mutation Helpers

### 5.1 Edit `src/routes/mod.rs`
Update [../src/routes/mod.rs](../src/routes/mod.rs) so it contains reusable
JSON mutation error helpers for future React mutation endpoints:

- status mapping from `ServiceError`
- shared `ApiMutationErrorDto` construction
- one helper that returns a consistent `HttpResponse`

Do this now so later phases can reuse the same envelope instead of each route
reinventing it.

### 5.2 Edit `src/routes/api.rs`
Expand [../src/routes/api.rs](../src/routes/api.rs) to expose these endpoints:

- `GET /v1/iam`
- `GET /v1/no-access`
- `GET /v1/tasks`
- `GET /v1/tasks/{task_id}`
- `GET /v1/users`
- `GET /v1/clients`
- `GET /v1/tracks`

Route behavior requirements:

- `GET /v1/tasks` uses the new task-collection DTO
- `GET /v1/tasks/{task_id}` returns `404` when the task is missing
- task and lookup routes return `401` for missing ToDo access
- unexpected failures still log and return `500`
- do not add `POST /api/v1/...` mutation routes in this phase

### 5.3 Edit `src/lib.rs`
Update [../src/lib.rs](../src/lib.rs) so the new Phase 3 API handlers are
imported and mounted under the existing `/api` scope.

## Task 6: Extend The Shared Frontend API Layer
Phase 3 does not migrate the index or task page yet, but it must leave the
frontend library able to consume the new contracts as soon as Phase 4 and
Phase 5 start.

### 6.1 Edit `frontend/src/lib/models.ts`
Expand [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts) with
TypeScript types for:

- task list items
- task collection payload
- task details payload
- task event items
- user lookups
- client lookups
- track lookups
- `ApiFieldError`
- `ApiMutationSuccess`
- `ApiMutationError`

Keep the Phase 2 shell and no-access models intact.

### 6.2 Edit `frontend/src/lib/api.ts`
Expand [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts) with parsers and
fetch helpers for:

- `fetchTaskCollection`
- `fetchTaskDetails`
- `fetchUserLookup`
- `fetchClientLookup`
- `fetchTrackLookup`
- `parseApiMutationSuccess`
- `parseApiMutationError`

Keep the existing auth-redirect handling behavior unchanged.

### 6.3 Create `frontend/src/lib/api.test.ts`
Create a focused frontend test file that proves:

- task collection payload parsing works
- task details payload parsing works
- lookup parsing works
- mutation envelope parsing works
- malformed payloads throw explicit errors

### 6.4 Frontend Non-Goal
Do not edit any frontend entrypoint or page component in Phase 3. This phase
prepares contracts only.

## Task 7: Add Backend Contract Coverage
Phase 3 must leave behind real API-contract tests, not just implementation.

### 7.1 Create `tests/api.rs`
Create [../tests/api.rs](../tests/api.rs) and reuse the existing
`tests/common/mod.rs` database harness.

Cover these scenarios:

- task collection data includes tasks, filters, lookups, and recently updated
  ids
- task details data includes author, assignee, client, and ordered events
- user lookup returns predictable typed items
- client lookup returns predictable typed items
- track lookup returns predictable typed items

Prefer calling the API service helpers directly in integration tests unless a
route-level HTTP test is clearly cheap and stable.

### 7.2 Add Route Helper Tests
Add tests to `src/routes/mod.rs` covering:

- mutation status mapping
- unauthorized mapping
- not-found mapping
- internal-error mapping

### 7.3 Confirm `GET /api/v1/tasks` Is No Longer The Raw Legacy Shape
Run:

```bash
rg -n "json\\(response\\.tasks\\)" src/routes/api.rs
```

Expected result:
- there is no remaining `HttpResponse::Ok().json(response.tasks)` in
  `src/routes/api.rs`

## Task 8: Confirm Runtime Behavior Is Still Unchanged
Phase 3 is API and boundary work only. Do not start the page cutovers here.

### 8.1 Confirm Only `/na` Uses Built Frontend HTML
Run:

```bash
rg -n "open_frontend_html|FRONTEND_(INDEX|TASK|NO_ACCESS)_DOCUMENT|/na" src
```

Expected result:
- `/na` remains the only user-facing route served from built frontend HTML
- there is still no built-document cutover for `/` or `/task/{task_id}`

### 8.2 Confirm The Main Pages Still Use Tera And The Modal Is Still HTML
Run:

```bash
rg -n "render_template\\(|task_modal|htmx|taskModalBody" src/routes templates/task
```

Expected result:
- the main page and task page still render via Tera
- `/task/{task_id}/modal` still exists
- the modal flow is still HTML/HTMX-driven in this phase

### 8.3 Confirm No JSON Mutation Endpoints Were Added Yet
Run:

```bash
rg -n "#\\[post\\(\"/v1/" src/routes
git diff -- src/routes/api.rs src/routes/main.rs src/routes/task.rs src/routes/mod.rs
```

Expected result:
- there are no `POST /api/v1/...` routes yet
- changes are limited to page-data GET APIs, route-boundary payload conversion,
  and shared mutation helpers

## Task 9: Full Phase 3 Verification
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

What to confirm after the full verification pass:

- frontend parsing helpers compile and tests pass
- backend DTO, form, and API tests pass
- the crate still builds with all features
- Clippy reports no warnings
- formatting is clean
- the only live route cutover remains `/na`
- no page component migration happened in this phase

## Expected Repository State After Phase 3
If you have done Phase 3 correctly, these new or expanded areas should exist:

```text
frontend/src/lib/
  api.test.ts
  api.ts
  models.ts
src/dto/
  api.rs
src/forms/
  main.rs
  mod.rs
  task.rs
src/routes/
  api.rs
  main.rs
  mod.rs
  task.rs
src/services/
  api.rs
  main.rs
  task.rs
tests/
  api.rs
```

## Phase 3 Exit Checklist
Mark Phase 3 done only if all of the following are true:

- `GET /api/v1/tasks` returns a typed task-collection DTO
- `GET /api/v1/tasks/{task_id}` exists and returns typed task details
- `GET /api/v1/users`, `GET /api/v1/clients`, and `GET /api/v1/tracks` exist
- `src/dto/api.rs` defines the shared mutation success/error DTOs
- `src/forms` owns Russian validation messages and exposes field-level errors
- `src/routes/main.rs` and `src/routes/task.rs` convert forms to payloads
  before calling services
- `frontend/src/lib/models.ts` and `frontend/src/lib/api.ts` understand the new
  task and lookup contracts
- backend contract tests exist for the new API service layer
- `/na` remains the only built-frontend route
- `GET /` and `GET /task/{task_id}` still render through Tera
- `/task/{task_id}/modal` still exists for now
- no JSON mutation endpoints were added yet

## Explicit Non-Goals For This Task File
Do not do any of the following here:

- switch `GET /` to built frontend HTML
- switch `GET /task/{task_id}` to built frontend HTML
- remove `/task/{task_id}/modal`
- add React page components for the task list or task details page
- add `POST /api/v1/...` mutation endpoints
- replace flash-driven HTML routes with JSON mutation routes
- remove Tera
- remove HTMX
- remove `actix-web-flash-messages`
- rewrite the shared shell introduced in Phase 2
- proxy auth or CRM lookups without explicitly documenting that dependency
