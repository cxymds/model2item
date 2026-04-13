# Custom Provider Model Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce first-class custom providers so windows resolve to explicit provider configurations instead of relying on legacy profile-plus-model routing.

**Architecture:** Add a new `custom_providers` persistence layer and a compatibility migration path that allows `window_bindings` to reference either legacy profiles or new providers while runtime resolution prefers providers. Update the Tauri commands, runtime services, and frontend target configuration UI to create, edit, bind, and execute against provider-first data.

**Tech Stack:** Rust, SQLx, SQLite migrations, Tauri commands, React, TypeScript, TanStack Query, Vitest

---

## File Structure

### New Files

- Create: `src-tauri/src/models/custom_provider.rs`
- Create: `src-tauri/src/services/custom_provider_service.rs`
- Create: `src-tauri/src/commands/custom_provider_commands.rs`
- Create: `src-tauri/src/db/migrations/0004_add_custom_providers.sql`
- Create: `src-tauri/tests/custom_provider_service.rs`
- Create: `src/features/targets/components/CustomProviderForm.tsx`

### Modified Files

- Modify: `src-tauri/src/models/window_binding.rs`
- Modify: `src-tauri/src/models/profile.rs`
- Modify: `src-tauri/src/services/window_binding_service.rs`
- Modify: `src-tauri/src/services/window_binding_sync_service.rs`
- Modify: `src-tauri/src/services/comparison_run_service.rs`
- Modify: `src-tauri/src/services/comparison_orchestrator.rs`
- Modify: `src-tauri/src/services/iterm_mcp_adapter.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/commands/window_binding_commands.rs`
- Modify: `src-tauri/src/commands/profile_commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauri.ts`
- Modify: `src/types/api.ts`
- Modify: `src/features/targets/pages/TargetConfigPage.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.test.tsx`
- Modify: `src-tauri/tests/window_binding_service.rs`
- Modify: `src-tauri/tests/comparison_orchestrator.rs`

### Existing Files To Read During Implementation

- Read: `docs/superpowers/specs/2026-04-13-custom-provider-model-selection-design.md`
- Read: `src-tauri/src/db/migrations/0002_add_execution_mode_to_model_profiles.sql`
- Read: `src-tauri/src/db/migrations/0003_add_enabled_to_model_profiles.sql`

### Task 1: Add the Custom Provider Data Model and Migration

**Files:**
- Create: `src-tauri/src/db/migrations/0004_add_custom_providers.sql`
- Create: `src-tauri/src/models/custom_provider.rs`
- Test: `src-tauri/tests/custom_provider_service.rs`

- [ ] **Step 1: Write the failing migration/service test**

```rust
#[tokio::test]
async fn lists_backfilled_custom_providers_from_existing_profiles(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;

    sqlx::query(
        r#"
        INSERT INTO model_profiles
          (id, name, provider, execution_mode, model_name, base_url, api_key_encrypted, system_prompt, temperature, max_tokens, extra_params_json, created_at, updated_at)
        VALUES
          ('profile-1', 'GLM via Claude CLI', 'glm', 'claude_cli', 'glm-5.1', 'https://glm.example.com/v1', 'secret://glm', '', NULL, NULL, '{}', '2026-04-13T00:00:00Z', '2026-04-13T00:00:00Z')
        "#
    )
    .execute(&pool)
    .await?;

    sqlx::migrate!("./src/db/migrations").run(&pool).await?;

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM custom_providers")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 1);

    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test custom_provider_service lists_backfilled_custom_providers_from_existing_profiles`
Expected: FAIL with `no such table: custom_providers` or missing model/service symbols

- [ ] **Step 3: Add the migration and provider model**

```sql
CREATE TABLE custom_providers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  provider_key TEXT NOT NULL,
  client_type TEXT NOT NULL,
  base_url TEXT NOT NULL DEFAULT '',
  api_key_encrypted TEXT NOT NULL,
  default_model TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  extra_params_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

ALTER TABLE window_bindings ADD COLUMN custom_provider_id TEXT;

INSERT INTO custom_providers (
  id, name, provider_key, client_type, base_url, api_key_encrypted, default_model, enabled, extra_params_json, created_at, updated_at
)
SELECT
  'provider-' || id,
  name,
  provider,
  execution_mode,
  base_url,
  api_key_encrypted,
  model_name,
  1,
  extra_params_json,
  created_at,
  updated_at
FROM model_profiles;

UPDATE window_bindings
SET custom_provider_id = 'provider-' || profile_id
WHERE custom_provider_id IS NULL;
```

