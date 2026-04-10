# Binding And Case Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add inline edit/delete management for saved window bindings and evaluation cases, while blocking deletion when records are already referenced by comparison runs.

**Architecture:** Extend existing Tauri CRUD services and commands with update/delete operations, enforce reference protection in Rust services, then wire the React list views to inline editing flows that reuse current form patterns. Keep all changes inside the current page structure so the UI remains lightweight and consistent with the workbench.

**Tech Stack:** Tauri 2, Rust, SQLx/SQLite, React, TypeScript, TanStack Query, Vitest

---

## File Map

- Modify: `src-tauri/src/models/window_binding.rs`
- Modify: `src-tauri/src/models/evaluation_case.rs`
- Modify: `src-tauri/src/services/window_binding_service.rs`
- Modify: `src-tauri/src/services/evaluation_case_service.rs`
- Modify: `src-tauri/src/commands/window_binding_commands.rs`
- Modify: `src-tauri/src/commands/evaluation_case_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tests/window_binding_service.rs`
- Modify: `src-tauri/tests/evaluation_case_service.rs`
- Modify: `src/types/api.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/targets/components/WindowBindingList.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.test.tsx`
- Modify: `src/features/cases/components/CaseForm.tsx`
- Modify: `src/features/cases/pages/CaseLibraryPage.tsx`
- Modify: `src/features/cases/components/CaseForm.test.tsx`

### Task 1: Add Backend Update/Delete Support For Window Bindings

**Files:**
- Modify: `src-tauri/src/models/window_binding.rs`
- Modify: `src-tauri/src/services/window_binding_service.rs`
- Modify: `src-tauri/src/commands/window_binding_commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/window_binding_service.rs`

- [ ] Step 1: Write failing tests for binding update and protected delete.
- [ ] Step 2: Run `cargo test --manifest-path src-tauri/Cargo.toml window_binding_service` and confirm the new assertions fail for missing methods/behavior.
- [ ] Step 3: Add `UpdateWindowBindingInput`, implement update/delete service methods, and reject delete when `comparison_targets.window_binding_id` exists.
- [ ] Step 4: Expose update/delete commands and register them in the Tauri handler.
- [ ] Step 5: Re-run `cargo test --manifest-path src-tauri/Cargo.toml window_binding_service` until it passes.

### Task 2: Add Backend Update/Delete Support For Evaluation Cases

**Files:**
- Modify: `src-tauri/src/models/evaluation_case.rs`
- Modify: `src-tauri/src/services/evaluation_case_service.rs`
- Modify: `src-tauri/src/commands/evaluation_case_commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/evaluation_case_service.rs`

- [ ] Step 1: Write failing tests for case update and protected delete.
- [ ] Step 2: Run `cargo test --manifest-path src-tauri/Cargo.toml evaluation_case_service` and confirm failures.
- [ ] Step 3: Add `UpdateEvaluationCaseInput`, implement update/delete service methods, keep JSON validation, and reject delete when `comparison_runs.evaluation_case_id` exists.
- [ ] Step 4: Expose update/delete commands and register them in the Tauri handler.
- [ ] Step 5: Re-run `cargo test --manifest-path src-tauri/Cargo.toml evaluation_case_service` until it passes.

### Task 3: Extend Frontend API Layer

**Files:**
- Modify: `src/types/api.ts`
- Modify: `src/lib/tauri.ts`

- [ ] Step 1: Add update input types for bindings and cases in `src/types/api.ts`.
- [ ] Step 2: Add Tauri invoke wrappers for update/delete binding and update/delete case in `src/lib/tauri.ts`.
- [ ] Step 3: Run `npm run build` once to catch type errors from the API surface changes.

### Task 4: Add Inline Binding Editing And Deletion UI

**Files:**
- Modify: `src/features/targets/components/WindowBindingList.tsx`
- Modify: `src/features/targets/pages/TargetConfigPage.tsx`
- Test: `src/features/targets/pages/TargetConfigPage.test.tsx`

- [ ] Step 1: Write a failing React test covering editing or deleting a binding from the target config page.
- [ ] Step 2: Run `npm run test -- TargetConfigPage` and verify the new test fails.
- [ ] Step 3: Add update/delete mutations in `TargetConfigPage`, invalidate `window-bindings`, and pass edit/delete handlers into `WindowBindingList`.
- [ ] Step 4: Implement inline edit state, save/cancel actions, delete confirmation, and error rendering in `WindowBindingList`.
- [ ] Step 5: Re-run `npm run test -- TargetConfigPage` until it passes.

### Task 5: Add Inline Case Editing And Deletion UI

**Files:**
- Modify: `src/features/cases/components/CaseForm.tsx`
- Modify: `src/features/cases/pages/CaseLibraryPage.tsx`
- Test: `src/features/cases/components/CaseForm.test.tsx`

- [ ] Step 1: Write failing React coverage for editing an existing case or surfacing protected delete errors.
- [ ] Step 2: Run `npm run test -- CaseForm CaseLibraryPage` and verify failure.
- [ ] Step 3: Refactor `CaseForm` to support reusable initial values and configurable submit labels without breaking create behavior.
- [ ] Step 4: Add case update/delete mutations and inline card editing flow in `CaseLibraryPage`.
- [ ] Step 5: Re-run `npm run test -- CaseForm CaseLibraryPage` until it passes.

### Task 6: Full Verification

**Files:**
- Verify only

- [ ] Step 1: Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Step 2: Run `npm run test`.
- [ ] Step 3: Run `npm run lint`.
- [ ] Step 4: Run `npm run build`.
- [ ] Step 5: Review the UI text and confirm delete-protection messages are Chinese and actionable.

## Self-Review

- Spec coverage checked: backend CRUD, reference protection, frontend inline edit/delete, and verification are all represented by tasks.
- Placeholder scan checked: no `TODO`/`TBD` markers remain.
- Type consistency checked: the plan consistently uses `UpdateWindowBindingInput` and `UpdateEvaluationCaseInput`.

Plan complete and saved to `docs/superpowers/plans/2026-04-10-binding-and-case-management.md`. I will proceed with Inline Execution in this session because you already asked me to continue directly.
