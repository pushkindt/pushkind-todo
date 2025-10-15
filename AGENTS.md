# AGENTS.md

This document provides guidance to AI code generators when working in this
repository. Follow these practices so that new code matches the established
architecture and conventions.

## Project Context

`pushkind-todo` is a Rust 2024 Actix Web application that uses Diesel with
SQLite, Tera templates, and the shared `pushkind-common` crate. The codebase is
layered into domain models, repository traits and implementations, service
modules, Actix routes, forms, and templates. Business logic belongs in the
service layer; handlers and repositories should stay thin and focused on I/O
concerns.

## Development Commands

Use these commands to verify your changes before committing:

**Build**
```bash
cargo build --all-features --verbose
```

**Run Tests**
```bash
cargo test --all-features --verbose
```

**Lint (Clippy)**
```bash
cargo clippy --all-features --tests -- -Dwarnings
```

**Format**
```bash
cargo fmt --all -- --check
```

## Coding Standards

- Use idiomatic Rust; avoid `unwrap` and `expect` in production paths.
- Keep modules focused: domain types in `src/domain`, Diesel models in
  `src/models`, and conversions implemented via `From`/`Into`.
- Define error enums with `thiserror` inside the crate that owns the failure and
  return `RepositoryResult<T>` / `ServiceResult<T>` from repository and service
  functions.
- Service functions should accept trait bounds (e.g., `TaskReader + TaskWriter`)
  so the `DieselRepository` and `mockall`-powered fakes remain interchangeable.
- Return domain types or simple success markers (`()`, counts, etc.) from
  services; keep HTTP concerns out of the service layer.
- Sanitize and validate user input early using `validator` and `ammonia` helpers
  from the form layer.
- Prefer dependency injection through function parameters over global state.
- Document all public APIs and any breaking changes.

## Database Guidelines

- Use Diesel’s query builder APIs with the generated `schema.rs` definitions; do
  not write raw SQL.
- Translate between Diesel structs (`src/models`) and domain types inside the
  repository layer using explicit `From` implementations.
- Reuse the filtering builders in `TaskListQuery`/`UserListQuery` when adding new
  queries and extend those structs rather than duplicating logic.
- Check related records (e.g., users) before inserts or updates and convert
  missing dependencies into `RepositoryError::NotFound` instead of panicking.

## HTTP and Template Guidelines

- Keep Actix handlers in `src/routes` focused on extracting inputs, invoking a
  service, and returning an HTTP response.
- Manage flash messages and redirects in the HTTP layer; services should not
  return HTTP-specific helper structs.
- Render templates with Tera contexts that only expose sanitized data. Use the
  existing component templates under `templates/` for shared UI.
- Respect the authorization checks via `pushkind_common::routes::check_role` and
  the `SERVICE_ACCESS_ROLE` constant.

## Testing Expectations

- Add unit tests for new service and form logic. When hitting the database, use
  Diesel migrations and helper constructors rather than hard-coded SQL strings.
- Use the mock repository module (`src/repository/mock.rs`) to isolate service
  tests from Diesel.
- Ensure new functionality is covered by tests before opening a pull request.

By following these principles the generated code will align with the project’s
architecture, technology stack, and long-term maintainability goals.
