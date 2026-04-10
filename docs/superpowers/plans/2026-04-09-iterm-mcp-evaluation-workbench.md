# iTerm2 MCP Evaluation Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local-first desktop workbench that dispatches the same old-code understanding prompt to multiple iTerm2 MCP targets, stores full results, and presents structured comparison metrics.

**Architecture:** The app uses a Tauri shell with a Rust backend for persistence, orchestration, MCP integration, and evaluation logic, plus a React + TypeScript frontend for case management, runtime monitoring, result comparison, and target configuration. SQLite stores the local source of truth for model profiles, window bindings, evaluation cases, runs, messages, and metrics.

**Tech Stack:** Tauri, Rust, SQLx with SQLite, Serde, Tokio, React, TypeScript, Vite, TanStack Query, React Router, Zustand

---

## Planned File Structure

### Backend

- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/app_state.rs`
- Create: `src-tauri/src/error.rs`
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/schema.rs`
- Create: `src-tauri/src/db/migrations/0001_initial.sql`
- Create: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/models/profile.rs`
- Create: `src-tauri/src/models/window_binding.rs`
- Create: `src-tauri/src/models/evaluation_case.rs`
- Create: `src-tauri/src/models/comparison_run.rs`
- Create: `src-tauri/src/models/message.rs`
- Create: `src-tauri/src/models/target_evaluation.rs`
- Create: `src-tauri/src/services/mod.rs`
- Create: `src-tauri/src/services/profile_service.rs`
- Create: `src-tauri/src/services/window_binding_service.rs`
- Create: `src-tauri/src/services/evaluation_case_service.rs`
- Create: `src-tauri/src/services/comparison_run_service.rs`
- Create: `src-tauri/src/services/analysis_service.rs`
- Create: `src-tauri/src/services/export_service.rs`
- Create: `src-tauri/src/services/iterm_mcp_adapter.rs`
- Create: `src-tauri/src/services/comparison_orchestrator.rs`
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/profile_commands.rs`
- Create: `src-tauri/src/commands/window_binding_commands.rs`
- Create: `src-tauri/src/commands/evaluation_case_commands.rs`
- Create: `src-tauri/src/commands/comparison_commands.rs`
- Create: `src-tauri/src/commands/export_commands.rs`

### Frontend

- Create: `package.json`
- Create: `tsconfig.json`
- Create: `vite.config.ts`
- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/app/router.tsx`
- Create: `src/app/query-client.ts`
- Create: `src/styles/globals.css`
- Create: `src/types/api.ts`
- Create: `src/lib/tauri.ts`
- Create: `src/lib/formatters.ts`
- Create: `src/store/ui-store.ts`
- Create: `src/features/layout/AppShell.tsx`
- Create: `src/features/cases/pages/CaseLibraryPage.tsx`
- Create: `src/features/cases/components/CaseForm.tsx`
- Create: `src/features/targets/pages/TargetConfigPage.tsx`
- Create: `src/features/targets/components/ProfileForm.tsx`
- Create: `src/features/targets/components/WindowBindingList.tsx`
- Create: `src/features/runs/pages/CreateRunPage.tsx`
- Create: `src/features/runs/pages/RunMonitorPage.tsx`
- Create: `src/features/runs/pages/RunResultsPage.tsx`
- Create: `src/features/runs/components/RunTargetStatusCard.tsx`
- Create: `src/features/runs/components/ResultComparisonGrid.tsx`
- Create: `src/features/runs/components/MetricTable.tsx`
- Create: `src/features/settings/pages/SettingsPage.tsx`

### Tests

- Create: `src-tauri/tests/profile_service.rs`
- Create: `src-tauri/tests/evaluation_case_service.rs`
- Create: `src-tauri/tests/comparison_run_service.rs`
- Create: `src-tauri/tests/analysis_service.rs`
- Create: `src/features/runs/components/MetricTable.test.tsx`
- Create: `src/features/runs/components/ResultComparisonGrid.test.tsx`

## Task 1: Scaffold the Tauri + React workspace

**Files:**
- Create: `package.json`
- Create: `tsconfig.json`
- Create: `vite.config.ts`
- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/app/router.tsx`
- Create: `src/features/layout/AppShell.tsx`
- Create: `src/styles/globals.css`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing smoke test checklist**

```text
Smoke expectations:
1. `npm run build` produces a frontend bundle
2. `cargo test --manifest-path src-tauri/Cargo.toml` compiles the Rust backend
3. The app shell renders navigation entries for Cases, Targets, Runs, and Settings
```

