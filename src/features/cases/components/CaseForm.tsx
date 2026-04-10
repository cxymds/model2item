import { useState } from "react";
import type { CreateEvaluationCaseInput } from "../../../types/api";

type CaseFormProps = {
  isPending: boolean;
  onSubmit: (input: CreateEvaluationCaseInput) => void;
};

const initialState: CreateEvaluationCaseInput = {
  title: "",
  prompt: "",
  context_payload: "{}",
  notes: "",
};

export function CaseForm({ isPending, onSubmit }: CaseFormProps) {
  const [form, setForm] = useState<CreateEvaluationCaseInput>(initialState);
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
          setJsonError("Context payload must be valid JSON.");
          return;
        }
        onSubmit({
          ...form,
          notes: form.notes?.trim() ?? "",
        });
        setForm(initialState);
      }}
    >
      <label className="field">
        <span>Case title</span>
        <input
          value={form.title}
          onChange={(event) => {
            setForm((current) => ({ ...current, title: event.target.value }));
          }}
          placeholder="Legacy parser walkthrough"
          required
        />
      </label>

      <label className="field">
        <span>Full prompt</span>
        <textarea
          value={form.prompt}
          onChange={(event) => {
            setForm((current) => ({ ...current, prompt: event.target.value }));
          }}
          rows={7}
          placeholder="Analyze this old codebase module and explain..."
          required
        />
      </label>

      <label className="field">
        <span>Context payload (JSON string)</span>
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
        <span>Notes</span>
        <textarea
          value={form.notes ?? ""}
          onChange={(event) => {
            setForm((current) => ({ ...current, notes: event.target.value }));
          }}
          rows={3}
          placeholder="What should reviewers pay attention to?"
        />
      </label>

      <button className="primary-btn" disabled={isPending} type="submit">
        {isPending ? "Saving..." : "Save evaluation case"}
      </button>
    </form>
  );
}
