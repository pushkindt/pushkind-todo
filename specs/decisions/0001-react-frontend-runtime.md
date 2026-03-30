# ADR 0001: Adopt Incremental React Frontend With Vite-Built Documents

## Status
Stable

## References
- Service baseline:
  [../../SPEC.md](../../SPEC.md)
- Feature spec:
  [../features/react-frontend-migration.md](../features/react-frontend-migration.md)


## Context
`pushkind-todo` currently renders its user-facing pages with Tera templates and
augments that markup with inline JavaScript, Bootstrap behaviors, HTMX modal
fragments, and flash-driven redirect flows.

The approved frontend migration goal is to move the user-facing UI to React
while preserving:
- the existing server-routed URLs
- the non-SPA navigation model
- Bootstrap styling
- Russian copy
- backend-owned authorization, validation, sanitization, notification, ZeroMQ,
  and persistence rules

That means the frontend migration cannot be implemented as a SPA rewrite or as
a client-routed application without changing the repository specification.

## Decision
- Keep Actix routes and server-side request handling as the source of truth for
  navigation, redirects, authentication, and authorization.
- Introduce React incrementally on the existing URLs.
- Do not introduce client-side routing.
- Place frontend source code under `frontend/`.
- Build frontend assets and HTML documents with Vite into `assets/dist/`.
- Let Rust serve built HTML documents after performing route-level access
  checks.
- Move page initialization to typed `/api/v1/...` JSON APIs instead of
  embedding more page data into server-generated HTML.
- Keep Tera only as a migration wrapper until React equivalents are shipped and
  verified.
- Keep flash-message middleware only until React-owned mutation flows replace
  redirect-based feedback.

## Consequences

### Positive
- React can be introduced without rewriting the backend architecture.
- The migration can proceed incrementally by route and interaction.
- Built frontend artifacts are served directly by the Rust application.
- The final runtime model becomes clearer: Rust owns routes and APIs, React
  owns page UI.

### Negative
- The service will temporarily carry both Tera and React concerns.
- A Node-based frontend toolchain becomes part of local development and CI.
- Some endpoints and flows will temporarily exist in both legacy and migrated
  forms during rollout.

## Rejected Alternatives
- Full SPA rewrite:
  rejected because it conflicts with the approved spec and would widen scope
  beyond a frontend migration.
- Continue with Tera + inline JavaScript + HTMX:
  rejected because it does not achieve the approved React migration target.
- Keep Rust assembling HTML document shells permanently:
  rejected because the target state explicitly gives frontend document
  ownership to Vite-built static HTML.