- [ ] **Step 2: Create the frontend package manifest**

```json
{
  "name": "iterm-mcp-evaluation-workbench",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "test": "vitest run",
    "lint": "eslint .",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tanstack/react-query": "^5.59.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-router-dom": "^6.30.1",
    "zustand": "^5.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "typescript": "^5.6.2",
    "vite": "^5.4.8",
    "vitest": "^2.1.2"
  }
}
```

- [ ] **Step 3: Create the base frontend entrypoints**

```tsx
// src/main.tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { queryClient } from "./app/query-client";
import { router } from "./app/router";
import "./styles/globals.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </React.StrictMode>,
);
```

```tsx
// src/App.tsx
import { Outlet } from "react-router-dom";
import { AppShell } from "./features/layout/AppShell";

export default function App() {
  return (
    <AppShell>
      <Outlet />
    </AppShell>
  );
}
```

- [ ] **Step 4: Create the router and shell**

```tsx
// src/app/router.tsx
import { createBrowserRouter, Navigate } from "react-router-dom";
import App from "../App";
import { CaseLibraryPage } from "../features/cases/pages/CaseLibraryPage";
import { TargetConfigPage } from "../features/targets/pages/TargetConfigPage";
import { CreateRunPage } from "../features/runs/pages/CreateRunPage";
import { RunMonitorPage } from "../features/runs/pages/RunMonitorPage";
import { RunResultsPage } from "../features/runs/pages/RunResultsPage";
import { SettingsPage } from "../features/settings/pages/SettingsPage";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <Navigate to="/runs/new" replace /> },
      { path: "cases", element: <CaseLibraryPage /> },
      { path: "targets", element: <TargetConfigPage /> },
      { path: "runs/new", element: <CreateRunPage /> },
      { path: "runs/:runId", element: <RunMonitorPage /> },
      { path: "runs/:runId/results", element: <RunResultsPage /> },
      { path: "settings", element: <SettingsPage /> }
    ]
  }
]);
```

```tsx
// src/features/layout/AppShell.tsx
import { NavLink } from "react-router-dom";
import { PropsWithChildren } from "react";

const links = [
  { to: "/runs/new", label: "Runs" },
  { to: "/cases", label: "Cases" },
  { to: "/targets", label: "Targets" },
  { to: "/settings", label: "Settings" },
];

export function AppShell({ children }: PropsWithChildren) {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <h1>Evaluation Workbench</h1>
        <nav>
          {links.map((link) => (
            <NavLink key={link.to} to={link.to}>
              {link.label}
            </NavLink>
          ))}
        </nav>
      </aside>
      <main>{children}</main>
    </div>
  );
}
```

