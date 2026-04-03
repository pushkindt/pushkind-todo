# pushkind-todo

`pushkind-todo` is the task management service that powers Pushkind hubs. It serves
React-built task pages, CSV imports, and JSON endpoints for creating, assigning,
and tracking work. The project is implemented in Rust on top of Actix Web and
Diesel, with a Vite/React frontend, and integrates tightly with the shared
`pushkind-common` crate for authentication, configuration, and reusable runtime
helpers.

Comprehensive API documentation generated with `cargo doc` is published at
[pushkindt.github.io/pushkind-todo](https://pushkindt.github.io/pushkind-todo/).

## Features

- **Role-locked task dashboard** – Authenticated hub members who hold
  `SERVICE_ACCESS_ROLE` can browse tasks with pagination, free-text search, status
  filters, and updated-before/after date filters that help narrow down large
  backlogs.
- **Task detail views with history** – Each task page surfaces metadata,
  author/assignee information, and a chronological event stream so teams can audit
  status changes, assignment churn, and metadata updates.
- **Inline task editing** – The React task page lets users update titles,
  descriptions, due dates, statuses, and assignments in one submit, while the
  service layer emits structured events for downstream consumers.
- **Quick task capture** – The home page provides a streamlined form for creating
  a single task, automatically sanitising descriptions and ensuring the author
  record is kept in sync.
- **Bulk CSV imports** – Operations teams can upload CSV files (up to 10 MB) to
  create many tasks at once with optional descriptions, ideal for onboarding work
  queues from spreadsheets.
- **Collaborative commentary** – Team members can add HTML-sanitised comments to a
  task, generating timeline events that keep conversations alongside the work.
- **Task archiving** – Users can delete tasks they no longer need; the action is
  validated against hub ownership and exposed through the same JSON mutation
  contract used by the frontend.
- **Typed JSON contracts** – The `/api/v1/*` endpoints expose the shell, task
  collection, task details, lookups, and mutation results in stable JSON shapes
  consumed by the React frontend and external integrations.
- **Email notifications** – The service publishes a ZeroMQ message for email
  delivery whenever a task is created or a task event occurs, notifying the
  author, the assignee, and every participant who has generated task events so
  far—always skipping the actor who performed the current change.

## Pages

- **Main page** – Displays the paginated task list with search and filter
  controls. Tasks are sorted by their `updated_at` timestamp with the most
  recently updated first. Entries whose `updated_at` is more recent than the
  current user's `visited_at` are highlighted so users can quickly spot changes
  since their last visit. The React page also includes the quick-create form for
  adding new tasks, and clicking a task row opens its detail page.
- **Task details page** – Shows the full task metadata alongside editing
  controls for the title, description, deadline, assignee, and status. Users can
  add comments and review the chronological event log. Any changes made to the
  task or comments that are submitted create corresponding entries in the task's
  event stream.

## Architecture at a Glance

The codebase follows a clean, layered structure so that business logic can be
exercised and tested without going through the web framework:

- **Domain (`src/domain`)** – Type-safe models for hubs, menus, roles, and users.
  Domain types never validate or normalize; they assume inputs are already
  cleaned and transformed by forms/services. Domain structs use strongly typed
  fields (e.g., `UserEmail`, `HubId`, `UserName`) so the
  type system enforces the invariants of each value.
- **Repository (`src/repository`)** – Traits that describe the persistence
  contract and a Diesel-backed implementation (`DieselRepository`) that speaks to
  a SQLite database. Each module translates between Diesel models and domain
  types and exposes strongly typed query builders.
- **Services (`src/services`)** – Application use-cases that orchestrate domain
  logic, repository traits, and Pushkind authentication helpers. Services return
  `ServiceResult<T>` and map infrastructure errors into well-defined service
  errors.
- **Forms (`src/forms`)** – `serde`/`validator` powered structs that handle
  request payload validation, CSV parsing, and transformation into domain types.
- **Routes (`src/routes`)** – Actix Web handlers that wire HTTP requests into the
  service layer, return JSON contracts, and serve the built frontend documents.
- **Frontend (`frontend/`)** – Vite-built React entries for the list, task, and
  no-access pages, compiled into `assets/dist/` for the Rust server to serve.

Because the repository traits live in `src/repository/mod.rs`, service functions
accept generic parameters that implement those traits. This makes unit tests easy
by swapping in the `mockall`-based fakes from `src/repository/mock.rs`.

## Technology Stack

- Rust 2024 edition
- [Actix Web](https://actix.rs/) with identity and session middleware
- [Diesel](https://diesel.rs/) ORM with SQLite and connection pooling via r2d2
- [React](https://react.dev/) and [Vite](https://vite.dev/) for the user-facing
  task pages
- [`pushkind-common`](https://github.com/pushkindt/pushkind-common) shared crate
  for authentication guards, configuration, database helpers, and reusable
  patterns
- Supporting crates: `chrono`, `validator`, `serde`, `ammonia`, `csv`, and
  `thiserror`

## Getting Started

### Prerequisites

- Rust toolchain (install via [rustup](https://www.rust-lang.org/tools/install))
- `diesel-cli` with SQLite support (`cargo install diesel_cli --no-default-features --features sqlite`)
- SQLite 3 installed on your system

### Frontend Toolchain

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

Built frontend assets are required for `GET /`, `GET /task/{task_id}`, and
`GET /na`. If the expected document under `assets/dist/app/` is missing, the
corresponding page returns `503 Service Unavailable` until you run
`cd frontend && npm run build`.

### Configuration

Settings are layered via the [`config`](https://crates.io/crates/config) crate in the following order (later entries override earlier ones):

1. `config/default.yaml` (checked in)
2. `config/{APP_ENV}.yaml` where `APP_ENV` defaults to `local`
3. Environment variables prefixed with `APP_` (loaded automatically from a `.env` file via `dotenvy`)

Key settings you may want to override:

| Environment variable | Description | Default |
| --- | --- | --- |
| `APP_SECRET` | 64-byte secret used to sign identity and session cookies | _required_ |
| `APP_DATABASE_URL` | Path to the SQLite database file | `app.db` |
| `APP_ADDRESS` | Interface to bind | `127.0.0.1` |
| `APP_PORT` | HTTP port | `8080` when `APP_ENV=local` |
| `APP_DOMAIN` | Cookie domain (without protocol) | `test.me` when `APP_ENV=local` |
| `APP_ZMQ_EMAILER_PUB` | ZeroMQ PUB endpoint for outgoing email events | `tcp://127.0.0.1:5557` |
| `APP_AUTH_SERVICE_URL` | Base URL of the Pushkind authentication service | _required_ |
| `APP_CRM_SERVICE_URL` | Base URL of the Pushkind CRM service | _required_ |

Switch to the production profile with `APP_ENV=prod` or provide your own
`config/{env}.yaml`. Environment variables always win over YAML values, so a
local `.env` file containing `APP_SECRET=<64-byte key>` (generate with
`openssl rand -base64 64`) and any overrides will take effect without changing
the checked-in config files.

### Database

Run the Diesel migrations before starting the server:

```bash
diesel setup
cargo install diesel_cli --no-default-features --features sqlite # only once
diesel migration run
```

A SQLite file will be created at the location given by `DATABASE_URL`.

## Running the Application

Start the HTTP server with:

```bash
cargo run
```

The server listens on `http://127.0.0.1:8080` by default (from
`config/local.yaml`) and serves static assets from `./assets`, including the
built frontend documents under `assets/dist/`. Authentication and authorization
are enforced via the Pushkind auth service and the `SERVICE_ACCESS_ROLE`
constant.

## Quality Gates

The project treats formatting, linting, and tests as required gates before
opening a pull request. Use the following commands locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-features --tests -- -Dwarnings
cargo test --all-features --verbose
cargo build --all-features --verbose
```

Alternatively, the `make check` target will format the codebase, run clippy, and
execute the test suite in one step.

## Testing

Unit tests exercise the service and form layers directly, while integration
tests live under `tests/`. Repository tests rely on Diesel’s query builders and
should avoid raw SQL strings whenever possible. Use the mock repository module to
isolate services from the database when writing new tests.

## Project Principles

- **Domain-driven**: keep business rules in the domain and service layers and
  translate to/from external representations at the boundaries.
- **Explicit errors**: use `thiserror` to define granular error types and convert
  them into `ServiceError`/`RepositoryError` variants instead of relying on
  `anyhow`.
- **No panics in production paths**: avoid `unwrap`/`expect` in request handlers,
  services, and repositories—propagate errors instead.
- **Security aware**: sanitize any user-supplied HTML using `ammonia`, validate
  inputs with `validator`, and always enforce role checks with
  `pushkind_common::routes::ensure_role`.
- **Testable**: accept traits rather than concrete types in services and prefer
  dependency injection so the mock repositories can be used in tests.

Following these guidelines will help new functionality slot seamlessly into the
existing architecture and keep the service reliable in production.
