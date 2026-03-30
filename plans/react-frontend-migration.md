# Plan: React Frontend Migration

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)

## Objective
Introduce React for the `pushkind-todo` frontend while preserving the current
route structure, Bootstrap styling, Russian copy, task workflows, and
backend-owned authorization, validation, sanitization, notification, ZeroMQ,
and persistence rules. The migration remains server-routed and non-SPA, and
converges on:
Vite-built static HTML for React-owned full pages,
typed `/api/v1/...` client data APIs,
resource-style GET endpoints,
structured JSON mutation responses,
and form-owned validation copy.

## Current-State Notes
- `src/lib.rs` currently wires Tera, `actix-web-flash-messages`, shared
  `not_assigned`, and Actix file serving from `/assets`.
- `templates/base.html` owns shared navigation, flash rendering, Bootstrap
  initialization, popovers/tooltips, and markdown preview setup.
- `templates/main/index.html` owns task-list rendering, filter/add-task modals,
  query-string hydration, and assignee lookup behavior.
- `templates/task/index.html` owns task-details rendering, quick status flows,
  comment composition, and the HTMX-backed edit modal flow.
- `src/routes/task.rs` still serves `/task/{task_id}/modal` as HTML fragment
  transport; that route is incompatible with the target React ownership model.
- `src/routes/api.rs` already mounts `GET /api/v1/tasks`; the migration should
  align and extend that existing API surface instead of duplicating it.

## Fixed Implementation Decisions
- Frontend source code WILL live in `frontend/`.
- Production frontend build output WILL live in `assets/dist/`.
- The React toolchain WILL use `npm`, React, TypeScript, and Vite.
- The backend WILL continue to own routing, authentication, authorization,
  validation, sanitization, notification, ZeroMQ publication, and persistence.
- The application server WILL continue to serve compiled frontend assets from
  the existing `/assets` path.
- Vite WILL own the static HTML documents for `GET /`, `GET /task/{task_id}`,
  and `GET /na` once those routes are React-owned.
- React page initialization WILL fetch typed JSON data from backend endpoints;
  page data WILL NOT remain embedded into server-generated HTML in the target
  state.
- New GET endpoints introduced for React-owned page data WILL be versioned
  under `/api/v1/`.
- Those GET endpoints MUST prefer reusable resource-style contracts over
  page-shaped bootstrap endpoints.
- React-owned mutation flows SHOULD also move to `/api/v1/...` JSON endpoints
  so flash-driven UI routes can be removed cleanly at the end of the migration.
- Validation copy for React-owned forms WILL live in `src/forms`.
- Russian validation strings WILL be defined directly in
  `#[validate(..., message = "...")]` annotations on form fields and in
  `#[error("...")]` annotations on `FormError` enum variants.
- Routes SHOULD convert `Form -> Payload` at the boundary before calling
  services so services can continue using `ServiceError` and DTO/domain return
  types.
- The shared navbar and user dropdown WILL align with the React pattern already
  used in the migrated Pushkind services.
- Todo-local dropdown items WILL render before auth-fetched menu items.
- Logout WILL remain the final dropdown action even when auth-fetched menu
  items change or fail to load.
- `pushkind-todo` WILL own a local React-backed `/na` route and
  `/api/v1/no-access` payload.
- Tera MAY remain only as a temporary migration wrapper while a page is being
  cut over, and MUST be removable once all migrated pages are React-owned.
- `tera` and `actix-web-flash-messages` MUST be removable from direct
  `pushkind-todo` dependencies by the end of the migration.
- HTMX fragment loading and template-owned inline JavaScript MUST be removed
  only after equivalent React behavior is shipped and verified.
- Regression verification WILL rely on backend contract tests, frontend
  component or integration tests, and targeted manual checks for
  authentication-dependent flows.

## Repository Layout
The implementation SHOULD create and use the following structure:

```text
frontend/
  package.json
  package-lock.json
  tsconfig.json
  vite.config.ts
  src/
    entries/
    components/
    pages/
    styles/
    lib/
assets/
  dist/
src/
  dto/
  forms/
  routes/
  services/
  frontend.rs
templates/
```

Directory intent:
- `frontend/src/entries/`:
  entrypoints for `/`, `/task/{task_id}`, and `/na`.
- `frontend/src/components/`:
  reusable shell, navbar, user-menu, modal, form, list, card, timeline, and
  pagination components.