```rust
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CustomProviderRecord {
    pub id: String,
    pub name: String,
    pub provider_key: String,
    pub client_type: String,
    pub base_url: String,
    pub api_key_encrypted: String,
    pub default_model: String,
    pub enabled: i64,
    pub extra_params_json: String,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test custom_provider_service lists_backfilled_custom_providers_from_existing_profiles`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/migrations/0004_add_custom_providers.sql src-tauri/src/models/custom_provider.rs src-tauri/tests/custom_provider_service.rs
git commit -m "feat: add custom provider storage"
```

### Task 2: Add Custom Provider CRUD Services and Commands

**Files:**
- Create: `src-tauri/src/services/custom_provider_service.rs`
- Create: `src-tauri/src/commands/custom_provider_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/models/custom_provider.rs`
- Test: `src-tauri/tests/custom_provider_service.rs`

- [ ] **Step 1: Write the failing CRUD tests**

```rust
#[tokio::test]
async fn creates_and_lists_custom_providers() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let service = CustomProviderService::with_secret_store(pool, secret_store);

    let created = service
        .create_custom_provider(CreateCustomProviderInput {
            name: "GLM via Claude CLI".to_string(),
            provider_key: "glm".to_string(),
            client_type: "claude_cli".to_string(),
            base_url: "https://glm.example.com/v1".to_string(),
            api_key: "glm-secret".to_string(),
            default_model: "glm-5.1".to_string(),
            extra_params_json: "{}".to_string(),
        })
        .await?;

    assert_eq!(created.provider_key, "glm");
    assert_eq!(created.client_type, "claude_cli");
    assert_eq!(service.list_custom_providers().await?.len(), 1);
    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test custom_provider_service creates_and_lists_custom_providers`
Expected: FAIL with missing service, command, or input types

- [ ] **Step 3: Implement CRUD service and command wiring**

```rust
pub async fn list_custom_providers(&self) -> Result<Vec<CustomProviderResponse>, AppError> {
    let rows = sqlx::query_as::<_, CustomProviderRecord>(
        "SELECT * FROM custom_providers WHERE enabled = 1 ORDER BY rowid DESC"
    )
    .fetch_all(&self.pool)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}
