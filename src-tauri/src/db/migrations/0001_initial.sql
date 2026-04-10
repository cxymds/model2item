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
