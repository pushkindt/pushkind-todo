# Tasks: React Frontend Migration Phase 1

## Scope
This task file covers only Phase 1 from
[react-frontend-migration.md](../plans/react-frontend-migration.md):

- create the React + TypeScript + Vite frontend workspace
- emit build output into `assets/dist/`
- add a backend helper for loading the Vite manifest and opening built HTML
- add the required ADR because this work changes frontend runtime architecture
- document how to install dependencies and build frontend assets
- keep the live application on the current Tera runtime path

Do not start Phase 2 in this file. Phase 1 is complete only when the
repository can build frontend assets, Rust can understand those built assets,
and the live routes still render through the current Tera templates.

## References
- Service baseline:
  [../SPEC.md](../SPEC.md)
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)
- Migration plan:
  [../plans/react-frontend-migration.md](../plans/react-frontend-migration.md)
- Existing app wiring:
  [../src/lib.rs](../src/lib.rs)
- Existing documentation:
  [../README.md](../README.md)

## Preconditions
- Work in `/home/matrizaev/pushkind/pushkind-todo`.
- Treat the feature spec and migration plan as the source of truth.
- Do not change current route behavior in this phase.
- Do not add new `/api/v1/...` endpoints in this phase.
- Do not remove Tera templates or flash-message middleware in this phase.
- Do not cut over `GET /`, `GET /task/{task_id}`, or `GET /na` to built HTML
  in this phase.

## What You Will Change In Phase 1
You will change only these repository areas:

- create `specs/decisions/0001-react-frontend-runtime.md`
- create `frontend/` and its initial source tree
- create `src/frontend.rs`
- edit `src/lib.rs` to expose the new backend helper module
- edit `.gitignore`
- edit `README.md`

If you find yourself changing routes, templates, or service logic, stop. That
belongs to later phases.

## Deliverables
- `frontend/` exists with a working React + TypeScript + Vite setup.
- `assets/dist/` is the configured production build output directory.
- `assets/dist/manifest.json` is produced by `npm run build`.
- `src/frontend.rs` can:
  load a Vite manifest,
  resolve a named manifest entry,
  open a built HTML file.
- `README.md` explains how to install frontend dependencies and build assets.
- The application still renders the current Tera UI at runtime.

## Step 0: Baseline Snapshot
Run these commands first so you know what changed later:

```bash
pwd
git status --short
find frontend -maxdepth 3 -type f 2>/dev/null
find specs/decisions -maxdepth 2 -type f 2>/dev/null
cargo build --all-features --verbose
```

Expected result before you start:
- there is no `frontend/` directory in this repo yet
- there is no `specs/decisions/` directory yet
- the Rust project builds successfully before any React work is added

## Task 1: Add The Required ADR First
This migration changes frontend runtime architecture. The repo rules require an
ADR before implementation work starts.

### 1.1 Create The Decisions Directory
Run:

```bash
mkdir -p specs/decisions
```

### 1.2 Create `specs/decisions/0001-react-frontend-runtime.md`
Create [../specs/decisions/0001-react-frontend-runtime.md](../specs/decisions/0001-react-frontend-runtime.md)
with exactly this content:

```md
# ADR 0001: Adopt Incremental React Frontend With Vite-Built Documents

## Status
Proposed

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
```

### 1.3 Verify The ADR Exists
Run:

```bash
sed -n '1,240p' specs/decisions/0001-react-frontend-runtime.md
```

## Task 2: Create The Frontend Workspace
The goal here is to make the repository capable of building frontend assets
without changing live runtime behavior yet.

### 2.1 Create The Directory Tree
Run:

```bash
mkdir -p frontend/app
mkdir -p frontend/src/entries
mkdir -p frontend/src/components
mkdir -p frontend/src/pages
mkdir -p frontend/src/styles
mkdir -p frontend/src/lib
```

After that, your frontend tree should exist, even though it is still empty.

### 2.2 Create `frontend/package.json`
Create [../frontend/package.json](../frontend/package.json) with exactly this
content:

```json
{
  "name": "pushkind-todo-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "lint": "tsc --noEmit",
    "typecheck": "tsc --noEmit",
    "format": "prettier --write .",
    "format:check": "prettier --check ."
  },
  "dependencies": {
    "react": "^19.2.4",
    "react-dom": "^19.2.4"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^6.0.1",
    "jsdom": "^29.0.1",
    "prettier": "^3.8.1",
    "typescript": "^6.0.2",
    "vite": "^8.0.1",
    "vitest": "^4.1.0"
  }
}
```

Notes:
- `lint` and `typecheck` both use `tsc --noEmit` in Phase 1. That is enough
  for the scaffold.
- Do not add any screenshot-tool-specific scripts. Frontend verification in
  this repo is intentionally tool-agnostic.

### 2.3 Create `frontend/tsconfig.json`
Create [../frontend/tsconfig.json](../frontend/tsconfig.json) with exactly this
content:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ES2022"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx"
  },
  "include": ["src"]
}
```

### 2.4 Create `frontend/vite.config.ts`
Create [../frontend/vite.config.ts](../frontend/vite.config.ts) with exactly
this content:

```ts
import { resolve } from "node:path";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: "/assets/dist/",
  plugins: [react()],
  test: {
    environment: "jsdom",
    environmentOptions: {
      jsdom: {
        url: "http://localhost/",
      },
    },
    include: ["src/**/*.test.ts?(x)"],
  },
  build: {
    manifest: "manifest.json",
    outDir: resolve(__dirname, "../assets/dist"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        "app/index.html": resolve(__dirname, "app/index.html"),
        "app/task.html": resolve(__dirname, "app/task.html"),
        "app/no-access.html": resolve(__dirname, "app/no-access.html"),
      },
      output: {
        entryFileNames: "entries/[name]-[hash].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: ({ name }) => {
          if (name?.endsWith(".css")) {
            return "styles/[name]-[hash][extname]";
          }

          return "assets/[name]-[hash][extname]";
        },
      },
    },
  },
});
```

Why this exact config:
- `base: "/assets/dist/"` makes built asset URLs line up with Actix static
  serving from `/assets`.
- `outDir: "../assets/dist"` puts production build output where Rust will serve
  it.
- the `input` block forces Vite to emit separate HTML documents for the routes
  you plan to migrate later.

### 2.5 Create The Vite HTML Entry Documents
Create [../frontend/app/index.html](../frontend/app/index.html) with exactly
this content:

```html
<!doctype html>
<html lang="ru">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>ToDo</title>
    <link rel="icon" href="/assets/favicon.ico" type="image/x-icon" />
    <link
      href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css"
      rel="stylesheet"
      integrity="sha384-QWTKZyjpPEjISv5WaRU9OFeRpok6YctnYmDr5pNlyT2bRjXh0JMhjY6hW+ALEwIH"
      crossorigin="anonymous"
    />
    <link
      rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css"
    />
    <script type="module" src="/src/entries/index.tsx"></script>
  </head>
  <body class="bg-light">
    <div id="react-root"></div>
    <script
      src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/js/bootstrap.bundle.min.js"
      integrity="sha384-YvpcrYf0tY3lHB60NNkmXc5s9fDVZLESaAA55NDzOxhy9GkcIdslK1eN7N6jIeHz"
      crossorigin="anonymous"
    ></script>
  </body>
</html>
```

Create [../frontend/app/task.html](../frontend/app/task.html) with exactly this
content:

```html
<!doctype html>
<html lang="ru">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>ToDo Task</title>
    <link rel="icon" href="/assets/favicon.ico" type="image/x-icon" />
    <link
      href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css"
      rel="stylesheet"
      integrity="sha384-QWTKZyjpPEjISv5WaRU9OFeRpok6YctnYmDr5pNlyT2bRjXh0JMhjY6hW+ALEwIH"
      crossorigin="anonymous"
    />
    <link
      rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css"
    />
    <script type="module" src="/src/entries/task.tsx"></script>
  </head>
  <body class="bg-light">
    <div id="react-root"></div>
    <script
      src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/js/bootstrap.bundle.min.js"
      integrity="sha384-YvpcrYf0tY3lHB60NNkmXc5s9fDVZLESaAA55NDzOxhy9GkcIdslK1eN7N6jIeHz"
      crossorigin="anonymous"
    ></script>
  </body>