- `frontend/src/pages/`:
  page-level React components for the task list, task details, and no-access
  surfaces.
- `frontend/src/lib/`:
  typed payload readers, API clients, endpoint builders, Bootstrap adapters,
  markdown helpers, and cross-service menu helpers.
- `frontend/src/styles/`:
  CSS imports preserving the current Bootstrap-based output.
- `assets/dist/`:
  compiled JavaScript, CSS, static HTML, and manifest output.
- `src/frontend.rs`:
  backend helpers for loading Vite manifest entries and serving built frontend
  HTML documents after route-level access checks.

## Toolchain And Build Outputs

### Frontend Package Management
- Use `npm` as the package manager.
- Commit `frontend/package-lock.json`.
- Do not introduce `pnpm`, `yarn`, or an alternative JavaScript runtime.

### Build Tool
- Use Vite to build the React frontend.
- Configure Vite to emit compiled assets into `assets/dist/`.
- Configure Vite to emit a manifest file at `assets/dist/manifest.json`.
- Configure explicit entrypoints for:
  the task list page,
  the task details page,
  the no-access page.

### Required `package.json` Scripts
The frontend package MUST expose at least these scripts:
- `dev`
- `build`
- `preview`
- `test`
- `lint`
- `typecheck`

### Source Control Hygiene
- Update `.gitignore` to exclude `frontend/node_modules/`.
- Add `assets/dist/` to `.gitignore` unless deployment later requires
  committed build artifacts.

## Backend Integration

### Asset Serving
- Keep Actix static serving for `/assets` and ensure it covers `assets/dist/`.

### Built HTML Serving
- Add a backend helper that serves the built Vite HTML entry for each
  React-owned full-page route after authentication and authorization checks.
- Rust MUST stop assembling full-page HTML at request time once a route has
  been fully migrated.

### Frontend Helper Alignment
- Add a backend helper for opening built frontend HTML documents aligned with
  the pattern already used in the migrated Pushkind services.
- Avoid introducing todo-specific frontend-loading abstractions unless they are
  clearly reusable across the page routes in this crate.

### Client Data APIs
- Add typed DTOs under `src/dto/` for reusable todo client data APIs.
- Prefer specific resource-style endpoints under `/api/v1/` over page-shaped
  bootstrap endpoints.
- The target GET surface SHOULD include:
  `GET /api/v1/iam`,
  `GET /api/v1/tasks`,
  `GET /api/v1/tasks/{task_id}`,
  `GET /api/v1/users`,
  `GET /api/v1/clients`,
  `GET /api/v1/tracks`,
  `GET /api/v1/no-access`.
- `GET /api/v1/tasks` SHOULD evolve from the currently mounted list route into
  the canonical task-collection contract for React.
- `GET /api/v1/tasks/{task_id}` SHOULD expose task metadata, author, assignee,
  client, and event timeline data without relying on template contexts.
- `GET /api/v1/users` and `GET /api/v1/clients` SHOULD replace the current
  template-owned direct fetches to the auth and CRM services.
- `GET /api/v1/iam` SHOULD expose the shell data React needs:
  current user identity,
  service-local navigation items,
  auth home URL,
  logout target,
  and any auth-menu fetch URL or menu DTOs needed for dropdown hydration.
- `GET /api/v1/no-access` SHOULD return the local content required by the React
  `/na` page.
- Do not expose raw template contexts directly to the frontend.

### Structured Mutation Responses
- Introduce typed JSON mutation response DTOs for React-owned task
  interactions.
- The initial JSON mutation surface SHOULD cover:
  create task,
  upload tasks,
  update task,
  quick status transition,
  add comment,
  delete task.
- Field errors SHOULD use a stable field-addressable shape.
- Success responses SHOULD return either the updated resource, a stable success
  marker, or a redirect target when the client genuinely needs it.
- Legacy redirect-plus-flash handlers MAY coexist temporarily, but React-owned
  pages MUST migrate to JSON request/response handling before the flash
  middleware is removed.

### Form Boundary Ownership
- Move React-owned validation copy into `src/forms`.
- Update `AddTaskForm`, `UpdateTaskForm`, `TaskCommentForm`, and related form
  helpers so field-level validation messages are authored on validator
  annotations and `FormError` annotations in Russian.
- Keep sanitization and type construction at the form boundary.
- Keep services free of HTTP-specific validation formatting.