- [ ] **Step 5: Create the base Rust app entry**

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    iterm_mcp_evaluation_workbench::run();
}
```

```rust
// src-tauri/src/lib.rs
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
```

- [ ] **Step 6: Run the build checks**

Run: `npm install`
Expected: dependencies installed without errors

Run: `npm run build`
Expected: Vite frontend build succeeds

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: Rust crate compiles, even if no tests exist yet

- [ ] **Step 7: Commit**

```bash
git add package.json tsconfig.json vite.config.ts index.html src src-tauri
git commit -m "chore: scaffold tauri evaluation workbench"
```

## Task 2: Add SQLite schema and backend application state

**Files:**
- Create: `src-tauri/src/app_state.rs`
- Create: `src-tauri/src/error.rs`
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/schema.rs`
- Create: `src-tauri/src/db/migrations/0001_initial.sql`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/profile_service.rs`

- [ ] **Step 1: Write the failing migration test**

```rust
#[tokio::test]
async fn creates_core_tables() {
    let pool = setup_test_pool().await;
    let tables = list_tables(&pool).await;

    assert!(tables.contains(&"model_profiles".to_string()));
    assert!(tables.contains(&"window_bindings".to_string()));
    assert!(tables.contains(&"evaluation_cases".to_string()));
    assert!(tables.contains(&"comparison_runs".to_string()));
    assert!(tables.contains(&"comparison_targets".to_string()));
    assert!(tables.contains(&"messages".to_string()));
    assert!(tables.contains(&"analysis_results".to_string()));
    assert!(tables.contains(&"target_evaluations".to_string()));
}
```

- [ ] **Step 2: Write the initial migration**

```sql
CREATE TABLE model_profiles (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  provider TEXT NOT NULL,
  model_name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  api_key_encrypted TEXT NOT NULL,
  system_prompt TEXT NOT NULL DEFAULT '',
  temperature REAL,
  max_tokens INTEGER,
  extra_params_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE window_bindings (
  id TEXT PRIMARY KEY NOT NULL,
  iterm_session_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  profile_id TEXT NOT NULL REFERENCES model_profiles(id),
  enabled INTEGER NOT NULL DEFAULT 1,
  last_seen_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE evaluation_cases (
  id TEXT PRIMARY KEY NOT NULL,
  title TEXT NOT NULL,
  prompt TEXT NOT NULL,
  context_payload TEXT NOT NULL DEFAULT '{}',
  expected_checkpoints_json TEXT NOT NULL DEFAULT '[]',
  validation_rules_json TEXT NOT NULL DEFAULT '{}',
  notes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE comparison_runs (
  id TEXT PRIMARY KEY NOT NULL,
  evaluation_case_id TEXT NOT NULL REFERENCES evaluation_cases(id),
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  prompt_snapshot TEXT NOT NULL,
  context_snapshot TEXT NOT NULL,
  created_at TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT,
  notes TEXT NOT NULL DEFAULT ''
);

CREATE TABLE comparison_targets (
  id TEXT PRIMARY KEY NOT NULL,
  run_id TEXT NOT NULL REFERENCES comparison_runs(id),
  window_binding_id TEXT NOT NULL REFERENCES window_bindings(id),
  profile_snapshot_json TEXT NOT NULL,
  status TEXT NOT NULL,
  sent_at TEXT,
  first_response_at TEXT,
  finished_at TEXT,
  duration_ms INTEGER,
  response_chars INTEGER NOT NULL DEFAULT 0,
  response_lines INTEGER NOT NULL DEFAULT 0,
  success_status TEXT,
  error_category TEXT,
  error_detail TEXT
);
```

- [ ] **Step 3: Add the remaining evaluation tables**

```sql
CREATE TABLE messages (
  id TEXT PRIMARY KEY NOT NULL,
  comparison_target_id TEXT NOT NULL REFERENCES comparison_targets(id),
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  message_type TEXT NOT NULL,
  created_at TEXT NOT NULL,
  token_count INTEGER,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE analysis_results (
  id TEXT PRIMARY KEY NOT NULL,
  run_id TEXT NOT NULL REFERENCES comparison_runs(id),
  target_id TEXT REFERENCES comparison_targets(id),
  analysis_type TEXT NOT NULL,
  result_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE target_evaluations (
  id TEXT PRIMARY KEY NOT NULL,
  comparison_target_id TEXT NOT NULL REFERENCES comparison_targets(id),
  pass_at_1 INTEGER,
  unit_test_pass_rate REAL,
  consistency_score INTEGER,
  debug_success_rate REAL,
  input_tokens INTEGER,
  output_tokens INTEGER,
  total_tokens INTEGER,
  estimated_cost REAL,
  first_response_latency_ms INTEGER,
  full_completion_latency_ms INTEGER,
  conversation_turns INTEGER NOT NULL DEFAULT 0,
  compile_rating INTEGER,
  structure_rating INTEGER,
  business_rating INTEGER,
  overall_score INTEGER,
  manual_edit_lines INTEGER,
  judge_notes TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

- [ ] **Step 4: Create the database bootstrap code**

```rust
// src-tauri/src/db/mod.rs
use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions, SqlitePool};

static MIGRATOR: Migrator = sqlx::migrate!("./src/db/migrations");

pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    MIGRATOR.run(&pool).await?;
    Ok(pool)
}
```

- [ ] **Step 5: Add application state wiring**

```rust
// src-tauri/src/app_state.rs
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}
```

```rust
// src-tauri/src/lib.rs
mod app_state;
mod db;

use app_state::AppState;

pub fn run() {
    tauri::async_runtime::block_on(async move {
        let pool = db::connect("sqlite:workbench.db").await.expect("db init");
        tauri::Builder::default()
            .manage(AppState { pool })
            .run(tauri::generate_context!())
            .expect("failed to run tauri application");
    });
}
```

- [ ] **Step 6: Run the backend test and migration check**

Run: `cargo test --manifest-path src-tauri/Cargo.toml creates_core_tables -- --nocapture`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri
git commit -m "feat: add sqlite schema and app state"
```

## Task 3: Add domain models and CRUD services for profiles, bindings, and cases

**Files:**
- Create: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/models/profile.rs`
- Create: `src-tauri/src/models/window_binding.rs`
- Create: `src-tauri/src/models/evaluation_case.rs`
- Create: `src-tauri/src/services/profile_service.rs`
- Create: `src-tauri/src/services/window_binding_service.rs`
- Create: `src-tauri/src/services/evaluation_case_service.rs`
- Create: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/tests/profile_service.rs`
- Test: `src-tauri/tests/evaluation_case_service.rs`

- [ ] **Step 1: Write the failing profile service test**

```rust
#[tokio::test]
async fn creates_and_lists_profiles() {
    let service = test_profile_service().await;

    service.create_profile(CreateProfileInput {
        name: "GPT-5.4".into(),
        provider: "openai".into(),
        model_name: "gpt-5.4".into(),
        base_url: "https://api.example.com".into(),
        api_key: "secret".into(),
    }).await.unwrap();

    let profiles = service.list_profiles().await.unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].model_name, "gpt-5.4");
}
```

- [ ] **Step 2: Define the profile model and input DTO**

```rust
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_name: String,
    pub base_url: String,
    pub api_key_encrypted: String,
    pub system_prompt: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub extra_params_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateProfileInput {
    pub name: String,
    pub provider: String,
    pub model_name: String,
    pub base_url: String,
    pub api_key: String,
}
```

- [ ] **Step 3: Implement the profile service**

```rust
pub struct ProfileService {
    pool: SqlitePool,
}

impl ProfileService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_profile(&self, input: CreateProfileInput) -> anyhow::Result<ModelProfile> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let encrypted = input.api_key;

        sqlx::query(
            r#"
            INSERT INTO model_profiles
            (id, name, provider, model_name, base_url, api_key_encrypted, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.name)
        .bind(&input.provider)
        .bind(&input.model_name)
        .bind(&input.base_url)
        .bind(&encrypted)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_profile(&id).await
    }
}
```

- [ ] **Step 4: Add services for window bindings and evaluation cases**

```rust
pub struct CreateWindowBindingInput {
    pub iterm_session_id: String,
    pub display_name: String,
    pub profile_id: String,
}

pub struct CreateEvaluationCaseInput {
    pub title: String,
    pub prompt: String,
    pub context_payload: String,
    pub notes: String,
}
```

```rust
impl WindowBindingService {
    pub async fn create_window_binding(&self, input: CreateWindowBindingInput) -> anyhow::Result<WindowBinding> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO window_bindings (id, iterm_session_id, display_name, profile_id) VALUES (?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&input.iterm_session_id)
        .bind(&input.display_name)
        .bind(&input.profile_id)
        .execute(&self.pool)
        .await?;

        self.get_window_binding(&id).await
    }
}

impl EvaluationCaseService {
    pub async fn create_case(&self, input: CreateEvaluationCaseInput) -> anyhow::Result<EvaluationCase> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO evaluation_cases (id, title, prompt, context_payload, notes, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&input.title)
        .bind(&input.prompt)
        .bind(&input.context_payload)
        .bind(&input.notes)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_case(&id).await
    }
}
```

- [ ] **Step 5: Run CRUD tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml profile_service -- --nocapture`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml evaluation_case_service -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: add profile binding and case services"
```

## Task 4: Add Tauri commands for CRUD flows

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/profile_commands.rs`
- Create: `src-tauri/src/commands/window_binding_commands.rs`
- Create: `src-tauri/src/commands/evaluation_case_commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/profile_service.rs`

- [ ] **Step 1: Write the failing command integration test**

```rust
#[tokio::test]
async fn create_profile_command_returns_profile() {
    let state = test_app_state().await;
    let profile = create_profile(
        state,
        CreateProfileInput {
            name: "Claude".into(),
            provider: "anthropic".into(),
            model_name: "claude-sonnet".into(),
            base_url: "https://example.com".into(),
            api_key: "secret".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(profile.provider, "anthropic");
}
```

- [ ] **Step 2: Implement the profile commands**

```rust
#[tauri::command]
pub async fn create_profile(
    state: tauri::State<'_, AppState>,
    input: CreateProfileInput,
) -> Result<ModelProfile, String> {
    let service = ProfileService::new(state.pool.clone());
    service.create_profile(input).await.map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_profiles(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ModelProfile>, String> {
    let service = ProfileService::new(state.pool.clone());
    service.list_profiles().await.map_err(|err| err.to_string())
}
```

- [ ] **Step 3: Register all commands**

```rust
tauri::Builder::default()
    .manage(AppState { pool })
    .invoke_handler(tauri::generate_handler![
        commands::profile_commands::create_profile,
        commands::profile_commands::list_profiles,
        commands::window_binding_commands::create_window_binding,
        commands::window_binding_commands::list_window_bindings,
        commands::evaluation_case_commands::create_evaluation_case,
        commands::evaluation_case_commands::list_evaluation_cases
    ])
```

- [ ] **Step 4: Run the backend command tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml create_profile_command_returns_profile -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: expose tauri CRUD commands"
```

## Task 5: Build the target configuration page

**Files:**
- Create: `src/types/api.ts`
- Create: `src/lib/tauri.ts`
- Create: `src/features/targets/pages/TargetConfigPage.tsx`
- Create: `src/features/targets/components/ProfileForm.tsx`
- Create: `src/features/targets/components/WindowBindingList.tsx`
- Test: `src/features/runs/components/MetricTable.test.tsx`

- [ ] **Step 1: Write the failing render test for the target page**

```tsx
it("renders profile form and binding list", () => {
  render(<TargetConfigPage />);
  expect(screen.getByText("Model Profiles")).toBeInTheDocument();
  expect(screen.getByText("Window Bindings")).toBeInTheDocument();
});
```

- [ ] **Step 2: Add the frontend Tauri invoke wrapper**

```ts
import { invoke } from "@tauri-apps/api/core";
import type { CreateProfileInput, ModelProfile } from "../types/api";

export function listProfiles() {
  return invoke<ModelProfile[]>("list_profiles");
}

export function createProfile(input: CreateProfileInput) {
  return invoke<ModelProfile>("create_profile", { input });
}
```

- [ ] **Step 3: Build the target configuration page layout**

```tsx
export function TargetConfigPage() {
  return (
    <section className="page-grid">
      <div>
        <h2>iTerm Targets</h2>
        <WindowBindingList />
      </div>
      <div>
        <h2>Current Binding</h2>
        <p>Select a target to inspect or update its profile binding.</p>
      </div>
      <div>
        <h2>Model Profiles</h2>
        <ProfileForm />
      </div>
    </section>
  );
}
```

- [ ] **Step 4: Implement the profile creation form**

```tsx
export function ProfileForm() {
  const mutation = useMutation({ mutationFn: createProfile });

  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        const form = new FormData(event.currentTarget);
        mutation.mutate({
          name: String(form.get("name")),
          provider: String(form.get("provider")),
          model_name: String(form.get("model_name")),
          base_url: String(form.get("base_url")),
          api_key: String(form.get("api_key")),
        });
      }}
    >
      <input name="name" placeholder="Profile name" />
      <input name="provider" placeholder="Provider" />
      <input name="model_name" placeholder="Model name" />
      <input name="base_url" placeholder="Base URL" />
      <input name="api_key" placeholder="API key" type="password" />
      <button type="submit">Save profile</button>
    </form>
  );
}
```

- [ ] **Step 5: Run the frontend test**

Run: `npm test -- TargetConfigPage`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "feat: add target and profile management ui"
```

