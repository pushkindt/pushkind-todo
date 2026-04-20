# AGENTS.md

This document provides guidance to AI code generators when working in this
repository. Follow these practices so that new code matches the established
architecture and conventions.

## Project Context

`pushkind-todo` is a Rust 2024 Actix Web application that uses Diesel with
SQLite, React-frontend, and the shared `pushkind-common` crate. The codebase is
layered into domain models, repository traits and implementations, service
modules, Actix routes, and forms. Business logic belongs in the
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
- Domain structs should expose strongly typed fields (e.g., `UserEmail`,
  `HubId`, `UserName`) that encode validation
  constraints. Construct these types at the boundaries (forms/services) so
  domain data is always trusted and cannot represent invalid input.
- Define error enums with `thiserror` inside the crate that owns the failure and
  return `RepositoryResult<T>` / `ServiceResult<T>` from repository and service
  functions.
- Services should return DTO-level structs when handing data to routes or other
  crates; perform domain-to-DTO conversion inside the service layer to keep
  handlers thin.
- Service functions should accept trait bounds (e.g., `TaskReader + TaskWriter`)
  so the `DieselRepository` and `mockall`-powered fakes remain interchangeable.
- Domain structs must not perform validation or normalization (e.g., no
  `to_lowercase`); assume inputs are already sanitized and transformed by forms
  or services before reaching the domain layer.
- Return domain types or simple success markers (`()`, counts, etc.) from
  services; keep HTTP concerns out of the service layer.
- Sanitize and validate user input early using `validator` and `ammonia` helpers
  from the form layer.
- Perform trimming, case normalisation, and other input clean-up before
  constructing domain types; domain builders assume callers supply sanitised
  values.
- Prefer dependency injection through function parameters over global state.
- For Diesel update models, avoid nested optionals; prefer single-layer `Option<T>`
  fields and rely on `#[diesel(treat_none_as_null = true)]` when nullable columns
  need to be cleared.
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

## HTTP and Frontend Guidelines

- Keep Actix handlers in `src/routes` focused on extracting inputs, invoking a
  service, and returning an HTTP response.
- Manage flash messages and redirects in the HTTP layer; services should not
  return HTTP-specific helper structs.
- Return DTOs to the React-frontend that only expose sanitized data. Use the
  shared component library in the frontend for a consistent UI.
- Respect the authorization checks via `pushkind_common::routes::ensure_role` and
  the `SERVICE_ACCESS_ROLE` constant.

## Testing Expectations

- Add unit tests for new service and form logic. When hitting the database, use
  Diesel migrations and helper constructors rather than hard-coded SQL strings.
- Use the mock repository module (`src/repository/mock.rs`) to isolate service
  tests from Diesel.
- Ensure new functionality is covered by tests before opening a pull request.

By following these principles the generated code will align with the project’s
architecture, technology stack, and long-term maintainability goals.

## Workflow Requirements

- Always obey `SPEC.md`.
- For any new work, require both `specs/features/<name>.md` and
  `plans/<name>.md`.
- If a change touches architecture, add or update an ADR under
  `specs/decisions/`.
