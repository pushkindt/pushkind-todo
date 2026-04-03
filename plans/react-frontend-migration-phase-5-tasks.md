# Tasks: React Frontend Migration Phase 5

## Scope
This task file covers only Phase 5 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- cut over `GET /task/{task_id}` to a React-owned Vite-built HTML document
- migrate the task details page UI from Tera to React
- replace the task page's HTMX edit-modal fragment flow with a typed React data
  flow
- add structured JSON mutation handling for task update, quick status, comment,
  and delete actions
- keep cleanup and dependency removal for Phase 6

Do not start Phase 6 in this file. Phase 5 is complete only when
`GET /task/{task_id}` is served from `assets/dist/app/task.html`, the visible
task page behavior is React-owned, and the live user flow no longer depends on
`/task/{task_id}/modal` returning HTML.

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
- Phase 4 task file:
  [../plans/react-frontend-migration-phase-4-tasks.md](../plans/react-frontend-migration-phase-4-tasks.md)
- Current frontend HTML helper:
  [../src/frontend.rs](../src/frontend.rs)
- Current task-page routes:
  [../src/routes/task.rs](../src/routes/task.rs)
  [../src/routes/api.rs](../src/routes/api.rs)
  [../src/routes/mod.rs](../src/routes/mod.rs)
- Current task service and API DTOs:
  [../src/services/task.rs](../src/services/task.rs)
  [../src/services/api.rs](../src/services/api.rs)
  [../src/dto/api.rs](../src/dto/api.rs)
- Current task templates:
  [../templates/task/index.html](../templates/task/index.html)
  [../templates/task/modal_body.html](../templates/task/modal_body.html)