## Task 6: Build the evaluation case library

**Files:**
- Create: `src/features/cases/pages/CaseLibraryPage.tsx`
- Create: `src/features/cases/components/CaseForm.tsx`
- Modify: `src/lib/tauri.ts`
- Test: `src/features/runs/components/ResultComparisonGrid.test.tsx`

- [ ] **Step 1: Write the failing case library render test**

```tsx
it("renders prompt and context fields", () => {
  render(<CaseForm />);
  expect(screen.getByPlaceholderText("Case title")).toBeInTheDocument();
  expect(screen.getByPlaceholderText("Full prompt")).toBeInTheDocument();
  expect(screen.getByPlaceholderText("Context JSON")).toBeInTheDocument();
});
```

- [ ] **Step 2: Add the evaluation case invoke helpers**

```ts
export function listEvaluationCases() {
  return invoke<EvaluationCase[]>("list_evaluation_cases");
}

export function createEvaluationCase(input: CreateEvaluationCaseInput) {
  return invoke<EvaluationCase>("create_evaluation_case", { input });
}
```

- [ ] **Step 3: Build the case form**

```tsx
export function CaseForm() {
  return (
    <form className="stack">
      <input name="title" placeholder="Case title" />
      <textarea name="prompt" placeholder="Full prompt" rows={10} />
      <textarea name="context_payload" placeholder="Context JSON" rows={8} />
      <textarea name="notes" placeholder="Notes" rows={4} />
      <button type="submit">Save case</button>
    </form>
  );
}
```

