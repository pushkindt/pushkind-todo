# React Frontend Migration Preserving Existing Todo UI

## Status
Stable

## Date
2026-03-29

## Summary
Migrate the current Tera-based `pushkind-todo` frontend to React-managed UI
components while preserving the existing route structure, Bootstrap styling,
Russian copy, task workflows, and backend-owned business rules. The migration
MUST follow the same stable pattern already implemented in
`pushkind-auth`, `pushkind-files`, `pushkind-crm`, and `pushkind-emailer`:
server-routed pages,
Vite-built static frontend documents for React-owned pages,
typed client data APIs under `/api/v1/`,
resource-style GET endpoints where practical,
and structured JSON mutation responses with form-owned validation copy.

`pushkind-todo` MUST NOT become a SPA.

## Problem
The current todo UI is split across Tera templates, flash-message redirects,
Bootstrap modal fragments, and template-owned interaction logic. That makes the
task list, filter modal, create-task modal, quick status changes, task editing,
and comments harder to compose, test, and evolve.

The service currently mixes several frontend ownership models:
- server-rendered full pages via Tera
- HTML fragment rendering for the task edit modal
- flash-driven POST/redirect UX
- one legacy JSON route that is not aligned to the newer React API pattern

That fragmentation is the main reason to migrate.

## Goals
- Introduce React as the component model for todo user-facing pages.
- Preserve the current Bootstrap-based design, URLs, semantics, and Russian
  user-visible copy.
- Preserve current backend authorization, validation, sanitization,
  notification, ZeroMQ publishing, and persistence rules.
- Replace Tera-owned interactive behavior with React-owned components and typed
  data contracts as pages are migrated.
- Keep `pushkind-todo` server-routed and non-SPA.
- Align frontend architecture with the migration pattern already established in
  the other migrated Pushkind services.

## Non-Goals
- Introducing client-side routing.
- Redesigning the UI or replacing Bootstrap.
- Moving validation, authorization, sanitization, notification, or persistence
  rules into the browser.
- Changing task domain semantics, repository rules, or ZeroMQ payload meaning
  beyond what React needs for UI parity.
- Replacing the auth/session model with browser token storage.

## In Scope
- The authenticated task list page at `GET /`.
- The task details page at `GET /task/{task_id}`.
- Shared shell concerns currently handled by Tera layout/navigation.
- Create-task, filter-task, edit-task, quick-status, comment, upload, and
  delete interactions currently driven by Tera, modal HTML, or flash-driven
  redirects.
- Frontend asset build and delivery needed to run React in production and local
  development.

## Out Of Scope
- Anonymous or unauthenticated access.
- Schema redesign, repository redesign, or task workflow redesign.
- Replacing outbound email notifications or ZeroMQ publication.
- Public third-party API design beyond the internal React client-data layer.

## Functional Requirements

### 1. Rendering Model
- The application MUST keep the existing server-owned route model.
- The application MUST NOT introduce client-side routing for `/` or
  `/task/{task_id}`.
- React MUST be introduced as page-level or island-level components mounted on
  the existing URLs.
- The target state for migrated pages MUST be React-owned page markup served
  from Vite-built static HTML documents after backend access checks.

### 2. Frontend Document Ownership
- React-owned full pages SHOULD be authored in the frontend workspace and built
  by Vite into static HTML documents under `assets/dist/`.
- Rust MUST continue to own authentication and authorization checks before
  serving those documents.
- Page initialization data MUST NOT remain embedded into server-generated HTML
  in the target state.
- Tera MAY remain only as a temporary migration wrapper until a page is fully
  React-backed.

### 3. Markup And Style Preservation
- Migrated React components MUST preserve the current Bootstrap-based layout,
  task card structure, modal structure, navigation hierarchy, and class
  conventions unless a deviation is explicitly documented.
- User-visible Russian copy SHOULD remain unchanged except for bug fixes or
  accessibility improvements.
- Existing Bootstrap JS behaviors such as dropdowns, modals, and tabs MUST
  continue to work.

### 4. Behavioral Parity
- `GET /` MUST continue to present the task list, filters, pagination, add-task
  affordance, CSV upload, quick status update controls, and links into task
  details.
- `GET /task/{task_id}` MUST continue to present task metadata, author,
  assignee, client linkage, event timeline, quick status flow, edit flow, and
  comment flow.
- The current add-task and filter-task modal UX on the task list MUST continue
  to work after migration, but the interaction ownership SHOULD move to React.
- The current task edit modal HTML fragment flow MUST be replaced by typed JSON
  data and React-owned modal rendering before that interaction is considered
  fully migrated.

### 5. Client Data API Model
- React-owned page initialization MUST prefer typed GET APIs under `/api/v1/`
  rather than HTML-embedded bootstrap payloads or HTML partial rendering.
- The target state SHOULD prefer reusable resource-style APIs over page-shaped
  bootstrap endpoints where practical.
- Shared shell data such as current user, home URL, navigation, and auth-driven
  user-menu items SHOULD be exposed through a typed shell API.
