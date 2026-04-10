# iTerm2 MCP Multi-Window Evaluation Workbench Design

## Overview

This project is a local desktop evaluation workbench built with `Tauri + Rust + React + SQLite`.

The primary goal is not to provide a generic chat client. The primary goal is to evaluate model performance on old-code understanding tasks by sending the same prompt to multiple iTerm2 windows bound to different model configurations, collecting the full results, and presenting them in a structured comparison view.

The product should support:

- Sending the same evaluation case to multiple iTerm2 windows
- Binding different windows to different model profiles
- Saving full prompts, conversation turns, outputs, metadata, and errors
- Generating structured comparison results for human review
- Supporting objective metrics and human scoring
- Preserving history for replay and export

## Product Positioning

Version 1 is a local evaluation workbench focused on one core scenario:

`Old code understanding evaluation under identical prompt conditions`

The system is intended to answer questions such as:

- Which model best understands an existing legacy codebase?
- Which model is fastest under the same prompt and context?
- Which model produces the most structurally useful result?
- Which model passes validation most often on the first try?
- How do multiple models compare side by side for the same task?

This version explicitly does not optimize for:

- General-purpose free-form chat as the main experience
- Cloud sync
- Multi-user collaboration
- Complex historical trend dashboards
- Large-scale benchmark orchestration across remote infrastructure

## Technical Stack

### Desktop Application

- `Tauri`
- `Rust`

### Frontend

- `React`
- `TypeScript`

### Local Storage

- `SQLite`

### Security

- API keys must not be stored in plaintext in general logs or exports
- Sensitive credentials should be encrypted before persistence or stored through a system keychain strategy where feasible

## Core Design Principles

1. Local-first
   All evaluation data, prompts, outputs, scores, and settings live locally by default.

2. Same-input fairness
   A comparison run must ensure each target receives the same prompt and contextual payload snapshot.

3. Target isolation
   Each iTerm2 window binding acts as an independent execution target with its own model profile and runtime outcome.

4. Full traceability
   The system should preserve enough raw information to explain why a score or comparison result was produced.

5. Structured comparison over raw chat
   The main artifact is a comparison result, not a collection of unrelated chats.

## Primary Use Case

The main scenario is evaluating old-code understanding ability.

An evaluator creates or selects an evaluation case containing:

- Case title
- Full prompt
- Code background or context
- Optional expected checkpoints
- Optional validation rules

The evaluator then selects multiple iTerm2 targets. Each target is a window or session bound to a specific model profile. The system sends the same evaluation case to all selected targets, gathers the results, computes structured metrics, and presents the outputs in a unified comparison view.

## High-Level Architecture

The application is composed of the following layers:

1. Frontend presentation layer
   Built with `React + TypeScript`. Responsible for task creation, monitoring, result comparison, scoring input, and configuration screens.

2. Tauri command bridge
   The boundary between frontend actions and Rust application services.

3. Rust application services
   Encapsulate orchestration, persistence, MCP communication, evaluation, and analysis.

4. Local persistence layer
   SQLite database for durable storage of configuration, runs, messages, and metrics.

### Rust Service Modules

#### `iterm_mcp_adapter`

Responsibilities:

- Discover available iTerm2 windows, tabs, or sessions
- Communicate with iTerm2 MCP endpoints
- Send prompts and contextual payloads
- Read responses and command outputs
- Normalize transport errors and runtime status

#### `comparison_orchestrator`

Responsibilities:

- Start a comparison run
- Expand one evaluation case into multiple targets
- Dispatch identical prompt snapshots to each selected target
- Track lifecycle states for each target
- Handle timeout, cancellation, retry, and completion aggregation

#### `session_store`

Responsibilities:

- Manage SQLite schema and queries
- Save raw messages, target state changes, metrics, and snapshots
- Persist evaluation cases and settings

#### `analysis_engine`

Responsibilities:

- Generate structured analysis outputs
- Compute summary stats
- Produce difference summaries
- Extract keywords and notable points from responses

#### `evaluation_service`

Responsibilities:

- Compute structured evaluation metrics
- Separate auto-collected metrics from human-entered scores
- Support overall scoring and result ranking

#### `settings_manager`

Responsibilities:

- Manage global application settings
- Manage model profiles
- Manage window bindings
- Enforce secret handling rules

## Core Domain Model

### `ModelProfile`

A reusable model configuration template.

Fields should include:

- `id`
- `name`
- `provider`
- `model_name`
- `base_url`
- `api_key_encrypted`
- `system_prompt`
- `temperature`
- `max_tokens`
- `extra_params_json`
- `created_at`
- `updated_at`

### `WindowBinding`

A binding between an iTerm2 target and a model profile.

Fields should include:

- `id`
- `iterm_window_id` or `iterm_session_id`
- `display_name`
- `profile_id`
- `enabled`
- `last_seen_at`
- `metadata_json`

Purpose:

- Window A can use model/profile X
- Window B can use model/profile Y
- Window C can use model/profile Z

This allows different windows to use different API keys and base URLs cleanly.

### `EvaluationCase`

A reusable evaluation prompt definition for old-code understanding tasks.

Fields should include:

- `id`
- `title`
- `prompt`
- `context_payload`
- `expected_checkpoints_json`
- `validation_rules_json`
- `notes`
- `created_at`
- `updated_at`

### `ComparisonRun`

One actual evaluation execution instance.

Fields should include:

- `id`
- `evaluation_case_id`
- `title`
- `status`
- `prompt_snapshot`
- `context_snapshot`
- `created_at`
- `started_at`
- `finished_at`
- `notes`

### `ComparisonTarget`

One execution target inside a comparison run.

Fields should include:

- `id`
- `run_id`
- `window_binding_id`
- `profile_snapshot_json`
- `status`
- `sent_at`
- `first_response_at`
- `finished_at`
- `duration_ms`
- `response_chars`
- `response_lines`
- `success_status`
- `error_category`
- `error_detail`

The snapshot is required so historical runs remain reproducible even if the underlying profile changes later.

### `Message`

The raw conversation or output records associated with a target.

Fields should include:

- `id`
- `comparison_target_id`
- `role`
- `content`
- `message_type`
- `created_at`
- `token_count`
- `metadata_json`

### `AnalysisResult`

Semi-structured analysis outputs that help summarize and compare targets.

Fields should include:

- `id`
- `run_id`
- `target_id`
- `analysis_type`
- `result_json`
- `created_at`

Expected `analysis_type` values may include:

- `keyword_extraction`
- `difference_summary`
- `winner_summary`
- `response_summary`

### `TargetEvaluation`

Structured evaluation metrics for one target within one run.

Fields should include:

- `id`
- `comparison_target_id`
- `pass_at_1`
- `unit_test_pass_rate`
- `consistency_score`
- `debug_success_rate`
- `input_tokens`
- `output_tokens`
- `total_tokens`
- `estimated_cost`
- `first_response_latency_ms`
- `full_completion_latency_ms`
- `conversation_turns`
- `compile_rating`
- `structure_rating`
- `business_rating`
- `overall_score`
- `manual_edit_lines`
- `judge_notes`
- `created_at`
- `updated_at`

### `ManualJudgment`

Optional final human judgment for a run.

Fields should include:

- `id`
- `run_id`
- `winner_target_id`
- `comment`
- `created_at`

## Evaluation Metrics

Version 1 should distinguish between objective metrics and human scoring.

### Objective Metrics

- `Pass@1`
- `Unit test pass rate`
- `Token cost`
- `Latency`
- `Conversation turns`
- `Total duration`

### Human or Semi-Automated Metrics

- `Structure rating`
- `Business rating`
- `Overall score (0-3)`
- `Manual edit lines`
- `Judge notes`

### Deferred Metrics

The following are important but may be phased in after version 1 if implementation cost is high:

- `Consistency`
- `Debug success rate`

### Metric Definitions

#### `Pass@1`

Whether the first complete answer is acceptable without an additional retry loop.

Recommended storage:

- `0` = fail
- `1` = pass

#### `Unit test pass rate`

Percentage of test cases passed when executable validation exists.

Recommended storage:

- `0-100`

#### `Token cost`

Should include:

- `input_tokens`
- `output_tokens`
- `total_tokens`
- `estimated_cost`

If exact token usage cannot be obtained from the underlying model path, these fields may be null or estimated.

#### `Latency`

Should be split into:

- `first_response_latency_ms`
- `full_completion_latency_ms`

#### `Structure rating`

Human score for output organization, clarity, decomposition, and usability.

Recommended scale:

- `0` unusable
- `1` weak
- `2` acceptable
- `3` strong

#### `Business rating`

Human score for whether the result captures the practical intent of the old-code understanding task.

Recommended scale:

- `0` incorrect
- `1` shallow
- `2` mostly useful
- `3` strong and actionable

#### `Overall score`

Final normalized human score from `0-3`.

## Core Product Screens

### 1. Evaluation Task Page

Purpose:

- Create a new comparison run
- Select an evaluation case
- Review the exact prompt and context
- Select multiple window bindings as targets
- Start the run

Key UI sections:

- Case selector
- Full prompt viewer
- Context payload viewer
- Target selector
- Run notes

### 2. Runtime Monitoring Page

Purpose:

- Monitor target execution state
- See running, completed, failed, timed out, or cancelled targets
- Retry a failed target or cancel the run