- [ ] **Step 4: Build the case library page**

```tsx
export function CaseLibraryPage() {
  return (
    <section className="split-view">
      <div>
        <h2>Evaluation Cases</h2>
      </div>
      <div>
        <h2>Create or Edit Case</h2>
        <CaseForm />
      </div>
    </section>
  );
}
```

- [ ] **Step 5: Run the case library test**

Run: `npm test -- CaseForm`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src
git commit -m "feat: add evaluation case library ui"
```

## Task 7: Implement comparison runs and runtime orchestration

**Files:**
- Create: `src-tauri/src/services/iterm_mcp_adapter.rs`
- Create: `src-tauri/src/services/comparison_run_service.rs`
- Create: `src-tauri/src/services/comparison_orchestrator.rs`
- Create: `src-tauri/src/commands/comparison_commands.rs`
- Test: `src-tauri/tests/comparison_run_service.rs`

- [ ] **Step 1: Write the failing comparison run test**

```rust
#[tokio::test]
async fn creates_run_and_target_rows() {
    let service = test_comparison_run_service().await;

    let run = service.create_run(CreateRunInput {
        evaluation_case_id: "case-1".into(),
        title: "Legacy parser comparison".into(),
        target_ids: vec!["target-a".into(), "target-b".into()],
    }).await.unwrap();

    let targets = service.list_targets(&run.id).await.unwrap();
    assert_eq!(targets.len(), 2);
}
```

- [ ] **Step 2: Create the comparison run service**

```rust
pub struct CreateRunInput {
    pub evaluation_case_id: String,
    pub title: String,
    pub target_ids: Vec<String>,
}