- Expected resource-style GET APIs for the target state include:
  - `/api/v1/iam`
  - `/api/v1/tasks`
  - `/api/v1/tasks/{task_id}`
  - `/api/v1/users`
  - `/api/v1/clients`
  - `/api/v1/tracks`
  - `/api/v1/no-access`
- The current unmounted `/api/v1/tasks` route MUST be aligned into that
  resource-style API surface rather than kept as a legacy side path.

### 6. Mutation And Validation Semantics
- React-owned mutation flows SHOULD use structured JSON success/error responses
  instead of flash-message-driven redirects or HTML partial rendering.
- Field-level validation errors MUST be addressable so React can render them
  inline.
- Validation copy for React-owned forms MUST be owned by `src/forms`, following
  the same pattern used in `pushkind-auth`, `pushkind-crm`, and
  `pushkind-emailer`.
- Russian validation strings MUST be defined directly on form field
  `#[validate(..., message = "...")]` annotations and on `#[error("...")]`
  annotations for `FormError` enum variants, rather than assembled in routes or
  services.
- Routes SHOULD convert `Form -> Payload` at the boundary before calling
  services, so services can continue using the common `ServiceError` pattern.
- Download-style or non-React endpoints MAY remain only where they are still
  the correct transport.

### 7. Backend Boundary
- Authorization, validation, sanitization, notification, queueing, and
  persistence MUST remain in Rust services and repositories.
- Routes MUST expose typed DTOs or UI-ready payloads to React rather than
  leaking template contexts directly.
- Legacy HTML fragment endpoints SHOULD be replaced by typed JSON data APIs
  before the corresponding interaction is considered fully migrated.

### 8. Shared Navigation And User Menu
- The top navigation SHOULD follow the same reusable React pattern already used
  in the migrated Pushkind services.
- The user dropdown MUST always include `Домой` and logout.
- Todo-local dropdown items MUST render before items fetched from the auth menu
  API.
- Additional menu items SHOULD come from the auth menu API.
- Failure to load auth-driven menu items MUST NOT make `pushkind-todo`
  unavailable.
- Logout MUST always render as the final dropdown action even if fetched menu
  items change.

### 9. No-Access Surface
- `pushkind-todo` MUST own its own no-access page the same way as the other
  migrated services.
- The target state MUST use a local React-backed `/na` page and
  `/api/v1/no-access` payload rather than depending on the shared
  `not_assigned` route implementation.

### 10. Frontend Tooling
- The repository MUST gain a supported frontend toolchain for React and
  TypeScript source code.
- Production builds MUST emit versioned static assets and required static HTML
  documents that can be served by the Rust application.
- The server MUST serve the compiled frontend assets directly.
- Local development MUST support efficient frontend iteration without manual
  asset copying.

## Migration Requirements
- The migration MUST be incremental.
- The migration SHOULD converge on the same stable shape used in the already
  migrated services:
  Vite-built static HTML for React-owned full pages,
  typed `/api/v1/...` client data APIs,
  resource-style GET endpoints,
  structured JSON mutation responses,
  and form-owned validation messages.
- Shared React shell components SHOULD be introduced early for navigation,
  user-menu behavior, and common mutation handling.
- Tera MUST be removable as a runtime dependency once all migrated pages are
  fully React-owned.
- `actix-web-flash-messages` MUST be removable as a direct runtime dependency
  once React-owned mutation flows replace flash-driven redirects.
- Inline JavaScript, template-owned interaction code, and HTML modal fragments
  SHOULD be removed only after equivalent React behavior is verified.
- Regression verification SHOULD rely on backend contract tests, frontend
  component or integration tests, and targeted manual checks for
  authentication-dependent flows.

## Acceptance Criteria
- The same URLs continue to serve the corresponding todo pages and actions.
- Visual appearance remains substantially unchanged for navigation, task list,
  filters, task details, modals, and comments.
- React-owned pages are served from Vite-built frontend documents after backend
  access checks.
- Page data comes from typed client data APIs rather than HTML-embedded
  bootstrap payloads.
- GET APIs exposed for React follow the resource-style `/api/v1/...` pattern
  rather than page-named bootstrap endpoints.
- React-owned mutations return structured success/error responses with
  field-addressable validation errors.
- Russian validation strings are owned by form field validator annotations and
  `FormError` enum annotations, not by routes or services.
- The shared user dropdown behaves consistently with the already migrated
  services.
- `pushkind-todo` owns a local React-backed `/na` surface.
- No backend business rule is moved to the client.
- Direct `tera` and `actix-web-flash-messages` dependencies are removed from
  `pushkind-todo` once the migration is complete.
- The React frontend builds reproducibly and its assets are served by the
  application runtime.
- Regression coverage exists for backend page-data contracts and critical
  frontend behavior.

## Risks
- React markup can drift from the current templates unless parity is checked
  explicitly.
- Task modal and quick-status flows currently depend on legacy HTML and
  redirect semantics, so those boundaries need careful route-by-route
  conversion.
- Todo pages have multiple mutation entry points, which increases the chance of
  leaving inconsistent flash-redirect and JSON-mutation behavior during an
  incremental migration.
