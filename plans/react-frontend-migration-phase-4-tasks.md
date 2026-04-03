# Tasks: React Frontend Migration Phase 4

## Scope
This task file covers only Phase 4 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- cut over `GET /` to a React-owned Vite-built HTML document
- migrate the task list page UI from Tera to React
- replace the index page's inline template behavior with React-owned state and
  Bootstrap modal handling
- add structured JSON mutation handling for list-page task creation, CSV upload,
  and any existing list-page-owned status action surface
- keep `GET /task/{task_id}` and `/task/{task_id}/modal` on the current
  Tera/HTMX path

Do not start Phase 5 or Phase 6 in this file. Phase 4 is complete only when
`GET /` is served from `assets/dist/app/index.html`, the task list page is
React-rendered with shareable filter URLs and typed `/api/v1/...` data, and the
task details page still runs through the current Tera route flow.

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
- Phase 3 task file:
  [../plans/react-frontend-migration-phase-3-tasks.md](../plans/react-frontend-migration-phase-3-tasks.md)
- Current frontend HTML helper:
  [../src/frontend.rs](../src/frontend.rs)
- Current index-page routes:
  [../src/routes/main.rs](../src/routes/main.rs)
  [../src/routes/api.rs](../src/routes/api.rs)
- Current task-collection API contracts:
  [../src/dto/api.rs](../src/dto/api.rs)
  [../src/services/api.rs](../src/services/api.rs)
- Current index template and modals:
  [../templates/main/index.html](../templates/main/index.html)
  [../templates/main/add_task_modal.html](../templates/main/add_task_modal.html)
  [../templates/main/filter_task_modal.html](../templates/main/filter_task_modal.html)
- Current React placeholder entry:
  [../frontend/src/entries/index.tsx](../frontend/src/entries/index.tsx)