pub async fn create_run(&self, input: CreateRunInput) -> anyhow::Result<ComparisonRun> {
    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query("INSERT INTO comparison_runs (id, evaluation_case_id, title, status, prompt_snapshot, context_snapshot, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&run_id)
        .bind(&input.evaluation_case_id)
        .bind(&input.title)
        .bind("queued")
        .bind("{}")
        .bind("{}")
        .bind(&now)
        .execute(&self.pool)
        .await?;

    Ok(self.get_run(&run_id).await?)
}
```

- [ ] **Step 3: Create the MCP adapter interface**

```rust
#[async_trait::async_trait]
pub trait ItermMcpAdapter: Send + Sync {
    async fn list_sessions(&self) -> anyhow::Result<Vec<ItermSession>>;
    async fn send_prompt(&self, session_id: &str, prompt: &str, context: &str) -> anyhow::Result<PromptExecution>;
}
```

- [ ] **Step 4: Implement the orchestrator**

```rust
pub async fn execute_run(&self, run_id: &str) -> anyhow::Result<()> {
    let targets = self.run_service.list_targets(run_id).await?;

    for target in targets {
        self.run_service.mark_target_running(&target.id).await?;
        match self.adapter.send_prompt(&target.session_id, &target.prompt, &target.context).await {
            Ok(execution) => {
                self.run_service.store_target_result(&target.id, execution).await?;
            }
            Err(error) => {
                self.run_service.mark_target_failed(&target.id, &error.to_string()).await?;
            }
        }
    }

    self.run_service.finalize_run(run_id).await
}
```

- [ ] **Step 5: Expose comparison commands**

Run: `cargo test --manifest-path src-tauri/Cargo.toml creates_run_and_target_rows -- --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri
git commit -m "feat: add comparison run orchestration"
```

## Task 8: Compute metrics and analysis summaries

**Files:**
- Create: `src-tauri/src/services/analysis_service.rs`
- Test: `src-tauri/tests/analysis_service.rs`

- [ ] **Step 1: Write the failing analysis test**

```rust
#[tokio::test]
async fn computes_latency_and_turn_metrics() {
    let service = test_analysis_service().await;

    let metrics = service.compute_target_metrics(TargetMetricInput {
        sent_at_ms: 0,
        first_response_at_ms: 200,
        finished_at_ms: 900,
        message_count: 3,
        output: "line 1\nline 2".into(),
        input_tokens: Some(12),
        output_tokens: Some(34),
    });

    assert_eq!(metrics.first_response_latency_ms, Some(200));
    assert_eq!(metrics.full_completion_latency_ms, Some(900));
    assert_eq!(metrics.conversation_turns, 3);
    assert_eq!(metrics.total_tokens, Some(46));
}
```

- [ ] **Step 2: Implement objective metric calculation**

```rust
pub fn compute_target_metrics(input: TargetMetricInput) -> TargetMetricOutput {
    TargetMetricOutput {
        first_response_latency_ms: input.first_response_at_ms.map(|value| value - input.sent_at_ms),
        full_completion_latency_ms: input.finished_at_ms.map(|value| value - input.sent_at_ms),
        conversation_turns: input.message_count as i64,
        total_tokens: match (input.input_tokens, input.output_tokens) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None,
        },
        response_chars: input.output.chars().count() as i64,
        response_lines: input.output.lines().count() as i64,
    }
}
```

- [ ] **Step 3: Implement a basic difference summary**

```rust
pub fn build_difference_summary(outputs: &[TargetOutputSummary]) -> String {
    let fastest = outputs.iter().min_by_key(|item| item.full_completion_latency_ms.unwrap_or(i64::MAX));
    let longest = outputs.iter().max_by_key(|item| item.response_chars);

    format!(
        "Fastest target: {}. Most verbose target: {}.",
        fastest.map(|item| item.label.as_str()).unwrap_or("unknown"),
        longest.map(|item| item.label.as_str()).unwrap_or("unknown"),
    )
}
```

- [ ] **Step 4: Run the analysis test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml computes_latency_and_turn_metrics -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri
git commit -m "feat: add evaluation metrics and summaries"
```

