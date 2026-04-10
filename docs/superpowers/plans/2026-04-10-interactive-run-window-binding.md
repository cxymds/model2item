# Interactive Run Window Binding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bind each iTerm window to a model profile immediately, then keep a single interactive Claude session alive per comparison target so later prompts can be broadcast into the same run context.

**Architecture:** Reuse the existing comparison target lifecycle and treat `running` targets as active interactive sessions. Extend the iTerm adapter with text-send and screen-read operations, add a binding sync service that exports model env vars plus a visible status message into the selected window, and change the orchestrator so starting a run launches an interactive `claude` process per target instead of a one-shot command.

**Tech Stack:** Rust, Tauri, SQLx, Python iTerm2 bridge, Vitest/React Query frontend hooks, Tokio async tests

---

### Task 1: Add failing adapter/orchestrator tests for interactive sessions

**Files:**
- Modify: `src-tauri/tests/comparison_orchestrator.rs`
- Test: `src-tauri/tests/comparison_orchestrator.rs`

- [ ] **Step 1: Write failing tests for interactive start and follow-up broadcast**

```rust
#[tokio::test]
async fn starts_interactive_sessions_and_broadcasts_follow_up_input() -> Result<(), Box<dyn std::error::Error>> {
    // fake adapter captures sent texts per session
    // start run should send a claude launch command and the initial prompt
    // broadcast should send the follow-up prompt into the same session ids
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test comparison_orchestrator starts_interactive_sessions_and_broadcasts_follow_up_input`
Expected: FAIL because the orchestrator has no broadcast API and the adapter cannot maintain interactive sessions yet

- [ ] **Step 3: Write minimal production code to satisfy the test**

```rust
pub async fn broadcast_follow_up(&self, run_id: &str, prompt: &str) -> Result<(), AppError> {
    // query running targets
    // send prompt text to each existing session
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --test comparison_orchestrator starts_interactive_sessions_and_broadcasts_follow_up_input`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tests/comparison_orchestrator.rs src-tauri/src/services/comparison_orchestrator.rs src-tauri/src/services/iterm_mcp_adapter.rs src-tauri/scripts/iterm_mcp_adapter.py
git commit -m "feat: support interactive comparison runs"
```

### Task 2: Add failing tests for binding sync

**Files:**
- Modify: `src-tauri/tests/window_binding_service.rs`
- Create: `src-tauri/src/services/window_binding_sync_service.rs`
- Test: `src-tauri/tests/window_binding_service.rs`

- [ ] **Step 1: Write failing tests for applying a binding to an iTerm session**

```rust
#[tokio::test]
async fn applies_binding_to_window_session_and_writes_visible_notice() -> Result<(), Box<dyn std::error::Error>> {
    // create profile + binding
    // apply binding to session
    // assert exported env vars and a visible notice were sent
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test window_binding_service applies_binding_to_window_session_and_writes_visible_notice`
Expected: FAIL because no sync service exists yet

- [ ] **Step 3: Write minimal production code to satisfy the test**

```rust
pub async fn apply_binding(&self, binding_id: &str) -> Result<(), AppError> {
    // join binding + profile + secret
    // send export commands and an echo/printf notice into the session
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --test window_binding_service applies_binding_to_window_session_and_writes_visible_notice`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tests/window_binding_service.rs src-tauri/src/services/window_binding_sync_service.rs src-tauri/src/services/iterm_mcp_adapter.rs
git commit -m "feat: sync bound model profiles into target windows"
```

### Task 3: Wire Tauri commands and frontend invocation

**Files:**
- Modify: `src-tauri/src/commands/window_binding_commands.rs`
- Modify: `src-tauri/src/commands/comparison_commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/tauri.ts`
- Modify: `src/types/api.ts`
- Modify: `src/features/targets/pages/TargetConfigPage.tsx`

- [ ] **Step 1: Add failing frontend/command tests or command usage assertions**

```ts
// verify update/create binding triggers backend sync command
// verify running run can call broadcast command
```

- [ ] **Step 2: Run targeted tests to verify they fail**

Run: `npm test -- --runInBand`
Expected: FAIL at the new command wiring expectation

- [ ] **Step 3: Implement minimal command wiring**

```ts
export function sendComparisonRunMessage(runId: string, prompt: string) {
  return core.invoke<void>("send_comparison_run_message", { runId, prompt });
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib && cargo test --test comparison_orchestrator && cargo test --test window_binding_service`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/window_binding_commands.rs src-tauri/src/commands/comparison_commands.rs src-tauri/src/lib.rs src/lib/tauri.ts src/types/api.ts src/features/targets/pages/TargetConfigPage.tsx
git commit -m "feat: expose interactive run and binding sync commands"
```