### Local No-Access Ownership
- Replace usage of `pushkind_common::routes::not_assigned` with a local todo
  route for `/na`.
- Keep backend authorization redirects intact, but send unauthorized todo
  traffic to the local React-backed no-access surface.

### Server-Rendered Shell During Migration
- During migration, the backend MAY render a minimal HTML shell that:
  includes the React entrypoint,
  includes compiled CSS,
  provides the mount node for React.
- Any such shell is transitional only. The target state for a migrated page is
  a Vite-built static HTML document, not a Rust-rendered page shell.

## Frontend Runtime Requirements

### Shared Shell And Navigation
- Implement a shared React shell for navbar, layout wiring, user-menu
  behavior, Bootstrap lifecycle integration, and flash/error presentation.
- The shared shell SHOULD align with the reusable dropdown/menu approach
  already used in the migrated Pushkind services.
- Auth menu loading MUST happen after required page data is available so auth
  slowness does not blank the todo page.
- Failure to load auth-driven menu items MUST still leave `Домой` and logout
  available.

### Bootstrap Integration
- Keep Bootstrap CSS and Bootstrap Icons in the rendered output.
- Preserve Bootstrap JS behavior for dropdowns, modals, tabs, tooltips, and
  popovers.
- Move inline Bootstrap lifecycle code into React-safe helpers under
  `frontend/src/lib/`.

### Task List Page Requirements
- The React `GET /` page MUST preserve:
  task rows,
  recently updated highlighting,
  pagination,
  query-string driven filtering,
  add-task modal,
  filter modal,
  CSV upload,
  quick status controls,
  and row navigation into task details.
- The frontend SHOULD keep URL query parameters as the source of truth for
  list filters and pagination so deep links remain stable.

### Task Details Page Requirements
- The React `GET /task/{task_id}` page MUST preserve:
  task header and metadata,
  quick status flow,
  edit flow,
  delete flow,
  comment flow,
  assignee and client display,
  and the event timeline.
- The task edit modal MUST become a typed React-owned modal before
  `/task/{task_id}/modal` can be removed.

### Markdown, Lookup, And Modal Behavior
- The current markdown compose-and-preview behavior SHOULD move into React-owned
  components without moving sanitization rules into the browser.
- User lookup and client lookup interactions SHOULD become React-owned typed
  autocomplete/select flows backed by the local `/api/v1/users` and
  `/api/v1/clients` endpoints.
- Modal open/close behavior SHOULD remain Bootstrap-compatible while React owns
  the content and submission state.

### Data Loading
- React-owned full pages MUST fetch typed JSON data after the static HTML
  document loads.
- The frontend SHOULD use shared API helpers that compose page state from
  narrower resource endpoints.
- React MUST render explicit loading and fatal error states for required data
  fetches.

### Form And Action Handling
- React-owned mutation flows SHOULD use structured JSON request/response
  handling instead of redirect-plus-flash patterns.
- Multipart upload MAY remain `multipart/form-data`, but the React-owned
  response contract MUST still be structured JSON.
- Native browser navigation SHOULD remain in place for full-page route changes;
  React MUST NOT introduce client-side routing.

## Migration Sequence

### Phase 1: Foundation
Deliverables:
- `frontend/` directory with React, TypeScript, and Vite configured.
- Build output emitted to `assets/dist/`.
- Backend helpers for loading frontend manifest entries and serving built HTML.
- `.gitignore` updated for frontend dependencies and generated assets.
- Developer documentation for installing Node and building frontend assets.

Exit criteria:
- `npm run build` succeeds.
- The server can resolve one Vite-built frontend document and its compiled
  assets.

### Phase 2: Shared Shell, Navigation, And No-Access Surface
Deliverables:
- Shared React shell for navbar, user-menu behavior, common layout wiring, and
  Bootstrap lifecycle integration.
- Typed `GET /api/v1/iam` and `GET /api/v1/no-access` endpoints.
- React-backed `/na` page served from a built Vite document after backend
  access checks.
- Local route ownership for `/na` instead of the shared `not_assigned` route.

Exit criteria:
- `pushkind-todo` can render its own React-backed no-access page.
- Shared shell behavior no longer depends on inline JavaScript in
  `templates/base.html`.

### Phase 3: Task And Lookup API Contracts
Deliverables:
- Typed `GET /api/v1/tasks`, `GET /api/v1/tasks/{task_id}`,
  `GET /api/v1/users`, `GET /api/v1/clients`, and `GET /api/v1/tracks`
  endpoints.