</html>
```

Create [../frontend/app/no-access.html](../frontend/app/no-access.html) with
exactly this content:

```html
<!doctype html>
<html lang="ru">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>ToDo No Access</title>
    <link rel="icon" href="/assets/favicon.ico" type="image/x-icon" />
    <link
      href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css"
      rel="stylesheet"
      integrity="sha384-QWTKZyjpPEjISv5WaRU9OFeRpok6YctnYmDr5pNlyT2bRjXh0JMhjY6hW+ALEwIH"
      crossorigin="anonymous"
    />
    <link
      rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.11.3/font/bootstrap-icons.min.css"
    />
    <script type="module" src="/src/entries/no-access.tsx"></script>
  </head>
  <body class="bg-light">
    <div id="react-root"></div>
    <script
      src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/js/bootstrap.bundle.min.js"
      integrity="sha384-YvpcrYf0tY3lHB60NNkmXc5s9fDVZLESaAA55NDzOxhy9GkcIdslK1eN7N6jIeHz"
      crossorigin="anonymous"
    ></script>
  </body>
</html>
```

### 2.6 Create The TypeScript Source Files
Create [../frontend/src/vite-env.d.ts](../frontend/src/vite-env.d.ts) with
exactly this content:

```ts
/// <reference types="vite/client" />
```

Create [../frontend/src/components/PhaseOneStatusCard.tsx](../frontend/src/components/PhaseOneStatusCard.tsx)
with exactly this content:

```tsx
type PhaseOneStatusCardProps = {
  badge: string;
  title: string;
  description: string;
  routeLabel: string;
};

export function PhaseOneStatusCard({
  badge,
  title,
  description,
  routeLabel,
}: PhaseOneStatusCardProps) {
  return (
    <div className="card border-0 shadow-sm">
      <div className="card-body p-4 p-lg-5">
        <span className="badge text-bg-secondary mb-3">{badge}</span>
        <h1 className="h3 mb-3">{title}</h1>
        <p className="text-body-secondary mb-4">{description}</p>
        <dl className="row mb-0">
          <dt className="col-sm-4">Маршрут</dt>
          <dd className="col-sm-8">
            <code className="phase-one-code">{routeLabel}</code>
          </dd>
          <dt className="col-sm-4">Статус</dt>
          <dd className="col-sm-8">
            В Phase 1 этот экран существует только как проверка frontend build
            pipeline. Живая страница по-прежнему рендерится через Tera.
          </dd>
        </dl>
      </div>
    </div>
  );
}
```

Create [../frontend/src/pages/PhaseOnePlaceholderPage.tsx](../frontend/src/pages/PhaseOnePlaceholderPage.tsx)
with exactly this content:

```tsx
import { PhaseOneStatusCard } from "../components/PhaseOneStatusCard";

type PhaseOnePlaceholderPageProps = {
  badge: string;
  title: string;
  description: string;
  routeLabel: string;
};

export function PhaseOnePlaceholderPage({
  badge,
  title,
  description,
  routeLabel,
}: PhaseOnePlaceholderPageProps) {
  return (
    <main className="phase-one-placeholder container py-4 py-lg-5">
      <div className="row justify-content-center">
        <div className="col-12 col-xl-8">
          <PhaseOneStatusCard
            badge={badge}
            title={title}
            description={description}
            routeLabel={routeLabel}
          />
        </div>
      </div>
    </main>
  );
}
```

Create [../frontend/src/pages/PhaseOnePlaceholderPage.test.tsx](../frontend/src/pages/PhaseOnePlaceholderPage.test.tsx)
with exactly this content:

```tsx
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { PhaseOnePlaceholderPage } from "./PhaseOnePlaceholderPage";

