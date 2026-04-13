export const EXECUTION_MODE_OPTIONS = [
  { value: "claude_cli", label: "Claude CLI" },
  { value: "openai_chat", label: "OpenAI Chat" },
] as const;

export type ExecutionMode = (typeof EXECUTION_MODE_OPTIONS)[number]["value"];

export type Provider = "anthropic" | "openai";

export const PROVIDER_BY_EXECUTION_MODE: Record<ExecutionMode, Provider> = {
  claude_cli: "anthropic",
  openai_chat: "openai",
};

export function normalizeExecutionMode(executionMode: string): ExecutionMode {
  return executionMode === "openai_chat" ? "openai_chat" : "claude_cli";
}

export function getExecutionModeLabel(executionMode: ExecutionMode): string {
  return executionMode === "openai_chat" ? "OpenAI Chat" : "Claude CLI";
}

export type ProfileResponse = {
  id: string;
  name: string;
  provider: Provider;
  execution_mode: ExecutionMode;
  model_name: string;
  base_url: string;
  system_prompt: string;
  temperature: number | null;
  max_tokens: number | null;
  extra_params_json: string;
  created_at: string;
  updated_at: string;
};

export type CreateProfileInput = {
  name: string;
  provider: Provider;
  execution_mode: ExecutionMode;
  model_name: string;
  base_url: string;
  api_key: string;
};

export type UpdateProfileInput = {
  name: string;
  provider: Provider;
  execution_mode: ExecutionMode;
  model_name: string;
  base_url: string;
  api_key: string;
};

export type WindowBindingResponse = {
  id: string;
  iterm_session_id: string;
  display_name: string;
  profile_id: string;
  enabled: number;
  last_seen_at: string | null;
  metadata_json: string;
};

export type ItermSessionResponse = {
  session_id: string;
  window_id: string;
  window_title: string;
  tab_id: string;
  tab_title: string;
  session_title: string;
};

export type CreateWindowBindingInput = {
  iterm_session_id: string;
  display_name: string;
  profile_id: string;
};

export type UpdateWindowBindingInput = {
  iterm_session_id: string;
  display_name: string;
  profile_id: string;
};

export type EvaluationCaseResponse = {
  id: string;
  title: string;
  prompt: string;
  context_payload: string;
  expected_checkpoints_json: string;
  validation_rules_json: string;
  notes: string;
  created_at: string;
  updated_at: string;
};

export type CreateEvaluationCaseInput = {
  title: string;
  prompt: string;
  context_payload: string;
  notes?: string;
};

export type UpdateEvaluationCaseInput = {
  title: string;
  prompt: string;
  context_payload: string;
  notes?: string;
};

export type CreateComparisonRunInput = {
  evaluation_case_id: string;
  title: string;
  target_ids: string[];
  notes?: string;
};

export type ComparisonRunResponse = {
  id: string;
  evaluation_case_id: string;
  title: string;
  status: string;
  prompt_snapshot: string;
  context_snapshot: string;
  created_at: string;
  started_at: string | null;
  finished_at: string | null;
  notes: string;
};

export type ComparisonTargetResponse = {
  position: number;
  id: string;
  run_id: string;
  window_binding_id: string;
  profile_snapshot_json: string;
  status: string;
  sent_at: string | null;
  first_response_at: string | null;
  finished_at: string | null;
  duration_ms: number | null;
  response_chars: number;
  response_lines: number;
  success_status: string | null;
  error_category: string | null;
  error_detail: string | null;
  latest_message_role: string | null;
  latest_message_content: string | null;
};

export type ComparisonSummaryTargetResponse = {
  target_id: string;
  label: string;
  display_name: string | null;
  provider: string | null;
  model_name: string | null;
  status: string;
  success_status: string | null;
  duration_ms: number | null;
  response_chars: number;
  response_lines: number;
};

export type ComparisonSummaryResponse = {
  run: ComparisonRunResponse;
  targets: ComparisonSummaryTargetResponse[];
  fastest_target_id: string | null;
  longest_target_id: string | null;
  queued_count: number;
  summary_text: string;
};

export type ComparisonMessageResponse = {
  id: string;
  comparison_target_id: string;
  role: string;
  content: string;
  message_type: string;
  created_at: string;
};