- DTOs for task-list items, task-details payloads, lookup results, shell
  payloads, and no-access payloads.
- Shared JSON mutation response DTOs and error envelope for React-owned forms.
- Route-boundary `Form -> Payload` conversion pattern for React-owned mutation
  handlers.
- Russian validator and form-error message ownership moved into `src/forms`.

Exit criteria:
- React can initialize both major pages from typed `/api/v1/...` APIs.
- No new React flow depends on HTML fragment endpoints or route-level flash
  strings.

### Phase 4: Task List Page Migration
Deliverables:
- React-backed `GET /` page served from a Vite-built HTML document after auth
  checks.
- React rendering for:
  task list,
  filter state,
  pagination,
  add-task modal,
  CSV upload,
  quick status controls,
  and recently updated highlighting.
- Structured JSON handling for create-task, upload, and any task-list-owned
  quick status interactions.
- React-owned replacement for the current page-specific inline scripts on the
  index page.

Exit criteria:
- The task list page is React-rendered with visual and behavioral parity.
- `GET /` no longer depends on Tera-owned page markup or template-owned inline
  behavior.

### Phase 5: Task Details Page Migration
Deliverables:
- React-backed `GET /task/{task_id}` page served from a Vite-built HTML
  document after auth checks.
- React rendering for:
  task metadata,
  quick status flow,
  edit modal,
  delete flow,
  comment composer,
  and event timeline.
- Typed React replacement for `/task/{task_id}/modal`.
- Structured JSON handling for update, comment, delete, and quick status flows.

Exit criteria:
- The task details page works end to end through React-owned UI without HTMX
  fragment rendering.
- `/task/{task_id}/modal` is no longer required for user-facing behavior.

### Phase 6: Cleanup And Dependency Removal
Deliverables:
- Remove obsolete templates and template fragments once they are unused by
  user-facing routes.
- Remove inline page scripts and shared template JavaScript that React now
  owns.
- Remove HTMX from runtime paths if nothing user-facing depends on it.
- Remove direct `tera` and `actix-web-flash-messages` dependencies once all
  migrated pages and mutations no longer need them.
- Update README and any operational docs for the final frontend build/runtime
  shape.

Exit criteria:
- No user-facing todo page depends on Tera or flash-message middleware for
  frontend rendering or React-owned mutation feedback.
- The server still serves the same URLs with the same backend business rules.

## Testing And Verification

### Frontend Verification
- Add component or integration coverage for:
  navbar dropdown behavior,
  filter modal state,
  add-task flow,
  edit-task flow,
  quick status actions,
  comment submission,
  and no-access rendering.
- Add type-checking as a required verification step.
- Add linting as a required verification step.

### Visual Parity Verification
- Use deterministic visual regression checks.
- Capture reference screenshots or equivalent visual baselines for:
  `GET /`,
  task list with active filters,
  add-task modal,
  task details page,
  edit-task modal,
  complete-task modal,
  `GET /na`.
- Verify visual parity at deterministic desktop and mobile viewports.
- Stabilize test data, timing, and fonts so visual checks are reproducible.

### Backend Verification
- Add integration coverage for `/api/v1/...` page-data and lookup contracts.
- Add backend tests for JSON mutation success and field-error response shapes.
- Add tests for manifest loading and built HTML resolution helpers.
- Preserve or extend existing service and repository tests where route behavior
  changes.

### Manual Verification
- Verify unauthorized access redirects to the local `/na` page.
- Verify the user dropdown still shows `Домой` before auth-fetched items and
  logout last.
- Verify task creation, update, completion, commenting, upload, and deletion
  still publish the same backend side effects.
- Verify task filter URLs remain shareable and stable.
- Verify task page and list page preserve Russian copy and Bootstrap layout.

### Required Commands
Run these commands from `pushkind-todo` unless noted otherwise:

1. `cd frontend && npm run typecheck`
2. `cd frontend && npm run test`
3. `cd frontend && npm run build`
4. `cargo build --all-features --verbose`
5. `cargo test --all-features --verbose`
6. `cargo clippy --all-features --tests -- -Dwarnings`
7. `cargo fmt --all -- --check`

## Follow-Up Before Implementation Starts
- Add an ADR under `specs/decisions/` capturing the frontend runtime
  architecture choices for this migration, since the end state changes page
  document ownership, API transport, and runtime dependencies.