describe("PhaseOnePlaceholderPage", () => {
  it("renders the provided route label", () => {
    const markup = renderToStaticMarkup(
      <PhaseOnePlaceholderPage
        badge="Phase 1"
        title="ToDo frontend scaffold"
        description="Build-only placeholder"
        routeLabel="GET /"
      />,
    );

    expect(markup).toContain("Phase 1");
    expect(markup).toContain("GET /");
    expect(markup).toContain("ToDo frontend scaffold");
  });
});
```

Create [../frontend/src/lib/mount.tsx](../frontend/src/lib/mount.tsx) with
exactly this content:

```tsx
import type { ReactNode } from "react";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../styles/foundation.css";

export function mountPage(elementId: string, page: ReactNode): void {
  const rootElement = document.getElementById(elementId);

  if (!rootElement) {
    throw new Error(`Missing React mount node: #${elementId}`);
  }

  const root = createRoot(rootElement);
  root.render(<StrictMode>{page}</StrictMode>);
}
```

Create [../frontend/src/styles/foundation.css](../frontend/src/styles/foundation.css)
with exactly this content:

```css
:root {
  color-scheme: light;
}

body {
  min-height: 100vh;
}

.phase-one-placeholder {
  min-height: 100vh;
}

.phase-one-code {
  font-family: var(--bs-font-monospace, monospace);
}
```

Create [../frontend/src/entries/index.tsx](../frontend/src/entries/index.tsx)
with exactly this content:

```tsx
import { mountPage } from "../lib/mount";
import { PhaseOnePlaceholderPage } from "../pages/PhaseOnePlaceholderPage";

mountPage(
  "react-root",
  <PhaseOnePlaceholderPage
    badge="Phase 1"
    title="ToDo frontend scaffold"
    description="Этот документ нужен только для проверки Vite build pipeline. Живой маршрут GET / пока остаётся на Tera."
    routeLabel="GET /"
  />,
);
```

Create [../frontend/src/entries/task.tsx](../frontend/src/entries/task.tsx)
with exactly this content:

```tsx
import { mountPage } from "../lib/mount";
import { PhaseOnePlaceholderPage } from "../pages/PhaseOnePlaceholderPage";

mountPage(
  "react-root",
  <PhaseOnePlaceholderPage
    badge="Phase 1"
    title="ToDo task page scaffold"
    description="Этот документ нужен только для проверки Vite build pipeline. Живой маршрут GET /task/{task_id} пока остаётся на Tera."
    routeLabel="GET /task/{task_id}"
  />,
);
```

Create [../frontend/src/entries/no-access.tsx](../frontend/src/entries/no-access.tsx)
with exactly this content:

```tsx
import { mountPage } from "../lib/mount";
import { PhaseOnePlaceholderPage } from "../pages/PhaseOnePlaceholderPage";

mountPage(
  "react-root",
  <PhaseOnePlaceholderPage
    badge="Phase 1"
    title="ToDo no-access scaffold"
    description="Этот документ нужен только для проверки Vite build pipeline. Живой маршрут /na ещё не переключён на React."
    routeLabel="GET /na"
  />,
);
```

### 2.7 Install Frontend Dependencies
Run exactly these commands:

```bash
cd frontend
npm install
cd ..
```

Expected result:
- `frontend/package-lock.json` is created automatically
- `frontend/node_modules/` is created automatically

Do not hand-edit `frontend/package-lock.json`.

### 2.8 Verify The Frontend Workspace Alone
Run exactly these commands:

```bash
cd frontend
npm run typecheck
npm run test
npm run build
cd ..
find assets/dist -maxdepth 3 -type f | sort
sed -n '1,200p' assets/dist/manifest.json
```

Expected result:
- type-check succeeds
- the single test succeeds
- build succeeds
- `assets/dist/manifest.json` exists
- built HTML files exist under:
  `assets/dist/app/index.html`
  `assets/dist/app/task.html`
  `assets/dist/app/no-access.html`

If `npm run test` says no tests were found, you missed
`frontend/src/pages/PhaseOnePlaceholderPage.test.tsx`.

## Task 3: Add The Backend Frontend Helper Module
This code is preparation only. Do not wire it into routes yet.

### 3.1 Create `src/frontend.rs`
Create [../src/frontend.rs](../src/frontend.rs) with exactly this content:

```rust
//! Helpers for loading compiled frontend assets and opening built HTML documents.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use actix_files::NamedFile;
use serde::Deserialize;
use thiserror::Error;

