import { useState } from "react";
import type { CreateEvaluationCaseInput } from "../../../types/api";

type CaseFormProps = {
  isPending: boolean;
  initialValue?: CreateEvaluationCaseInput;
  submitLabel?: string;
  resetOnSubmit?: boolean;
  onSubmit: (input: CreateEvaluationCaseInput) => void;
};

const initialState: CreateEvaluationCaseInput = {
  title: "",
  prompt: "",
  context_payload: "{}",
  notes: "",
};

export function CaseForm({
  isPending,
  initialValue,
  submitLabel,
  resetOnSubmit = true,
  onSubmit,
}: CaseFormProps) {
  const [form, setForm] = useState<CreateEvaluationCaseInput>(initialValue ?? initialState);
  const [jsonError, setJsonError] = useState<string>("");

  return (
    <form
      className="stack-form"
      onSubmit={(event) => {
        event.preventDefault();
        if (!form.title.trim() || !form.prompt.trim()) return;
        try {
          JSON.parse(form.context_payload);
          setJsonError("");
        } catch {
          setJsonError("上下文载荷必须是合法的 JSON。");
          return;
        }
        onSubmit({
          ...form,
          notes: form.notes?.trim() ?? "",
        });
        if (resetOnSubmit) {
          setForm(initialState);
        }
      }}
    >
      <label className="field">
        <span>案例标题</span>
        <input
          value={form.title}
          onChange={(event) => {
            setForm((current) => ({ ...current, title: event.target.value }));
          }}
          placeholder="旧版解析器走查"
          required
        />
      </label>

      <label className="field">
        <span>完整提示词</span>
        <textarea
          value={form.prompt}
          onChange={(event) => {
            setForm((current) => ({ ...current, prompt: event.target.value }));
          }}
          rows={7}
          placeholder="分析这个旧代码模块并说明其实现逻辑..."
          required
        />
      </label>

      <label className="field">
        <span>上下文载荷（JSON 字符串）</span>
        <textarea
          value={form.context_payload}
          onChange={(event) => {
            if (jsonError) setJsonError("");
            setForm((current) => ({ ...current, context_payload: event.target.value }));
          }}
          rows={4}
        />
      </label>
      {jsonError ? <p className="error-text">{jsonError}</p> : null}

      <label className="field">
        <span>备注</span>
        <textarea
          value={form.notes ?? ""}
          onChange={(event) => {
            setForm((current) => ({ ...current, notes: event.target.value }));
          }}
          rows={3}
          placeholder="希望评审重点关注什么？"
        />
      </label>

      <button className="primary-btn" disabled={isPending} type="submit">
        {isPending ? "保存中..." : submitLabel ?? "保存评测案例"}
      </button>
    </form>
  );
}