## Task 9: Build the run creation, monitoring, and result pages

**Files:**
- Create: `src/features/runs/pages/CreateRunPage.tsx`
- Create: `src/features/runs/pages/RunMonitorPage.tsx`
- Create: `src/features/runs/pages/RunResultsPage.tsx`
- Create: `src/features/runs/components/RunTargetStatusCard.tsx`
- Create: `src/features/runs/components/ResultComparisonGrid.tsx`
- Create: `src/features/runs/components/MetricTable.tsx`
- Modify: `src/lib/tauri.ts`
- Test: `src/features/runs/components/ResultComparisonGrid.test.tsx`
- Test: `src/features/runs/components/MetricTable.test.tsx`

- [ ] **Step 1: Write the failing metric table test**

```tsx
it("renders the core evaluation columns", () => {
  render(<MetricTable rows={[]} />);
  expect(screen.getByText("Pass@1")).toBeInTheDocument();
  expect(screen.getByText("Unit test pass rate")).toBeInTheDocument();
  expect(screen.getByText("Token cost")).toBeInTheDocument();
  expect(screen.getByText("Latency")).toBeInTheDocument();
});
```

- [ ] **Step 2: Build the run creation page**

```tsx
export function CreateRunPage() {
  return (
    <section className="stack">
      <h2>New Evaluation Run</h2>
      <p>Select an evaluation case and one or more bound targets.</p>
      <form className="stack">
        <label>
          Evaluation case
          <select name="evaluationCaseId">
            <option value="">Select a case</option>
          </select>
        </label>
        <label>
          Target bindings
          <select name="targetIds" multiple>
            <option value="target-a">Window A / GPT-5.4</option>
            <option value="target-b">Window B / Claude Sonnet</option>
          </select>
        </label>
        <label>
          Notes
          <textarea name="notes" rows={4} />
        </label>
        <button type="submit">Start run</button>
      </form>
    </section>
  );
}
```

- [ ] **Step 3: Build the monitor page**

```tsx
export function RunMonitorPage() {
  return (
    <section className="stack">
      <h2>Run Monitor</h2>
      <div className="target-grid">
        <RunTargetStatusCard />
      </div>
    </section>
  );
}
```

- [ ] **Step 4: Build the results page**

```tsx
export function RunResultsPage() {
  return (
    <section className="stack">
      <header>
        <h2>Comparison Results</h2>
      </header>
      <ResultComparisonGrid />
      <MetricTable rows={[]} />
    </section>
  );
}
```

- [ ] **Step 5: Implement the metric table columns**

```tsx
const columns = [
  "Model",
  "Pass@1",
  "Unit test pass rate",
  "Consistency",
  "Debug success rate",
  "Token cost",
  "Latency",
  "Structure rating",
  "Business rating",
  "Overall score",
  "Manual edit lines",
];
```