/// Root directory for built frontend artifacts emitted by Vite.
pub const FRONTEND_DIST_DIR: &str = "assets/dist";

/// Relative path of the Vite manifest inside [`FRONTEND_DIST_DIR`].
pub const FRONTEND_MANIFEST_PATH: &str = "manifest.json";

/// Built HTML document that will eventually back `GET /`.
pub const FRONTEND_INDEX_DOCUMENT: &str = "app/index.html";

/// Built HTML document that will eventually back `GET /task/{task_id}`.
pub const FRONTEND_TASK_DOCUMENT: &str = "app/task.html";

/// Built HTML document that will eventually back `GET /na`.
pub const FRONTEND_NO_ACCESS_DOCUMENT: &str = "app/no-access.html";

/// Parsed subset of the Vite manifest entry fields this service needs.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FrontendManifestEntry {
    pub file: String,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(rename = "isEntry", default)]
    pub is_entry: bool,
    #[serde(default)]
    pub src: Option<String>,
}

/// Parsed Vite manifest keyed by the original entry name.
pub type FrontendManifest = BTreeMap<String, FrontendManifestEntry>;

/// Errors raised while reading, parsing, or resolving frontend assets.
#[derive(Debug, Error)]
pub enum FrontendAssetError {
    #[error("failed to read frontend asset: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse frontend manifest: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("frontend manifest entry not found: {0}")]
    MissingEntry(String),
}

/// Build an absolute-on-repo path under [`FRONTEND_DIST_DIR`].
pub fn frontend_dist_path(relative_path: impl AsRef<Path>) -> PathBuf {
    Path::new(FRONTEND_DIST_DIR).join(relative_path)
}

/// Load and parse a Vite manifest from the provided path.
pub fn load_frontend_manifest_from_path(
    path: impl AsRef<Path>,
) -> Result<FrontendManifest, FrontendAssetError> {
    let manifest_json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&manifest_json)?)
}

/// Load and parse the default Vite manifest emitted by this service.
pub fn load_default_frontend_manifest() -> Result<FrontendManifest, FrontendAssetError> {
    load_frontend_manifest_from_path(frontend_dist_path(FRONTEND_MANIFEST_PATH))
}

/// Resolve a named manifest entry from a parsed Vite manifest.
pub fn resolve_manifest_entry<'a>(
    manifest: &'a FrontendManifest,
    entry_name: &str,
) -> Result<&'a FrontendManifestEntry, FrontendAssetError> {
    manifest
        .get(entry_name)
        .ok_or_else(|| FrontendAssetError::MissingEntry(entry_name.to_string()))
}