- Current React task entry and shared frontend helpers:
  [../frontend/src/entries/task.tsx](../frontend/src/entries/task.tsx)
  [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts)
  [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts)
  [../frontend/src/lib/markdown.ts](../frontend/src/lib/markdown.ts)
  [../frontend/src/components/TodoModal.tsx](../frontend/src/components/TodoModal.tsx)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-todo`.
- Treat the feature spec and migration plan as the source of truth.
- Assume Phase 4 is already complete:
  `GET /` is React-backed,
  task-list JSON mutations exist,
  and the shared React shell/components already live under
  `frontend/src/components/`, `frontend/src/lib/`, and `frontend/src/pages/`.
- Assume Phase 3 is already complete:
  typed `GET /api/v1/tasks/{task_id}`,
  typed lookup endpoints,
  shared mutation envelopes,
  and route-boundary `Form -> Payload` conversion already exist.
- Keep cleanup work for Phase 6:
  do not remove Tera, flash-message middleware, or HTMX globally here.
- Keep the existing templates in the repository even after the route cutover.
- Prefer React initialization from typed APIs:
  `GET /api/v1/iam`,
  `GET /api/v1/tasks/{task_id}`,
  `GET /api/v1/users`,
  `GET /api/v1/clients`,
  and `GET /api/v1/tracks`.
- Do not reintroduce server-rendered task bootstrap payloads into the built
  HTML document.
- Preserve the current visible task-page behavior unless the main migration
  plan explicitly requires a better React-native replacement.

## What You Will Change In Phase 5
You will change only these repository areas:

- edit `src/routes/task.rs`
- edit `src/routes/api.rs`
- edit `src/routes/mod.rs`
- edit `src/lib.rs`
- edit `src/dto/api.rs` only if the existing task-details or mutation DTOs need
  a small React-facing extension for the edit flow
- edit `frontend/src/entries/task.tsx`
- create a real task-details page under `frontend/src/pages/`
- create task-details components under `frontend/src/components/`
- edit or add task-details state helpers under `frontend/src/lib/`
- edit `frontend/src/lib/api.ts`
- edit `frontend/src/lib/models.ts`
- edit `frontend/src/lib/api.test.ts`
- add frontend tests for the new task-page behavior
- append any required task-page styles to `frontend/src/styles/foundation.css`
- extend `tests/api.rs`

If you find yourself deleting `templates/task/index.html`,
`templates/task/modal_body.html`, removing `tera` or flash-message middleware
from the backend, or doing general dependency cleanup, stop. That is Phase 6.

## Deliverables
- `GET /task/{task_id}` is served from the built frontend document
  `assets/dist/app/task.html` after backend auth checks.
- The React task page fetches shell data from `GET /api/v1/iam` and task
  details from `GET /api/v1/tasks/{task_id}`.
- The React task page preserves the current task-details surface:
  metadata,
  description,
  author and assignee display,
  client link,
  quick status flow,
  edit flow,
  delete flow,
  comment composer,
  and event timeline.
- The edit flow no longer depends on `/task/{task_id}/modal` returning HTML.
- Task edit lookups use the local typed lookup contracts from Phase 3 rather
  than direct browser-side auth or CRM fetches embedded in template JS.
- Task update, quick status, comment, and delete actions have JSON endpoints
  that return the shared mutation success/error envelopes.
- `GET /` remains React-backed from Phase 4.
- `/task/{task_id}/modal` is no longer required for user-facing behavior.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 5 changes:

```bash
pwd
git status --short
sed -n '1,280p' src/routes/task.rs
sed -n '1,320p' src/routes/api.rs
sed -n '1,260p' templates/task/index.html
sed -n '1,220p' templates/task/modal_body.html
sed -n '1,220p' frontend/src/entries/task.tsx
rg -n "#\\[get\\(\"/task|#\\[post\\(\"/task|#\\[post\\(\"/v1/tasks/.+(update|status|comments|delete)" src/routes
rg -n "/task/\\{task_id\\}/modal|hx-post|taskModalBody|completeTaskModal|htmx:|TomSelect" templates/task frontend/src src/routes
```

Expected result before Phase 5 starts:
- `frontend/src/entries/task.tsx` still renders the Phase 1 placeholder page
- `GET /task/{task_id}` in `src/routes/task.rs` still renders
  `task/index.html`
- `POST /task/{task_id}/modal` still renders `task/modal_body.html`
- task update, quick status, comment, and delete routes still return
  flash-driven redirects rather than JSON
- there are no task-details JSON mutation endpoints yet
- the task template still owns the page-specific HTMX and TomSelect behavior

## Task 1: Cut Over `GET /task/{task_id}` To The Built Frontend Document
Phase 5 starts by changing document ownership for the task-details page. Rust
still owns auth and access control, but it must stop assembling task-page HTML
through Tera.

### 1.1 Edit `src/routes/task.rs`
Update [../src/routes/task.rs](../src/routes/task.rs):

- `GET /task/{task_id}` must stop calling `render_template`
- `GET /task/{task_id}` must use the existing frontend helper from
  `src/frontend.rs` to open `FRONTEND_TASK_DOCUMENT`
- keep the current authorization behavior:
  authorized users get the built HTML document,
  unauthorized users get a flash error and redirect to `/na`
- preserve task-not-found behavior if you can do so without server-rendering
  the page payload; if that is not practical, make the React page render a
  proper not-found state from the typed API instead of falling back to Tera

### 1.2 Do Not Embed Task Bootstrap Data Into HTML
React must initialize from the typed APIs already added in earlier phases:

- `GET /api/v1/iam`
- `GET /api/v1/tasks/{task_id}`
- the existing lookup endpoints as needed by the edit flow

Do not add task JSON blobs to the built HTML document.

### 1.3 Leave Template Cleanup For Phase 6
After `GET /task/{task_id}` switches to the built document:

- leave `templates/task/index.html` and `templates/task/modal_body.html` in the
  repository
- do not delete or heavily rewrite them here
- do not remove Tera from runtime dependencies yet

## Task 2: Add React-Owned JSON Endpoints For Task-Details Actions
Once the task page is React-backed, it can no longer depend on flash-driven
redirects or HTML fragments for the live user flow.

### 2.1 Add Task-Details Mutation Routes In `src/routes/api.rs`
Expand [../src/routes/api.rs](../src/routes/api.rs) with the task-details
mutation surface:

- `POST /v1/tasks/{task_id}/update`
- `POST /v1/tasks/{task_id}/status`
- `POST /v1/tasks/{task_id}/comments`
- `POST /v1/tasks/{task_id}/delete`

These routes should become the canonical React mutation contracts for the task
page.

### 2.2 Add A Typed Replacement For `/task/{task_id}/modal`
Replace the HTML fragment dependency with typed JSON:

- prefer `GET /v1/tasks/{task_id}/edit` as the direct React replacement for the
  current modal bootstrap route
- if the existing `GET /v1/tasks/{task_id}` payload plus existing lookup APIs
  already gives React everything it needs for the edit modal, do not invent a
  redundant second route
- the React page must not depend on `/task/{task_id}/modal` returning HTML

### 2.3 Reuse Existing Forms And Services
Do not invent a second validation stack just for React. Reuse the forms and
payload conversions already established in earlier phases:

- update should accept request data matching `UpdateTaskForm`
- quick status should accept request data matching `QuickTaskStatusForm`
- comments should accept request data matching `TaskCommentForm`
- delete may accept an empty form body if no additional fields are required

At the route boundary:

- convert form data into the existing payload objects
- call the existing task service functions in `src/services/task.rs`
- return JSON using the shared mutation envelope helpers from `src/routes/mod.rs`

### 2.4 Mutation Response Rules
Use the shared mutation envelope shape rather than inventing route-specific
responses:

- success responses should use `ApiMutationSuccessDto`
- failure responses should use `mutation_error_response(...)`
- keep status mapping consistent with `mutation_error_status(...)`
- prefer frontend refetch after mutation success over complex optimistic
  mutation payload design

### 2.5 API Route Mounting
Update [../src/lib.rs](../src/lib.rs) so the new task-details JSON handlers are
mounted under the existing `/api` scope.

## Task 3: Extend The Frontend API Layer For Task-Page Actions
The React task page needs typed read and write helpers, but transport and
request-shape details should stay in `frontend/src/lib/`.

### 3.1 Edit `frontend/src/lib/models.ts`
Expand [../frontend/src/lib/models.ts](../frontend/src/lib/models.ts) only as
needed for Phase 5:

- keep the Phase 2 shell models
- keep the Phase 3 task-details, lookup, and mutation-envelope models
- add only small supporting types needed for task-page form state, edit-modal
  bootstrap data, or mutation result handling

Do not add cleanup-only compatibility types for the old Tera templates.

### 3.2 Edit `frontend/src/lib/api.ts`
Expand [../frontend/src/lib/api.ts](../frontend/src/lib/api.ts) with
task-details-owned helpers:

- fetch task details by id
- fetch any edit-modal bootstrap data if a dedicated endpoint is used
- submit task updates
- submit quick status changes
- submit new comments
- submit task deletion

Keep the API helpers responsible for:

- URL construction
- `FormData` or request encoding
- JSON parsing
- HTTP error normalization
- returning typed mutation envelopes to the page layer

### 3.3 Lookup Consumption Rules
The React edit flow must stop depending on template JS that fetches directly
from external services.

Use the local ToDo-owned endpoints instead:

- `GET /api/v1/users`
- `GET /api/v1/clients`
- `GET /api/v1/tracks`

If a lookup contract is missing a field React genuinely needs, extend the
typed local DTO rather than restoring browser-side calls to auth or CRM.

## Task 4: Replace The Task Placeholder Entry With A Real React Page
The current `frontend/src/entries/task.tsx` still renders a placeholder. Phase
5 replaces it with the real task-details page.

### 4.1 Edit `frontend/src/entries/task.tsx`
Update [../frontend/src/entries/task.tsx](../frontend/src/entries/task.tsx):

- stop rendering `PhaseOnePlaceholderPage`
- mount a real task-details page component
- derive the current `task_id` from the route URL instead of embedding server
  data into HTML

### 4.2 Create A Real Task-Details Page
Add a page component under `frontend/src/pages/`, for example
`TaskDetailsPage.tsx`, that:

- loads shell state and task details
- handles loading, not-found, unauthorized, and fatal API states
- keeps page-specific interaction state close to the page layer
- reuses shared shell and modal components instead of recreating navbar/layout
  wiring

### 4.3 Preserve Current Task-Page Surface
The React page should preserve the visible behaviors currently owned by
`templates/task/index.html`:

- title and status badge in the page header
- metadata cards and description rendering
- author, assignee, and client presentation
- pending to in-progress quick action
- completion flow with optional comment
- add-comment composer
- event timeline with empty state

Do not quietly drop current task-page features during the cutover.

## Task 5: Rebuild The Page-Specific Interactions In React
The old task page relies on HTMX fragment loading, inline scripts, and template
markup. Replace those behaviors with React-owned state and components.

### 5.1 Build React Components For The Task Page
Create task-page components under `frontend/src/components/` for the major
surfaces as needed:

- task metadata/details display
- quick status actions
- edit-task modal
- delete confirmation flow
- comment composer
- event timeline

Prefer small components with clear ownership over one oversized page file.

### 5.2 Edit Flow Requirements
The React edit flow must preserve the capabilities of the current modal:

- title, description, due date, status, track, and priority editing
- assignee and client lookup/select behavior
- delete action reachable from the edit flow or another equally discoverable
  task-page control
- validation errors shown from the shared JSON mutation envelope

The edit UI no longer needs to mirror the old HTMX loading mechanics, but it
must preserve the underlying behavior and field coverage.

### 5.3 Quick Status Rules
Preserve the current quick-status behavior from the live task page:

- pending tasks expose the "take in progress" action
- in-progress tasks expose the completion flow with optional comment
- do not invent unrelated new status-control surfaces on the task page just
  because React makes that easy

If you intentionally improve the current UX, keep the same backend semantics.

### 5.4 Markdown And Rich Text Rules
Reuse the existing frontend markdown helper rather than restoring template-side
inline JS:

- comment composition should still support the current markdown authoring flow
- task descriptions and comment/event content should render sanitized HTML from
  the typed API contracts

## Task 6: Remove User-Facing Dependence On The HTMX Modal Route
Phase 5 ends when the live user flow no longer needs HTML fragments from
`/task/{task_id}/modal`.

### 6.1 Stop Calling The Modal Fragment Route From React
Make sure the React page:

- never calls `/task/{task_id}/modal`
- never waits for HTML to populate a Bootstrap modal body
- never depends on `htmx:*` events for task-page behavior

### 6.2 Keep Legacy Route Cleanup Narrow
You may keep the old `POST /task/{task_id}/modal` route and template in place
temporarily if that keeps the migration isolated, but:

- the live user flow must no longer need it
- do not spend Phase 5 deleting old templates or doing broad cleanup
- reserve full dead-code and dependency removal for Phase 6

## Task 7: Add Test Coverage For The New Task Page
Phase 5 changes both backend API behavior and frontend interaction behavior.
Add coverage for both.

### 7.1 Frontend Tests
Add or extend frontend tests to cover:

- task-page loading and fatal states
- edit modal open and submit behavior
- quick status actions
- comment submission
- delete confirmation flow
- event timeline rendering, including the empty state

### 7.2 Backend API Tests
Extend `tests/api.rs` with coverage for:

- any new typed edit-bootstrap endpoint
- task update JSON mutation
- task comment JSON mutation
- task status JSON mutation
- task delete JSON mutation

Keep the tests focused on contract shape, status codes, and error-envelope
behavior.

## Task 8: Verification
After implementation, run the full repository verification set:

```bash
cd frontend && npm run typecheck
cd frontend && npm run test
cd frontend && npm run build
cargo build --all-features --verbose
cargo test --all-features --verbose
cargo clippy --all-features --tests -- -Dwarnings
cargo fmt --all -- --check
```

Manual verification should include:

- open `GET /task/{task_id}` in a browser
- verify task details render without server-templated page markup
- verify edit, quick status, comment, and delete flows succeed through the JSON
  API routes
- verify the page no longer depends on `/task/{task_id}/modal`

## Expected Repository State After Phase 5
After Phase 5 is complete:

- `frontend/src/entries/task.tsx` mounts a real task-details page
- `frontend/src/pages/` contains the task-details page implementation
- `frontend/src/components/` contains task-page-specific React components
- `GET /task/{task_id}` serves `assets/dist/app/task.html`
- task update, comment, status, and delete actions are available as JSON API
  routes
- the live task page no longer depends on HTMX fragment rendering
- the old task templates may still exist in the repository, but they are no
  longer required for user-facing behavior

## Phase 5 Exit Checklist
- `GET /task/{task_id}` serves the built task frontend document after auth
  checks
- the visible task-details page is React-rendered
- task details initialize from typed `/api/v1/...` data rather than Tera
  context
- task update, quick status, comment, and delete flows use structured JSON
  success/error responses
- `/task/{task_id}/modal` is no longer required for the live user flow
- `GET /` remains React-backed and Phase 4 behavior is intact
- Phase 6 cleanup work has not been mixed into this phase

## Explicit Non-Goals For This Task File
- deleting `templates/task/index.html` or `templates/task/modal_body.html`
- removing Tera, HTMX, or flash-message middleware from the repository
- broad dead-code cleanup outside the task-page cutover
- changing backend business rules for task ownership, authorization, or event
  generation
- starting the final dependency-removal work reserved for Phase 6