```

```rust
#[tauri::command]
pub async fn create_custom_provider(
    state: tauri::State<'_, AppState>,
    input: CreateCustomProviderInput,
) -> Result<CustomProviderResponse, String> {
    state
        .custom_provider_service
        .create_custom_provider(input)
        .await
        .map_err(|error| error.to_string())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test custom_provider_service`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/custom_provider_service.rs src-tauri/src/commands/custom_provider_commands.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/src/models/custom_provider.rs src-tauri/tests/custom_provider_service.rs
git commit -m "feat: add custom provider CRUD commands"
```

### Task 3: Switch Window Bindings to Resolve Providers First

**Files:**
- Modify: `src-tauri/src/models/window_binding.rs`
- Modify: `src-tauri/src/services/window_binding_service.rs`
- Modify: `src-tauri/src/services/window_binding_sync_service.rs`
- Modify: `src-tauri/src/commands/window_binding_commands.rs`
- Test: `src-tauri/tests/window_binding_service.rs`

- [ ] **Step 1: Write the failing window binding tests**

```rust
#[tokio::test]
async fn creates_binding_with_custom_provider_id() -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let service = WindowBindingService::new(pool.clone());

    sqlx::query(
        "INSERT INTO custom_providers (id, name, provider_key, client_type, base_url, api_key_encrypted, default_model, enabled, extra_params_json, created_at, updated_at) VALUES ('provider-1', 'GLM via Claude CLI', 'glm', 'claude_cli', 'https://glm.example.com/v1', 'secret://glm', 'glm-5.1', 1, '{}', '2026-04-13T00:00:00Z', '2026-04-13T00:00:00Z')"
    )
    .execute(&pool)
    .await?;

    let binding = service
        .create_window_binding(CreateWindowBindingInput {
            iterm_session_id: "session-1".to_string(),
            display_name: "Window 1".to_string(),
            profile_id: String::new(),
            custom_provider_id: Some("provider-1".to_string()),
        })
        .await?;

    assert_eq!(binding.custom_provider_id.as_deref(), Some("provider-1"));
    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test window_binding_service creates_binding_with_custom_provider_id`
Expected: FAIL with missing `custom_provider_id` fields or service logic

- [ ] **Step 3: Extend the binding model and sync queries**

```rust
pub struct WindowBindingRecord {
    pub id: String,
    pub iterm_session_id: String,
    pub display_name: String,
    pub profile_id: String,
    pub custom_provider_id: Option<String>,
    pub enabled: i64,
    pub last_seen_at: Option<String>,
    pub metadata_json: String,
}
```

```rust
INSERT INTO window_bindings (id, iterm_session_id, display_name, profile_id, custom_provider_id)
VALUES (?, ?, ?, ?, ?)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test window_binding_service`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models/window_binding.rs src-tauri/src/services/window_binding_service.rs src-tauri/src/services/window_binding_sync_service.rs src-tauri/src/commands/window_binding_commands.rs src-tauri/tests/window_binding_service.rs
git commit -m "feat: bind windows to custom providers"
```

### Task 4: Update Runtime Resolution and Comparison Snapshots

**Files:**
- Modify: `src-tauri/src/services/comparison_run_service.rs`
- Modify: `src-tauri/src/services/comparison_orchestrator.rs`
- Modify: `src-tauri/src/services/iterm_mcp_adapter.rs`
- Test: `src-tauri/tests/comparison_orchestrator.rs`
- Test: `src-tauri/tests/window_binding_service.rs`

- [ ] **Step 1: Write the failing runtime resolution tests**

```rust
#[tokio::test]
async fn execute_run_prefers_custom_provider_over_profile_fields(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    let secret_store = Arc::new(MemorySecretStore::default());
    let orchestrator = ComparisonOrchestrator::with_dependencies(
        pool.clone(),
        secret_store.clone(),
        FakeAdapter::default(),
    );

    // Setup omitted here in prose is not allowed in implementation.
    // The real test should insert a profile, a custom provider with different values,
    // a window binding pointing at the provider, then assert the executed request uses provider values.

    let requests = /* capture requests from adapter */;
    assert_eq!(requests[0].provider, "glm");
    assert_eq!(requests[0].model_name, "glm-5.1");
    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test comparison_orchestrator execute_run_prefers_custom_provider_over_profile_fields`
Expected: FAIL because runtime still resolves from legacy profile joins

- [ ] **Step 3: Update provider-first runtime queries**

```rust
SELECT
  wb.id AS window_binding_id,
  COALESCE(cp.provider_key, mp.provider) AS provider,
  COALESCE(cp.client_type, mp.execution_mode) AS execution_mode,
  COALESCE(cp.default_model, mp.model_name) AS model_name,
  COALESCE(cp.base_url, mp.base_url) AS base_url,
  COALESCE(cp.api_key_encrypted, mp.api_key_encrypted) AS api_key_locator,
  COALESCE(cp.extra_params_json, mp.extra_params_json) AS extra_params_json
FROM window_bindings wb
LEFT JOIN custom_providers cp ON cp.id = wb.custom_provider_id
LEFT JOIN model_profiles mp ON mp.id = wb.profile_id
```

```rust
let provider_env = provider_env_vars(
    &request.provider,
    &request.api_key,
    &request.base_url,
    &request.model_name,
);
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test comparison_orchestrator`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/comparison_run_service.rs src-tauri/src/services/comparison_orchestrator.rs src-tauri/src/services/iterm_mcp_adapter.rs src-tauri/tests/comparison_orchestrator.rs src-tauri/tests/window_binding_service.rs
git commit -m "feat: resolve executions from custom providers"
```

### Task 5: Replace Frontend Profile-First UI With Provider-First UI

**Files:**
- Create: `src/features/targets/components/CustomProviderForm.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.test.tsx`
- Modify: `src/lib/tauri.ts`
- Modify: `src/types/api.ts`

- [ ] **Step 1: Write the failing frontend test**

```tsx
it("creates a provider and binds a window to that provider", async () => {
  render(<TargetConfigPage />);

  fireEvent.change(screen.getByLabelText("名称"), {
    target: { value: "GLM via Claude CLI" },
  });
  fireEvent.change(screen.getByLabelText("上游标识"), {
    target: { value: "glm" },
  });
  fireEvent.change(screen.getByLabelText("客户端类型"), {
    target: { value: "claude_cli" },
  });
  fireEvent.change(screen.getByLabelText("默认模型"), {
    target: { value: "glm-5.1" },
  });

  fireEvent.click(screen.getByRole("button", { name: "保存 Provider" }));

  await waitFor(() => {
    expect(createCustomProvider).toHaveBeenCalledWith({
      name: "GLM via Claude CLI",
      provider_key: "glm",
      client_type: "claude_cli",
      default_model: "glm-5.1",
      base_url: "",
      api_key: "",
      extra_params_json: "{}",
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/features/targets/pages/TargetConfigPage.test.tsx`
Expected: FAIL with missing provider APIs and UI controls

- [ ] **Step 3: Implement provider-first API types and UI**

```ts
export interface CustomProviderResponse {
  id: string;
  name: string;
  provider_key: string;
  client_type: string;
  base_url: string;
  default_model: string;
  extra_params_json: string;
  created_at: string;
  updated_at: string;
}
```

```ts
export function listCustomProviders() {
  return core.invoke<CustomProviderResponse[]>("list_custom_providers");
}
```

```tsx
<label className="field">
  <span>上游标识</span>
  <input value={form.provider_key} onChange={handleProviderKeyChange} />
</label>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- src/features/targets/pages/TargetConfigPage.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/features/targets/components/CustomProviderForm.tsx src/features/targets/pages/TargetConfigPage.tsx src/features/targets/pages/TargetConfigPage.test.tsx src/lib/tauri.ts src/types/api.ts
git commit -m "feat: add provider-first target configuration UI"
```

### Task 6: De-Emphasize Legacy Profiles and Verify Compatibility

**Files:**
- Modify: `src-tauri/src/commands/profile_commands.rs`
- Modify: `src-tauri/src/models/profile.rs`
- Modify: `src/features/targets/pages/TargetConfigPage.tsx`
- Test: `src-tauri/tests/profile_service.rs`
- Test: `src-tauri/tests/window_binding_service.rs`
- Test: `src-tauri/tests/comparison_orchestrator.rs`

- [ ] **Step 1: Write the failing compatibility tests**

```rust
#[tokio::test]
async fn legacy_profile_binding_still_executes_when_custom_provider_is_null(
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = support::create_test_pool().await?;
    // Create legacy profile-only binding and assert execution still resolves.
    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test profile_service legacy_profile_binding_still_executes_when_custom_provider_is_null`
Expected: FAIL until compatibility path is wired

- [ ] **Step 3: Implement compatibility messaging and fallback**

```rust
let resolved_provider = if let Some(custom_provider_id) = &binding.custom_provider_id {
    self.load_custom_provider(custom_provider_id).await?
} else {
    self.load_legacy_profile(&binding.profile_id).await?
};
```

```tsx
<p className="muted">
  旧版模型配置仅用于兼容历史绑定。新建窗口请直接绑定 Provider。
</p>
```

- [ ] **Step 4: Run verification suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test custom_provider_service`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test window_binding_service`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test comparison_orchestrator`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test profile_service`
Expected: PASS

Run: `npm test -- src/features/targets/pages/TargetConfigPage.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/profile_commands.rs src-tauri/src/models/profile.rs src/features/targets/pages/TargetConfigPage.tsx src-tauri/tests/profile_service.rs src-tauri/tests/window_binding_service.rs src-tauri/tests/comparison_orchestrator.rs
git commit -m "feat: preserve legacy profile compatibility during provider migration"
```