Key UI sections:

- Per-target status cards
- Start time and elapsed time
- Error summaries
- Retry controls

### 3. Result Comparison Page

This is the most important page in version 1.

Purpose:

- Organize all model results for the same prompt in one place
- Make old-code understanding quality easy to compare

Required sections:

#### Context Header

Shows:

- Evaluation case title
- Full prompt
- Context snapshot
- Run time
- Participating models/targets

#### Side-by-Side Result Area

Each target card should include:

- Model name
- Window name
- Status
- Full output
- Conversation turns
- Total duration
- Token usage
- Error details if present

#### Metric Table

Suggested columns:

- `Model`
- `Pass@1`
- `Unit test pass rate`
- `Consistency`
- `Debug success rate`
- `Token cost`
- `Latency`
- `Structure rating`
- `Business rating`
- `Overall score`
- `Manual edit lines`

#### Auto Summary Panel

Should summarize:

- Which models arrived at similar conclusions
- Which models omitted key points
- Which target was fastest
- Which target appeared strongest overall

### 4. Evaluation Case Library

Purpose:

- Maintain reusable old-code understanding tasks
- Store prompt and context definitions
- Reuse standardized cases across comparison runs

### 5. Window and Model Configuration Page

This is a required first-class page, not a hidden settings subsection.

Purpose:

- Discover iTerm2 windows or sessions
- Manage model profiles
- Bind targets to profiles

Recommended layout:

- Left: iTerm2 target list
- Center: current binding details
- Right: profile editor

This page must make it obvious which window uses which model, base URL, and API key.

### 6. System Settings Page

Purpose:

- Manage default timeout
- Manage concurrency
- Manage database path
- Manage export options
- Manage secret storage strategy

## Core Workflow

1. Create or select an evaluation case from the case library
2. Ensure each iTerm2 target is bound to the correct model profile
3. Create a new comparison run from the evaluation task page
4. Select multiple targets
5. Dispatch the same prompt and context snapshot to all targets
6. Collect messages, outputs, timings, and errors
7. Compute objective metrics and summary analysis
8. Present side-by-side outputs and evaluation metrics
9. Record human scores and final judgment
10. Save history and allow export

## Result Collection Boundaries

Version 1 should explicitly preserve:

- Full prompt snapshot
- Context snapshot
- Full target output
- Runtime status changes
- Error messages
- Conversation turn count
- Timing metadata

The implementation should aim to preserve full multi-turn output when available, not just the final answer.

## Export Requirements

Version 1 should support at least:

- CSV export for structured metrics
- JSON export for raw run data
- Markdown export for human-readable comparison reports

Exports must exclude or redact sensitive secrets.

## Runtime Controls

Version 1 should support:

- Configurable timeout
- Retry for a single failed target
- Cancel the current run
- Re-run an entire comparison

Implementation recommendations:

- Max concurrency should default to the number of selected targets in version 1, with an optional global cap exposed in system settings
- One target failure should not invalidate the whole run
- The run should complete with mixed target statuses when necessary

## Security Considerations

- API keys should be encrypted at rest where possible
- Logs should avoid plaintext secret output
- Exported reports should not include secrets
- Profile snapshots should redact credentials in UI and export surfaces

## Version 1 Scope

Version 1 should include:

- Evaluation case management
- Window discovery and model profile binding
- Comparison run creation
- Multi-target prompt dispatch
- Full result collection
- Side-by-side comparison view
- Structured evaluation metrics
- Human scoring
- Local history
- Export support

Version 1 should not require:

- Cloud sync
- Team workflows
- Complex benchmark suites
- Long-horizon analytics dashboards

## Risks and Open Questions

1. Token accuracy
   Exact token accounting may not always be available through the iTerm2 + MCP integration path.

2. Output boundary detection
   The adapter must reliably determine when a target response is complete.

3. Validation depth
   `Pass@1` and unit test pass rate require clear task-specific validation strategies.

4. Consistency metric cost
   Repeated executions increase runtime and cost, so this metric may need phased rollout.

5. Debug success rate design
   This metric implies a separate evaluation mode where a failure is followed by a repair attempt.

## Recommended Phase Plan

### Phase 1

- Core data model
- Window/profile binding
- Evaluation case library
- Comparison run execution
- Result comparison page
- Base metrics

### Phase 2

- Consistency runs
- Debug evaluation mode
- Richer difference summaries
- Better export templates

## Summary

This design defines a local-first desktop evaluation workbench specifically optimized for comparing multiple models through iTerm2 windows under identical prompt conditions. The center of the product is the comparison result, not the chat session. The design supports old-code understanding evaluation with reusable cases, target bindings, structured metrics, human scoring, and historical traceability.
