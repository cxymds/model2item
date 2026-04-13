CREATE TABLE custom_providers (
  id TEXT PRIMARY KEY NOT NULL,
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

ALTER TABLE window_bindings
ADD COLUMN custom_provider_id TEXT REFERENCES custom_providers(id);

INSERT INTO custom_providers (
  id,
  name,
  provider_key,
  client_type,
  base_url,
  api_key_encrypted,
  default_model,
  enabled,
  extra_params_json,
  created_at,
  updated_at
)
SELECT
  'provider-' || id,
  name,
  provider,
  execution_mode,
  base_url,
  api_key_encrypted,
  model_name,
  enabled,
  extra_params_json,
  created_at,
  updated_at
FROM model_profiles;

UPDATE window_bindings
SET custom_provider_id = 'provider-' || profile_id
WHERE custom_provider_id IS NULL
  AND EXISTS (
    SELECT 1
    FROM custom_providers cp
    WHERE cp.id = 'provider-' || window_bindings.profile_id
  );

CREATE INDEX idx_window_bindings_custom_provider_id
ON window_bindings(custom_provider_id);