- Current shared frontend helpers:
  [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)
  [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
  [../frontend/src/lib/useTodoShell.ts](../frontend/src/lib/useTodoShell.ts)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-todo`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 3 is already complete:
  typed `GET /api/v1/tasks`,
  typed task and lookup DTOs,
  shared mutation envelope DTOs,
  route-boundary `Form -> Payload` conversion,
  and the React-backed `/na` page already exist.
- Keep `GET /task/{task_id}` on the current Tera rendering path in this phase.
- Keep `/task/{task_id}/modal` as the current HTML fragment route in this
  phase.
- Keep the existing HTML POST routes in `src/routes/main.rs` and
  `src/routes/task.rs` available unless removing one is strictly required and
  you can prove the task-details page no longer depends on it.
- Do not remove Tera, flash-message middleware, or HTMX in this phase.
- Do not migrate the task details page, edit modal, comment flow, or delete flow
  here.
- Do not invent new visible list-page controls that are not already present on
  the current index page.

## What You Will Change In Phase 4
You will change only these repository areas:

- edit `src/routes/main.rs`
- edit `src/routes/api.rs`
- edit `src/routes/mod.rs`
- edit `src/lib.rs`
- edit `src/dto/api.rs` only if the existing mutation success DTO needs a small
  React-facing extension
- edit `frontend/src/entries/index.tsx`
- create a real task-list page under `frontend/src/pages/`
- create list-page components under `frontend/src/components/`
- edit or add list-page state helpers under `frontend/src/lib/`
- edit `frontend/src/lib/api.ts`
- edit `frontend/src/lib/models.ts`
- edit `frontend/src/lib/api.test.ts`
- add frontend tests for the new index page behavior
- append any required list-page styles to `frontend/src/styles/foundation.css`
- extend `tests/api.rs`

If you find yourself editing `src/routes/task.rs`, `src/services/task.rs`,
`templates/task/index.html`, `templates/task/modal_body.html`, or deleting
`templates/main/index.html`, stop. That is not Phase 4.

## Deliverables
- `GET /` is served from the built frontend document
  `assets/dist/app/index.html` after backend auth checks.
- The React task list page fetches shell data from `GET /api/v1/iam` and task
  collection data from `GET /api/v1/tasks`.
- The React task list page preserves:
  the list layout,
  active-filter badge,
  shareable filter URLs,
  pagination,
  task row navigation,
  add-task modal,
  CSV upload,
  and recently updated highlighting.
- Add-task assignee lookup uses the local `GET /api/v1/users` contract rather
  than direct auth-service calls from template JS.
- `POST /api/v1/tasks`, `POST /api/v1/tasks/upload`, and any actually-needed
  list-page status mutation route return structured JSON success/error
  envelopes.
- `GET /task/{task_id}` still renders through Tera.
- `/task/{task_id}/modal` still exists and remains HTML/HTMX-driven.

## Phase 4 Parity Rule For Quick Status
The current Tera index page does not render explicit inline quick-status
buttons. The main migration plan mentions quick status in Phase 4, but this
phase must still preserve the current UI rather than inventing new visible
controls.

Apply this rule:

- if the live index page already has a list-page-owned status action you are
  replacing, migrate it to a JSON-backed React flow
- if no such action exists on `GET /`, do not add a new visible quick-status UI
  just because the broader plan mentions it
- you may still add the minimal client/helper plumbing needed for an existing
  list-page interaction, but visible task-details status actions remain Phase 5

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 4 changes:

```bash
pwd
git status --short
sed -n '1,260p' src/routes/main.rs
sed -n '1,260p' src/routes/api.rs
sed -n '1,260p' templates/main/index.html
sed -n '1,260p' templates/main/add_task_modal.html
sed -n '1,260p' templates/main/filter_task_modal.html
sed -n '1,220p' frontend/src/entries/index.tsx
rg -n "#\\[post\\(\"/v1/" src/routes
rg -n "render_template\\(|main/index.html|taskModal|filtersModal|TomSelect" src/routes templates/main frontend/src
```

Expected result before Phase 4 starts:
- `frontend/src/entries/index.tsx` still renders the Phase 1 placeholder page
- `GET /` in `src/routes/main.rs` still renders `main/index.html`
- the index template still owns add-task and filters modal markup
- the index template still owns page-specific inline JavaScript
- there are no `POST /api/v1/...` list-page mutation routes yet
- `GET /task/{task_id}` still owns the visible quick-status flow

## Task 1: Cut Over `GET /` To The Built Frontend Document
Phase 4 starts by changing document ownership for the task list page. Rust
still owns auth and access control, but it must stop assembling list-page HTML
through Tera.

### 1.1 Edit `src/routes/main.rs`
Update [../src/routes/main.rs](../src/routes/main.rs):

- `GET /` must stop calling `render_template`
- `GET /` must use the existing frontend helper from `src/frontend.rs` to open
  `FRONTEND_INDEX_DOCUMENT`
- keep the current authorization behavior:
  authorized users get the built HTML document,
  unauthorized users get a flash error and redirect to `/na`
- keep `POST /task/add` and `POST /tasks/upload` in place for now unless you
  have a precise compatibility reason to remove them

### 1.2 Do Not Embed Bootstrap Data Into HTML
Do not reintroduce server-rendered bootstrap payloads for the index page.
React must initialize from the typed APIs already added in Phase 2 and Phase 3:

- `GET /api/v1/iam`
- `GET /api/v1/tasks`
- `GET /api/v1/users`
- `GET /api/v1/clients`
- `GET /api/v1/tracks`

### 1.3 Leave Template Cleanup For Phase 6
After `GET /` switches to the built document:

- leave `templates/main/index.html` and its partials in the repository
- do not delete or heavily rewrite them here
- do not remove Tera from runtime dependencies yet

## Task 2: Add React-Owned JSON Mutation Endpoints For The List Page
The list page can no longer rely on flash-driven redirects once `GET /` is a
React document. Phase 4 introduces JSON responses for the list-page-owned
mutations while leaving the old HTML POST routes available for anything still
on the Tera path.

### 2.1 Add API Mutation Routes In `src/routes/api.rs`
Expand [../src/routes/api.rs](../src/routes/api.rs) with the list-page mutation
surface:

- `POST /v1/tasks` for add-task
- `POST /v1/tasks/upload` for CSV upload
- only add a list-page status mutation route if you can point to a real list
  interaction being migrated in this phase

### 2.2 Reuse Existing Forms And Services
Do not invent parallel JSON request DTOs just to satisfy React. Prefer request
shapes that let the existing forms continue to own validation copy:

- `POST /v1/tasks` may accept `application/x-www-form-urlencoded` or
  `FormData` matching `AddTaskForm`
- `POST /v1/tasks/upload` should accept `multipart/form-data` matching
  `UploadTasksForm`
- any list-page status mutation route should reuse `QuickTaskStatusForm`

At the route boundary:

- convert request form data into the existing Phase 3 payload objects
- call the existing service functions in `src/services/main.rs` and
  `src/services/task.rs`
- return JSON using the shared mutation envelope helpers from `src/routes/mod.rs`

### 2.3 Mutation Response Rules
Use the shared mutation envelope shape rather than inventing route-specific
error formats:

- success responses should use `ApiMutationSuccessDto`
- failure responses should use `mutation_error_response(...)`
- keep status mapping consistent with `mutation_error_status(...)`
- if the existing `ApiMutationSuccessDto` is sufficient, do not create a second
  success-envelope family
- prefer frontend collection refetch after success over complex optimistic
  mutation payload design

### 2.4 API Route Mounting
Update [../src/lib.rs](../src/lib.rs) so the new list-page mutation handlers are
mounted under the existing `/api` scope.

## Task 3: Extend The Frontend API Layer For List-Page Actions
The React list page needs typed read and write helpers, but it should stay thin.
Push transport details into `frontend/src/lib/`.

### 3.1 Edit `frontend/src/lib/models.ts`
Expand [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts) only as
needed for Phase 4:

- keep the Phase 2 shell models
- keep the Phase 3 task collection and mutation envelope models
- add only small supporting types required for list-page form state or mutation
  results

Do not add task-details-page UI models here yet unless they are already needed
by existing code.

### 3.2 Edit `frontend/src/lib/api.ts`
Expand [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts) with
list-page-owned mutation helpers:

- `createTask(...)`
- `uploadTasks(...)`
- add a status mutation helper only if the list page truly uses it in Phase 4

Implementation rules:

- keep `credentials: "include"`
- preserve the existing unauthorized handling pattern
- use `URLSearchParams` or `FormData` where that keeps the request aligned with
  `src/forms`
- parse success and error envelopes consistently
- make the helpers easy for page components to call without reimplementing HTTP
  details

### 3.3 Add URL State Helpers Under `frontend/src/lib/`
Create a small helper module under `frontend/src/lib/` for task-list URL state.
This helper should:

- parse `window.location.search` into the filter/query shape used by the list
  page
- serialize filter state back into a stable query string
- preserve shareable URLs
- keep `page`, `status`, `track`, `assignee`, `client`, `priority`,
  `updated_after`, `updated_before`, `public_id`, and `search` handling aligned
  with `IndexQuery`

Do not add `react-router` or client-side route ownership.

## Task 4: Replace The Placeholder Entry With A Real React Task List Page
Phase 4 replaces the placeholder entry for `/` with a real page component.

### 4.1 Replace `frontend/src/entries/index.tsx`
Open [../frontend/src/entries/index.tsx](../frontend/src/entries/index.tsx).

Replace the Phase 1 placeholder mount with the real list page mount.

This entry should mount a React page that:

- uses the shared shell from Phase 2
- loads shell data and task-collection data
- handles loading, empty, and fatal states
- does not depend on server-rendered HTML fragments

### 4.2 Create A Task List Page Under `frontend/src/pages/`
Create a real page component under `frontend/src/pages/`, for example
`TaskListPage.tsx`.

Page responsibilities:

- fetch shell data via the shared shell hook
- fetch task collection data from `GET /api/v1/tasks`
- react to URL query-string changes
- update browser history when filters or pagination change
- refetch collection data after successful create/upload mutations
- keep navigation to `/task/{task_id}` as a normal server route

### 4.3 Recently Updated Highlighting
Move the current `.task-recent` behavior out of the template and into the React
page plus frontend CSS:

- preserve the visual highlight for recently updated task ids
- do not silently drop this behavior during the cutover

## Task 5: Rebuild The Index Page UI In React With Bootstrap Parity
The React page must preserve the current layout and Russian copy rather than
introducing a redesign.

### 5.1 Create List-Page Components Under `frontend/src/components/`
Create only the components needed to keep the page maintainable, for example:

- task-list layout or toolbar component
- task row/card component
- filters modal component
- add-task modal component
- upload section component if it keeps the modal code readable

Do not over-componentize the page into dozens of one-off wrappers.

### 5.2 Preserve The Current Index Layout
The React page should preserve the current user-facing shape from
`templates/main/index.html`:

- add-task affordance at the top
- filters button with active badge
- task rows showing track, title, updated-at timestamp, assignee, description,
  status, and priority
- empty state when there are no tasks
- pagination controls

### 5.3 Rebuild The Filters Modal In React
Replace the template-owned filter modal with a React-owned Bootstrap modal that
preserves:

- the existing field set
- the current Russian labels
- apply and reset behavior
- shareable URLs after apply/reset
- the active-filter badge behavior

Do not add new filter fields in this phase.

### 5.4 Rebuild The Add-Task Modal In React
Replace the template-owned add-task modal with a React-owned Bootstrap modal.

Preserve:

- title, track, priority, description, and assignee inputs
- the current Russian copy
- CSV upload in the modal footer
- the current track suggestions behavior using local lookup data where needed

For assignee selection:

- stop calling the auth service directly from the browser
- use the local ToDo lookup contract under `GET /api/v1/users`
- keep the final submitted field names aligned with `AddTaskForm`

### 5.5 Preserve Existing Query-Param Modal Behavior If Still Needed
The current index template reopens the add-task modal when `name` and `email`
appear in the query string.

Phase 4 must make an explicit choice:

- either preserve that behavior in React
- or document why it is dead code and remove it intentionally

Do not drop it by accident.

## Task 6: Remove Template-Owned Index Behavior Without Touching The Task Page
Once the React list page is in place, the old index template must no longer own
live behavior.

### 6.1 Confirm `GET /` No Longer Uses `main/index.html`
Run:

```bash
rg -n "render_template\\(|main/index.html" src/routes/main.rs
```

Expected result:
- `GET /` no longer renders `main/index.html`

### 6.2 Confirm The Placeholder Entry Is Gone
Run:

```bash
sed -n '1,220p' frontend/src/entries/index.tsx
rg -n "PhaseOnePlaceholderPage|GET / пока остаётся на Tera" frontend/src
```

Expected result:
- the index entry mounts the real task-list page
- the placeholder remains only where still needed for `/task/{task_id}` or
  related unfinished routes

### 6.3 Confirm The Task Page Is Still Legacy-Owned
Run:

```bash
rg -n "render_template\\(|task_modal|/task/\\{task_id\\}/status|modal_body.html" src/routes/task.rs templates/task
```

Expected result:
- the task page still renders through Tera
- the task modal route still exists
- task-details quick status remains on the legacy path for now

## Task 7: Add Verification For The New List-Page Contract And UI
Phase 4 is the first full page cutover. It needs both backend and frontend
coverage.

### 7.1 Extend `tests/api.rs`
Update [../tests/api.rs](../tests/api.rs) to cover the new mutation-contract
surface at least at the route or helper level:

- add-task mutation success shape
- add-task validation error shape
- upload mutation success shape
- upload parse/validation error shape
- unauthorized mutation behavior
- not-found behavior for any list-page status route you actually add

Prefer stable tests over overly broad end-to-end route harnesses.

### 7.2 Add Frontend Tests For The List Page
Add focused frontend tests, for example under `frontend/src/pages/` or
`frontend/src/components/`, that cover:

- task collection rendering
- empty state rendering
- filter badge visibility
- query-string parse/serialize behavior
- add-task success and validation-error handling
- upload success and validation-error handling

If you keep the `name`/`email` modal-prefill behavior, test it.

### 7.3 Keep `frontend/src/lib/api.test.ts` Up To Date
Extend [../frontend/src/lib/api.test.ts](../frontend/src/lib/api.test.ts) for
the new mutation helpers so malformed success/error payloads fail loudly.

## Task 8: Full Phase 4 Verification
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

- the React index page builds and mounts from the real entrypoint
- frontend tests cover the list-page URL and mutation behavior
- backend tests cover the new list-page mutation response shapes
- `GET /` is now the built frontend document
- `GET /task/{task_id}` is still the legacy Tera page
- there is still no React replacement for `/task/{task_id}/modal`

## Expected Repository State After Phase 4
If you have done Phase 4 correctly, these new or expanded areas should exist:

```text
frontend/src/components/
  ...
frontend/src/entries/
  index.tsx
frontend/src/lib/
  api.test.ts
  api.ts
  models.ts
  ...
frontend/src/pages/
  TaskListPage.tsx
  ...
src/routes/
  api.rs
  main.rs
  mod.rs
tests/
  api.rs
```

## Phase 4 Exit Checklist
Mark Phase 4 done only if all of the following are true:

- `GET /` serves the built frontend document after backend auth checks
- the React task list page initializes from typed `/api/v1/iam` and
  `/api/v1/tasks`
- the React task list page preserves list rendering, filter badge, pagination,
  task navigation, add-task modal, CSV upload, and recently updated highlighting
- the list page no longer depends on template-owned inline JavaScript
- assignee lookup on the add-task flow no longer calls the auth service
  directly from the browser
- `POST /api/v1/tasks` and `POST /api/v1/tasks/upload` return structured JSON
  envelopes
- `ApiMutationSuccessDto` and `ApiMutationErrorDto` are reused rather than
  bypassed
- `GET /task/{task_id}` still renders through Tera
- `/task/{task_id}/modal` still exists
- the task details comment, edit, delete, and modal flows are untouched

## Explicit Non-Goals For This Task File
Do not do any of the following here:

- switch `GET /task/{task_id}` to built frontend HTML
- replace `/task/{task_id}/modal` with JSON
- migrate task-details edit, comment, delete, or full quick-status flows
- remove Tera from the crate
- remove `actix-web-flash-messages`
- remove HTMX
- delete `templates/main/index.html` or task templates
- add client-side routing
- add direct browser calls to auth or CRM services for task-list lookups
- redesign the task list page instead of preserving the current Bootstrap UI