/// Open a Vite-built HTML document for a React-owned route.
pub async fn open_frontend_html(path: impl AsRef<Path>) -> Result<NamedFile, FrontendAssetError> {
    let file = NamedFile::open_async(path).await?;
    Ok(file.use_last_modified(true).prefer_utf8(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn frontend_dist_path_joins_relative_path() {
        let path = frontend_dist_path("app/index.html");
        assert_eq!(path, Path::new("assets/dist").join("app/index.html"));
    }

    #[test]
    fn loads_manifest_from_disk() {
        let dir = tempdir().expect("tempdir should be created");
        let manifest_path = dir.path().join("manifest.json");

        std::fs::write(
            &manifest_path,
            r#"{
  "app/index.html": {
    "file": "entries/app/index-abc123.js",
    "css": ["styles/app/index-abc123.css"],
    "imports": ["chunks/shared-def456.js"],
    "isEntry": true,
    "src": "app/index.html"
  }
}"#,
        )
        .expect("manifest should be written");

        let manifest = load_frontend_manifest_from_path(&manifest_path)
            .expect("manifest should parse successfully");
        let entry = resolve_manifest_entry(&manifest, "app/index.html")
            .expect("entry should exist");

        assert_eq!(entry.file, "entries/app/index-abc123.js");
        assert_eq!(entry.css, vec!["styles/app/index-abc123.css".to_string()]);
        assert_eq!(entry.imports, vec!["chunks/shared-def456.js".to_string()]);
        assert!(entry.is_entry);
        assert_eq!(entry.src.as_deref(), Some("app/index.html"));
    }

    #[test]
    fn missing_manifest_entry_returns_error() {
        let manifest = FrontendManifest::new();

        let error = resolve_manifest_entry(&manifest, "app/index.html")
            .expect_err("missing entry should return an error");

        assert!(matches!(
            error,
            FrontendAssetError::MissingEntry(ref name) if name == "app/index.html"
        ));
    }

    #[test]
    fn can_open_existing_file() {
        let dir = tempdir().expect("tempdir should be created");
        let html_path = dir.path().join("index.html");
        std::fs::write(&html_path, "<!doctype html><title>ok</title>")
            .expect("html file should be written");

        let result = actix_web::rt::System::new().block_on(open_frontend_html(&html_path));
        assert!(result.is_ok());
    }

    #[test]
    fn missing_document_returns_read_error() {
        let error = actix_web::rt::System::new()
            .block_on(open_frontend_html("assets/dist/does-not-exist.html"))
            .expect_err("missing file should return an error");

        assert!(matches!(error, FrontendAssetError::Read(_)));
    }
}
```

### 3.2 Edit `src/lib.rs` To Export The New Module
Open [../src/lib.rs](../src/lib.rs).

Find this existing block:

```rust
#[cfg(feature = "server")]
pub mod error_conversions;
#[cfg(feature = "server")]
pub mod forms;
#[cfg(feature = "data")]
pub mod models;
```

Change it to this:

```rust
#[cfg(feature = "server")]
pub mod error_conversions;
#[cfg(feature = "server")]
pub mod forms;
#[cfg(feature = "server")]
pub mod frontend;
#[cfg(feature = "data")]
pub mod models;
```

Do not make any other `src/lib.rs` changes in Phase 1.

### 3.3 Verify Only The New Backend Helper Compiles
Run:

```bash
cargo test --all-features frontend::tests -- --nocapture
```

Expected result:
- the `src/frontend.rs` tests pass
- no routes or templates were touched

## Task 4: Add Ignore Rules
You want generated JS dependencies and build artifacts out of version control.

### 4.1 Edit `.gitignore`
Open [../.gitignore](../.gitignore).

Append these two lines at the end of the file:

```gitignore
frontend/node_modules/
assets/dist/
```

After editing, the full file should look like this:

```gitignore
/target
.env
app.*
.agent/
tarpaulin-report*
frontend/node_modules/
assets/dist/
```

### 4.2 Verify Ignore Rules
Run:

```bash
git status --short
```

Expected result:
- `frontend/package-lock.json` should be visible as a tracked file candidate
- `frontend/node_modules/` should not flood `git status`
- `assets/dist/` should not flood `git status`

## Task 5: Update `README.md`
Phase 1 changes local developer workflow, even though it does not yet change
live runtime behavior.

### 5.1 Insert A New Frontend Toolchain Section
Open [../README.md](../README.md).

Find the `### Prerequisites` list under `## Getting Started` and insert the
following new subsection immediately after the bullet list that ends with
`SQLite 3 installed on your system`:

````md
### Frontend Toolchain (Phase 1)

Phase 1 of the React migration adds a frontend workspace under `frontend/`.
The application runtime still serves the current Tera UI in this phase, so
`cargo run` does **not** require a prior frontend build yet.

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

Phase 1 note:

- built frontend assets are required for frontend verification commands
- built frontend assets are not yet used by `GET /`, `GET /task/{task_id}`, or
  `/na`
- those routes continue to use the current Tera templates until later phases
````

Be careful when editing markdown:
- keep the indentation exactly as normal README prose
- do not wrap the inserted block inside another list
- do not remove any existing README sections

### 5.2 Verify The README Change
Run:

```bash
rg -n "Frontend Toolchain \\(Phase 1\\)|assets/dist|cargo run does \\*\\*not\\*\\* require" README.md
sed -n '1,220p' README.md
```

Expected result:
- the new section is present
- the README clearly says Phase 1 still uses Tera at runtime

## Task 6: Confirm Runtime Behavior Is Still Unchanged
Phase 1 is scaffolding only. You must actively confirm you did not start
Phase 2 by accident.

### 6.1 Confirm The New Frontend Helper Is Not Used By Routes Yet
Run:

```bash
rg -n "open_frontend_html|load_default_frontend_manifest|resolve_manifest_entry|FRONTEND_INDEX_DOCUMENT|FRONTEND_TASK_DOCUMENT|FRONTEND_NO_ACCESS_DOCUMENT" src
```

Expected result:
- matches should appear only in `src/frontend.rs`
- there should be no matches in `src/routes/*.rs`

### 6.2 Confirm Tera Is Still The Live Runtime Path
Run:

```bash
rg -n "Tera::new|render_template|FlashMessagesFramework" src/lib.rs src/routes
```

Expected result:
- Tera is still initialized in `src/lib.rs`
- current routes still call `render_template`
- flash-message middleware is still wired

### 6.3 Confirm No New APIs Were Added
Run:

```bash
git diff -- src/routes src/services src/dto
```

Expected result:
- only `src/frontend.rs` and the `src/lib.rs` module export change should be
  present
- there should be no new route handlers or DTO changes in Phase 1

## Task 7: Full Phase 1 Verification
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
- the frontend scaffold type-checks
- the frontend scaffold test passes
- Vite produces `assets/dist/manifest.json`
- Rust builds and tests still pass
- Clippy reports no warnings
- formatting is clean
- `frontend/node_modules/` and `assets/dist/` stay ignored
- the only intentional tracked changes are:
  `specs/decisions/0001-react-frontend-runtime.md`
  `frontend/package.json`
  `frontend/package-lock.json`
  `frontend/tsconfig.json`
  `frontend/vite.config.ts`
  `frontend/app/index.html`
  `frontend/app/task.html`
  `frontend/app/no-access.html`
  `frontend/src/...`
  `src/frontend.rs`
  `src/lib.rs`
  `.gitignore`
  `README.md`

## Expected Repository State After Phase 1
If you have done Phase 1 correctly, this tree should exist:

```text
specs/
  decisions/
    0001-react-frontend-runtime.md
frontend/
  app/
    index.html
    no-access.html
    task.html
  package-lock.json
  package.json
  tsconfig.json
  vite.config.ts
  src/
    components/
      PhaseOneStatusCard.tsx
    entries/
      index.tsx
      no-access.tsx
      task.tsx
    lib/
      mount.tsx
    pages/
      PhaseOnePlaceholderPage.test.tsx
      PhaseOnePlaceholderPage.tsx
    styles/
      foundation.css
    vite-env.d.ts
src/
  frontend.rs
```

## Phase 1 Exit Checklist
Mark Phase 1 done only if all of the following are true:

- the ADR exists under `specs/decisions/`
- `frontend/` exists with React, TypeScript, and Vite configured
- `frontend/package-lock.json` is committed
- `npm run build` emits assets into `assets/dist/`
- `assets/dist/manifest.json` is produced by the build
- Rust can parse a Vite manifest and resolve an entry from it
- Rust can open a built HTML file with the helper in `src/frontend.rs`
- `.gitignore` excludes `frontend/node_modules/` and `assets/dist/`
- `README.md` explains the new frontend toolchain
- live routes still render via Tera and flash-message middleware

## Explicit Non-Goals For This Task File
Do not do any of the following here:

- switch `GET /` to built frontend HTML
- switch `GET /task/{task_id}` to built frontend HTML
- add a local `/na` route
- add `/api/v1/iam`
- add `/api/v1/tasks/{task_id}`
- add `/api/v1/users`
- add `/api/v1/clients`
- add `/api/v1/tracks`
- add JSON mutation endpoints
- migrate any template markup to React
- remove `tera`
- remove `actix-web-flash-messages`
- remove HTMX
- remove any existing template file
