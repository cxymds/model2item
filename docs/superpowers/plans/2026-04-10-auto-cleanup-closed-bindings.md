# Auto Cleanup Closed Bindings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Automatically remove closed window bindings that are no longer referenced, while keeping referenced bindings for history.

**Architecture:** Centralize binding/session reconciliation inside `WindowBindingService`, then reuse that method from both the manual refresh command and a Rust background polling task started during app setup. Keep the UI change small by only updating the explanatory text.

**Tech Stack:** Tauri 2, Rust, SQLx/SQLite, Tokio, React, TypeScript, Vitest

---

### Task 1: Add Sync-And-Cleanup Service Logic

**Files:**
- Modify: `src-tauri/src/services/window_binding_service.rs`
- Test: `src-tauri/tests/window_binding_service.rs`

- [ ] Add failing tests for deleting unreferenced offline bindings and keeping referenced offline bindings.
- [ ] Run the focused Rust test command and verify the new assertions fail.
- [ ] Implement one service method that updates online timestamps and deletes only unreferenced missing-session bindings.
- [ ] Re-run the focused Rust tests until they pass.

### Task 2: Reuse Sync Logic From Commands And Background Polling

**Files:**
- Modify: `src-tauri/src/commands/window_binding_commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] Repoint the manual refresh command to the new sync method.
- [ ] Start a background polling task during app startup that periodically lists iTerm sessions and syncs bindings.
- [ ] Ensure missing adapter dependencies only skip a polling cycle instead of crashing the app.

### Task 3: Update UI Messaging And Verification

**Files:**
- Modify: `src/features/targets/components/WindowBindingList.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.test.tsx`

- [ ] Add a failing front-end test for the new auto-cleanup hint text.
- [ ] Update the page hint to explain automatic cleanup of closed, unreferenced bindings.
- [ ] Run targeted front-end tests, then full verification (`cargo test`, `npm run test`, `npm run lint`, `npm run build`).
