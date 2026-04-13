# Custom Provider Model Selection Design

## Goal

Replace the current "profile + model_name" mental model with a first-class custom provider model so that window routing is determined by an explicit provider configuration instead of loosely passing a model string through multiple execution paths.

## Problem

The current configuration model stores `provider`, `execution_mode`, `model_name`, `base_url`, and `api_key` on a profile, then window bindings reference that profile. This creates three problems:

1. "Model selection" is not a true control point. The system mostly routes by provider/base URL/client behavior, while `model_name` is only a string passed through later.
2. Claude CLI upstream use cases such as `Claude CLI / GLM-5.1` are valid, but the current UI and data model do not make that relationship explicit.
3. Window behavior is conceptually tied to the actual upstream provider configuration, not to a reusable profile abstraction.

## Design Summary

Introduce a first-class `custom_providers` concept and make window bindings resolve to a concrete provider configuration. A provider configuration explicitly defines:

- which client launches the request
- which upstream provider semantics apply
- which base URL and API key are used
- which default model is selected

Under this design, "model selection" becomes "select the provider configuration this window should use."

## Core Entities

### Custom Provider

Add a new persisted entity:

- `id`
- `name`
- `provider_key`
- `client_type`
- `base_url`
- `api_key_encrypted`
- `default_model`
- `enabled`
- `extra_params_json`
- `created_at`
- `updated_at`

Field semantics:

- `name`: user-facing label such as `GLM via Claude CLI`
- `provider_key`: upstream identity such as `glm`, `kimi`, `qwen`, `anthropic`, `openai`
- `client_type`: execution client such as `claude_cli` or `openai_chat`
- `default_model`: default model string such as `glm-5.1`
- `extra_params_json`: client-specific launch options such as CLI args, env overrides, or cwd

### Window Binding

Window bindings should resolve to a concrete provider configuration.

Preferred end state:

- `window_bindings.custom_provider_id`

Compatibility migration phase:

- keep `profile_id`
- add `custom_provider_id`
- runtime prefers `custom_provider_id`
- legacy bindings continue to work until migrated

### Legacy Profile

Existing profiles remain temporarily for migration only. Their long-term role should be reduced or removed once bindings and runs are fully based on custom providers.

## Execution Semantics

Runtime should no longer infer request behavior from loosely related fields. Instead, it should load the resolved custom provider and execute based on that provider's configuration.

### Claude CLI

For `client_type = claude_cli`:

- launch the CLI client
- pass `--model <default_model>`
- inject environment variables based on `provider_key`
- use `ANTHROPIC_*` env vars for `anthropic` or `claude`
- use `OPENAI_*` env vars for OpenAI-compatible upstreams such as `glm`, `kimi`, `qwen`, `openrouter`, and similar custom providers

### OpenAI Chat

For `client_type = openai_chat`:

- send requests directly using `base_url`
- authenticate with `api_key`
- use `default_model` as the request model

## UI Changes

### Provider Configuration Page

Rename the current "模型配置" concept to "Provider 配置".

Provider creation/edit fields should be:

- 名称
- 上游标识
- 客户端类型
- 默认模型
- Base URL
- API Key
- 额外参数

This makes the user-facing configuration match the actual routing behavior.

### Window Binding UI

Window binding selection should bind a window directly to a provider configuration.

Display example:

- `Claude CLI / GLM / glm-5.1`
- `Claude CLI / Kimi / kimi-k2.5`
- `OpenAI Chat / OpenAI / gpt-5.4`

The selected window should clearly show:

- client type
- provider key
- default model

## Migration Strategy

### Phase 1

- add `custom_providers`
- add `custom_provider_id` to `window_bindings`
- create provider records from existing profiles
- backfill `window_bindings.custom_provider_id`
- keep profiles readable for compatibility

### Phase 2

- switch runtime reads to prefer `custom_provider_id`
- update create/edit flows to write provider-first data
- update run snapshots to capture provider configuration rather than legacy profile shape

### Phase 3

- remove or de-emphasize legacy profile flows
- optionally migrate history references to provider snapshots only

## Error Handling

Failures should reference the provider name shown in the UI, because that is now the true execution source.

Examples:

- missing API key for provider `GLM via Claude CLI`
- cannot start target `Window A` because provider `Kimi via Claude CLI` is missing secure storage credentials

## Testing Requirements

Tests should cover:

- provider CRUD
- migration from existing profiles
- binding creation using `custom_provider_id`
- Claude CLI env mapping by `provider_key`
- OpenAI chat requests using provider fields
- run snapshot correctness after provider resolution
- deletion constraints for providers that are still referenced by bindings or run history

## Recommendation

Implement this in a compatibility-first migration:

1. add custom providers and dual-read binding resolution
2. switch UI and runtime to provider-first behavior
3. remove legacy profile dependency after the new path is stable
