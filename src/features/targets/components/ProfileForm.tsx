import { useState } from "react";
import type { CreateProfileInput } from "../../../types/api";

type ProfileFormProps = {
  isPending: boolean;
  onSubmit: (input: CreateProfileInput) => void;
};

const initialState: CreateProfileInput = {
  name: "",
  provider: "",
  model_name: "",
  base_url: "",
  api_key: "",
};

export function ProfileForm({ isPending, onSubmit }: ProfileFormProps) {
  const [form, setForm] = useState<CreateProfileInput>(initialState);

  return (
    <form
      className="stack-form"
      onSubmit={(event) => {
        event.preventDefault();
        if (!form.name.trim() || !form.model_name.trim() || !form.api_key.trim()) return;
        onSubmit(form);
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
          placeholder="GPT-5.4 基线"
          required
        />
      </label>

      <label className="field">
        <span>提供方</span>
        <input
          value={form.provider}
          onChange={(event) => {
            setForm((current) => ({ ...current, provider: event.target.value }));
          }}
          placeholder="openai"
          required
        />
      </label>

      <label className="field">
        <span>模型名称</span>
        <input
          value={form.model_name}
          onChange={(event) => {
            setForm((current) => ({ ...current, model_name: event.target.value }));
          }}
          placeholder="gpt-5.4"
          required
        />
      </label>

      <label className="field">
        <span>基础地址</span>
        <input
          type="url"
          value={form.base_url}
          onChange={(event) => {
            setForm((current) => ({ ...current, base_url: event.target.value }));
          }}
          placeholder="https://api.example.com/v1"
          required
        />
      </label>

      <label className="field">
        <span>API key</span>
        <input
          type="password"
          value={form.api_key}
          onChange={(event) => {
            setForm((current) => ({ ...current, api_key: event.target.value }));
          }}
          placeholder="sk-..."
          required
        />
      </label>

      <button className="primary-btn" disabled={isPending} type="submit">
        {isPending ? "保存中..." : "创建配置"}
      </button>
    </form>
  );
}
