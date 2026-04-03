# Tasks: React Frontend Migration Phase 6

## Scope
This task file covers only Phase 6 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- remove obsolete Tera templates and HTML fragment flows once they are unused
  by user-facing routes
- remove inline template JavaScript and HTMX runtime paths that React now owns
- remove direct `tera` and `actix-web-flash-messages` runtime dependencies once
  all migrated pages and mutations no longer need them
- remove stale config and dead server wiring that existed only for Tera-based
  rendering
- update README and operational docs for the final frontend build/runtime shape

Do not start a new feature phase in this file. Phase 6 is complete only when
the live ToDo pages still serve the same URLs and backend business rules, but
no user-facing ToDo page depends on Tera, flash-message middleware, HTMX, or
template-owned interaction code.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Frontend runtime ADR:
  [../specs/decisions/0001-react-frontend-runtime.md](../specs/decisions/0001-react-frontend-runtime.md)
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
- Phase 5 task file:
  [../plans/react-frontend-migration-phase-5-tasks.md](../plans/react-frontend-migration-phase-5-tasks.md)
- Current runtime wiring and config:
  [../Cargo.toml](../Cargo.toml)
  [../src/lib.rs](../src/lib.rs)
  [../src/models/config.rs](../src/models/config.rs)
  [../config/default.yaml](../config/default.yaml)
- Current route/service cleanup surface:
  [../src/routes/main.rs](../src/routes/main.rs)
  [../src/routes/task.rs](../src/routes/task.rs)
  [../src/routes/aux.rs](../src/routes/aux.rs)
  [../src/services/task.rs](../src/services/task.rs)
  [../src/dto/task.rs](../src/dto/task.rs)
- Current template tree and shared template runtime:
  [../templates/base.html](../templates/base.html)
  [../templates/components/macros.html](../templates/components/macros.html)
  [../templates/components/navigation.html](../templates/components/navigation.html)
  [../templates/main/index.html](../templates/main/index.html)
  [../templates/task/index.html](../templates/task/index.html)
  [../templates/task/modal_body.html](../templates/task/modal_body.html)
