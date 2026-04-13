import { useState } from "react";
import type { CreateCustomProviderInput } from "../../../types/api";

type CustomProviderFormProps = {
  isPending: boolean;
  onSubmit: (input: CreateCustomProviderInput) => void;
};

const initialState: CreateCustomProviderInput = {
  name: "",
  provider_key: "",
  client_type: "claude_cli",
  base_url: "",
  api_key: "",
  default_model: "",
  extra_params_json: "{}",
};

export function CustomProviderForm({ isPending, onSubmit }: CustomProviderFormProps) {
  const [form, setForm] = useState<CreateCustomProviderInput>(initialState);

  return (
    <form
      className="stack-form"
      onSubmit={(event) => {
        event.preventDefault();
        if (!form.name.trim() || !form.provider_key.trim() || !form.client_type.trim() || !form.default_model.trim()) {
          return;
        }

        onSubmit({
          ...form,
          name: form.name.trim(),
          provider_key: form.provider_key.trim(),
          client_type: form.client_type.trim(),
          default_model: form.default_model.trim(),
          base_url: form.base_url.trim(),
          api_key: form.api_key.trim(),
          extra_params_json: form.extra_params_json.trim() || "{}",
        });
        setForm(initialState);
      }}
    >
      <label className="field">
        <span>名称</span>
        <input
          value={form.name}
          onChange={(event) => {
            setForm((current) => ({ ...current, name: event.target.value }));
          }}
          placeholder="GLM via Claude CLI"
          required
        />
      </label>

      <label className="field">
        <span>上游标识</span>
        <input
          value={form.provider_key}
          onChange={(event) => {
            setForm((current) => ({ ...current, provider_key: event.target.value }));
          }}
          placeholder="glm"
          required
        />
      </label>

      <label className="field">
        <span>客户端类型</span>
        <input
          value={form.client_type}
          onChange={(event) => {
            setForm((current) => ({ ...current, client_type: event.target.value }));
          }}
          placeholder="claude_cli"
          required
        />
      </label>

      <label className="field">
        <span>默认模型</span>
        <input
          value={form.default_model}
          onChange={(event) => {
            setForm((current) => ({ ...current, default_model: event.target.value }));
          }}
          placeholder="glm-5.1"
          required
        />
      </label>

      <label className="field">
        <span>Base URL</span>
        <input
          value={form.base_url}
          onChange={(event) => {
            setForm((current) => ({ ...current, base_url: event.target.value }));
          }}
          placeholder="https://gateway.example.com/v1"
        />
      </label>

      <label className="field">
        <span>API Key</span>
        <input
          type="password"
          value={form.api_key}
          onChange={(event) => {
            setForm((current) => ({ ...current, api_key: event.target.value }));
          }}
          placeholder="sk-..."
        />
      </label>

      <label className="field">
        <span>额外参数</span>
        <input
          value={form.extra_params_json}
          onChange={(event) => {
            setForm((current) => ({ ...current, extra_params_json: event.target.value }));
          }}
          placeholder="{}"
        />
      </label>

      <button className="primary-btn" disabled={isPending} type="submit">
        {isPending ? "保存中..." : "保存 Provider"}
      </button>
    </form>
  );
}
