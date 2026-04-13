ALTER TABLE model_profiles
ADD COLUMN execution_mode TEXT NOT NULL DEFAULT 'claude_cli';

UPDATE model_profiles
SET execution_mode = CASE
    WHEN lower(trim(coalesce(provider, ''))) IN ('openai', 'openai-compatible', 'openrouter', 'litellm') THEN 'openai_chat'
    WHEN lower(trim(coalesce(provider, ''))) IN ('anthropic', 'claude', '') THEN 'claude_cli'
    ELSE 'claude_cli'
END;

UPDATE model_profiles
SET provider = CASE
    WHEN execution_mode = 'openai_chat' THEN 'openai'
    ELSE 'anthropic'
END;