- Current final-user documentation:
  [../README.md](../README.md)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-todo`.
- Treat the feature spec, ADR, and migration plan as the source of truth.
- Assume Phase 5 is already complete:
  `GET /`,
  `GET /task/{task_id}`,
  and `GET /na` are React-backed built documents,
  and React-owned JSON mutations already cover create, upload, update, quick
  status, comment, and delete flows.
- Confirm the live React task page no longer depends on
  `/task/{task_id}/modal`.
- Confirm the live React list page no longer depends on the legacy HTML POST
  routes in `src/routes/main.rs`.
- Preserve the current URLs and backend business rules:
  no SPA routing,
  no auth model redesign,
  no validation/notification/ZeroMQ behavior changes.
- If cleanup reveals a real architecture divergence from
  [0001-react-frontend-runtime.md](../specs/decisions/0001-react-frontend-runtime.md),
  update the ADR rather than leaving undocumented drift.

## What You Will Change In Phase 6
You will change only these repository areas:

- edit `Cargo.toml`
- edit `src/lib.rs`
- edit `src/models/config.rs`
- edit `config/default.yaml`
- edit `src/routes/main.rs`
- edit `src/routes/task.rs`
- edit `src/routes/aux.rs` only if a small cleanup is required after removing
  flash middleware
- edit `src/services/task.rs`
- edit `src/dto/task.rs`
- delete obsolete files under `templates/`
- edit `README.md`
- update tests that still assume legacy template/runtime wiring

If you find yourself redesigning the React pages, changing `/api/v1/...`
contracts, or altering backend task semantics, stop. That is not Phase 6.

## Deliverables
- Obsolete templates and template fragments are deleted once they are proven
  unused by live user-facing routes.
- No live ToDo route depends on `render_template`, `Tera::new(...)`, or Tera
  app data.
- No live ToDo user flow depends on `FlashMessage`, flash-message middleware,
  or flash-driven redirect feedback.
- No live ToDo user flow depends on HTMX or template-owned inline page scripts.
- Dead HTML task mutation routes and the legacy task modal route are removed
  once there is no remaining caller.
- `Cargo.toml` no longer carries direct `tera` or
  `actix-web-flash-messages` runtime dependencies.
- `ServerConfig` and checked-in config no longer include `templates_dir`.
- README and runtime docs describe the final React/Vite build flow rather than
  the old Tera-based frontend.
- `GET /`, `GET /task/{task_id}`, and `GET /na` still serve the same pages
  through built frontend documents after backend auth checks.

## Step 0: Confirm The Starting Point
Run these commands before you make any Phase 6 changes:

```bash
pwd
git status --short
sed -n '1,260p' Cargo.toml
sed -n '1,260p' src/lib.rs
sed -n '1,260p' src/models/config.rs
sed -n '1,220p' config/default.yaml
sed -n '1,260p' src/routes/main.rs
sed -n '1,320p' src/routes/task.rs
sed -n '1,240p' README.md
find templates -maxdepth 3 -type f | sort
rg -n "tera|FlashMessage|flash-messages|Tera::new|render_template\\(|task_modal|load_task_modal|htmx|APP_TEMPLATES_DIR|templates_dir" src templates README.md config
```

Expected result before Phase 6 starts:
- `Cargo.toml` still enables `tera` and `actix-web-flash-messages`
- `src/lib.rs` still builds `FlashMessagesFramework`, initializes `Tera`, and
  mounts `task_modal`
- `src/models/config.rs` and `config/default.yaml` still include
  `templates_dir`
- the `templates/` tree still exists even though the main pages are now React
  documents
- `templates/base.html` still contains HTMX and shared inline JavaScript
- `src/routes/main.rs` and `src/routes/task.rs` still contain flash-driven HTML
  POST handlers
- `src/services/task.rs` still contains `load_task_modal`
- `README.md` still describes Tera/server-rendered pages and
  `APP_TEMPLATES_DIR`

## Task 1: Prove The Legacy HTML Flow Is Dead Before Deleting It
Phase 6 is cleanup, but cleanup must be evidence-based. Confirm that the live
React pages already cover the same user-facing behavior before removing legacy
code.

### 1.1 Audit Remaining Legacy Callers
Search for callers of these legacy paths and helpers:

- `POST /task/add`
- `POST /tasks/upload`
- `POST /task/{task_id}/update`
- `POST /task/{task_id}/status`
- `POST /task/{task_id}/comments`
- `POST /task/{task_id}/delete`
- `POST /task/{task_id}/modal`
- `load_task_modal`
- `render_template(...)`

Only delete a route or helper after you can show there is no remaining
user-facing caller outside the files being deleted in this phase.

### 1.2 Cleanup Rule
Apply this rule throughout Phase 6:

- if a legacy route still has a real live caller, keep it and document the
  blocker
- if the only remaining callers are dead templates or dead docs being removed
  in this phase, delete the route/helper with them

## Task 2: Remove Legacy HTML Mutation Routes And Modal Glue
The React pages now own mutation UX. The old HTML POST handlers and modal
fragment route should not survive just for convenience.

### 2.1 Edit `src/routes/main.rs`
Remove the old list-page HTML POST handlers once the React list page no longer
depends on them:

- `POST /task/add`
- `POST /tasks/upload`

Keep `GET /` intact because it still serves the built frontend document.

### 2.2 Edit `src/routes/task.rs`
Remove the old task-page HTML handlers once the React task page no longer
depends on them:

- `POST /task/{task_id}/modal`
- `POST /task/{task_id}/update`
- `POST /task/{task_id}/status`
- `POST /task/{task_id}/comments`
- `POST /task/{task_id}/delete`

Keep `GET /task/{task_id}` intact because it still serves the built frontend
document.

### 2.3 Edit `src/services/task.rs` And `src/dto/task.rs`
Delete the modal-only dead code after the route is gone:

- `load_task_modal`
- `TaskModalData`
- any imports, tests, or helper code that existed only for the Tera modal flow

Do not delete `TaskDetails` or other DTOs/helpers that still back the typed
API surface.

### 2.4 Edit `src/lib.rs`
Stop mounting the deleted HTML mutation routes and modal route under the main
scope once they are removed.

## Task 3: Remove Tera From Runtime Wiring
Once no live route renders templates, remove Tera from the runtime rather than
carrying it as dead infrastructure.

### 3.1 Edit `src/lib.rs`
Remove:

- `tera::Tera`
- Tera initialization
- Tera app data registration
- imports that existed only for template rendering

After this step, the server should still serve the built frontend documents and
the `/api/v1/...` JSON surface exactly as before.

### 3.2 Edit `Cargo.toml`
Remove the direct `tera` dependency and feature wiring once the server no
longer uses it.

### 3.3 Edit Config Surface
Remove Tera-specific config that no longer has a runtime consumer:

- `templates_dir` from [config/default.yaml](../config/default.yaml)
- `templates_dir` from [src/models/config.rs](../src/models/config.rs)
- `APP_TEMPLATES_DIR` from [README.md](../README.md)

Do not leave dead config keys in place “just in case”.

## Task 4: Remove Flash-Message Middleware From Runtime
React-owned mutations already return structured JSON. User-facing ToDo pages
should no longer depend on flash-driven redirects or flash banners.

### 4.1 Replace Flash Usage In Remaining Routes
Update the remaining GET routes that still use flash-based role feedback:

- `GET /`
- `GET /task/{task_id}`

If authorization fails, redirect to `/na` or return the appropriate response
without using `FlashMessage`.

### 4.2 Edit `src/lib.rs`
Remove flash middleware setup once no route depends on it:

- `FlashMessagesFramework`
- `CookieMessageStore`
- associated `.wrap(...)` wiring

### 4.3 Edit `Cargo.toml`
Remove the direct `actix-web-flash-messages` dependency and feature wiring
once the runtime no longer uses it.

### 4.4 Documentation Rule
Where README currently says secrets sign “cookies and flash messages”, update
the wording to reflect the final cookie/session usage only.

## Task 5: Delete The Obsolete Template Tree And HTMX Runtime
Once the server no longer renders templates, the template tree and HTMX-based
interaction code should be removed rather than left as dead baggage.

### 5.1 Delete Obsolete Templates
Delete the Tera files that are no longer used by live ToDo routes:

- `templates/base.html`
- `templates/components/macros.html`
- `templates/components/navigation.html`
- `templates/main/index.html`
- `templates/main/add_task_modal.html`
- `templates/main/filter_task_modal.html`
- `templates/main/not_assigned.html`
- `templates/task/index.html`
- `templates/task/modal_body.html`

If all files under `templates/` are deleted, remove the empty directory as
part of normal cleanup.

### 5.2 Remove HTMX And Template JS References
After the template files are gone:

- there should be no repository references to `htmx`
- there should be no shared template-owned inline page scripts remaining
- there should be no dead TomSelect setup code that only existed in templates

This phase is where HTMX finally leaves the runtime path.

## Task 6: Update README And Operational Documentation
The README currently describes a Tera-powered UI and Phase-based temporary
states that should not survive the final cleanup.

### 6.1 Edit `README.md`
Update [../README.md](../README.md) so it reflects the final architecture:

- describe the frontend as React-managed built documents served by Rust after
  auth checks
- remove Tera/server-rendered UI language
- remove flash-message UX language
- describe the current pages as React-backed:
  `/`,
  `/task/{task_id}`,
  `/na`
- update the frontend build section so it describes the final required runtime
  artifacts rather than Phase-specific temporary notes
- remove `APP_TEMPLATES_DIR` from the config table

### 6.2 Update Any Other Tera-Specific Operational References
If there are other local docs or comments that still describe Tera as a live
runtime dependency, update or delete them in this phase.

## Task 7: Keep The Final Runtime Lean
Phase 6 should leave the repo cleaner, not just “working by accident”.

### 7.1 Dead-Code Rule
After removing templates, Tera, flash middleware, and HTMX:

- remove dead imports
- remove dead helper functions
- remove dead tests that only validated removed Tera-only behavior
- keep or extend tests that validate the surviving React/API runtime

### 7.2 Lockfile And Dependency Hygiene
If dependency removal updates `Cargo.lock`, keep the lockfile in sync with the
final dependency set.

Do not leave direct runtime dependencies in `Cargo.toml` that no code uses.

## Task 8: Verification
After cleanup, run the full verification set:

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

- open `GET /`
- open `GET /task/{task_id}`
- open `GET /na`
- verify unauthorized access still lands on the local `/na` page
- verify task creation, upload, update, completion, commenting, and deletion
  still work through the JSON API-backed React flow
- verify no user-facing ToDo route depends on Tera, flash messages, or HTMX

## Expected Repository State After Phase 6
After Phase 6 is complete:

- there is no live Tera rendering path for user-facing ToDo pages
- the obsolete `templates/` files are gone
- `src/lib.rs` no longer initializes Tera or flash-message middleware
- `Cargo.toml` no longer directly depends on `tera` or
  `actix-web-flash-messages`
- `ServerConfig` no longer includes `templates_dir`
- README documents the final React/Vite runtime shape
- the same user-facing ToDo page URLs still work with the same backend rules

## Phase 6 Exit Checklist
- no live ToDo route calls `render_template(...)`
- no live ToDo route depends on `FlashMessage` or flash-message middleware
- no live ToDo route depends on HTMX or template-owned inline JavaScript
- obsolete HTML mutation routes and the legacy task modal route are removed
- dead modal-only service/DTO code is removed
- direct `tera` and `actix-web-flash-messages` dependencies are removed
- stale template config is removed from code, config, and docs
- README reflects the final React-backed runtime
- the server still serves the same user-facing page URLs with the same backend
  business rules

## Explicit Non-Goals For This Task File
- redesigning the React UI
- changing `/api/v1/...` contracts unless cleanup reveals a real bug
- changing task domain semantics, repository rules, auth model, or ZeroMQ
  payload meaning
- introducing client-side routing or SPA behavior
- adding new end-user features instead of removing dead migration scaffolding