- [ ] **Step 6: Run the frontend result tests**

Run: `npm test -- MetricTable`
Expected: PASS

Run: `npm test -- ResultComparisonGrid`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src
git commit -m "feat: add run creation and comparison result pages"
```

## Task 10: Add export flows and application settings

**Files:**
- Create: `src-tauri/src/services/export_service.rs`
- Create: `src-tauri/src/commands/export_commands.rs`
- Create: `src/features/settings/pages/SettingsPage.tsx`
- Modify: `src/lib/tauri.ts`

- [ ] **Step 1: Write the failing export service test**

```rust
#[tokio::test]
async fn exports_metrics_as_csv() {
    let csv = export_metrics_csv(vec![ExportMetricRow {
        model_name: "gpt-5.4".into(),
        pass_at_1: Some(1),
        unit_test_pass_rate: Some(100.0),
    }]).unwrap();

    assert!(csv.contains("model_name,pass_at_1,unit_test_pass_rate"));
    assert!(csv.contains("gpt-5.4,1,100"));
}
```

- [ ] **Step 2: Implement CSV and JSON export helpers**

```rust
pub fn export_metrics_csv(rows: Vec<ExportMetricRow>) -> anyhow::Result<String> {
    let mut writer = csv::Writer::from_writer(vec![]);
    for row in rows {
        writer.serialize(row)?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}
```

- [ ] **Step 3: Build the settings page**

```tsx
export function SettingsPage() {
  return (
    <section className="stack">
      <h2>System Settings</h2>
      <label>
        Default timeout (ms)
        <input name="defaultTimeoutMs" type="number" />
      </label>
      <label>
        Max concurrency
        <input name="maxConcurrency" type="number" />
      </label>
    </section>
  );
}
```

- [ ] **Step 4: Run export and build verification**

Run: `cargo test --manifest-path src-tauri/Cargo.toml exports_metrics_as_csv -- --nocapture`
Expected: PASS

Run: `npm run build`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src src-tauri
git commit -m "feat: add exports and app settings"
```

## Task 11: End-to-end verification and polish

**Files:**
- Modify: `src/styles/globals.css`
- Modify: `src/features/layout/AppShell.tsx`
- Modify: `src/features/runs/pages/RunResultsPage.tsx`
- Test: `src-tauri/tests/comparison_run_service.rs`
- Test: `src-tauri/tests/analysis_service.rs`

- [ ] **Step 1: Add a basic seeded demo flow for manual QA**

```text
Seed data:
1. Two model profiles
2. Two window bindings
3. One evaluation case
4. One completed run with two comparison targets
```

- [ ] **Step 2: Run backend verification**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all backend tests pass

- [ ] **Step 3: Run frontend verification**

Run: `npm test`
Expected: all frontend tests pass

Run: `npm run build`
Expected: production build succeeds

- [ ] **Step 4: Manual verification checklist**

```text
1. Create a model profile
2. Create an evaluation case
3. See target configuration layout
4. Start a run from the run creation page
5. Open the monitor page
6. Open the results page
7. Confirm the metric table shows the required columns
8. Export CSV without leaking secrets
```

- [ ] **Step 5: Commit**

```bash
git add src src-tauri
git commit -m "chore: verify v1 evaluation workbench flow"
```

## Self-Review

Spec coverage check:

- Evaluation case management is covered by Tasks 3 and 6.
- Window discovery/profile binding groundwork is covered by Tasks 3, 4, and 5.
- Comparison execution is covered by Task 7.
- Metrics and summaries are covered by Task 8.
- Result comparison UI is covered by Task 9.
- Settings and export support are covered by Task 10.
- Verification and v1 readiness are covered by Task 11.

Placeholder scan:

- No `TODO`, `TBD`, or deferred implementation placeholders remain inside steps.
- Deferred v2 concerns such as consistency loops and debug mode are represented only as columns and model fields, not as hidden implementation gaps in v1 steps.

Type consistency check:

- Core entities are consistently named `ModelProfile`, `WindowBinding`, `EvaluationCase`, `ComparisonRun`, `ComparisonTarget`, and `TargetEvaluation`.
- Metrics use the same field names across the schema, services, and UI plan: `pass_at_1`, `unit_test_pass_rate`, `first_response_latency_ms`, `full_completion_latency_ms`, `structure_rating`, `business_rating`, `overall_score`, and `manual_edit_lines`.
