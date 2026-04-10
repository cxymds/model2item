import { useMutation, useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  createComparisonRun,
  listEvaluationCases,
  listWindowBindings,
  startComparisonRun,
} from "../../../lib/tauri";
import type { CreateComparisonRunInput } from "../../../types/api";

const initialDraft: CreateComparisonRunInput = {
  evaluation_case_id: "",
  title: "",
  target_ids: [],
  notes: "",
};

export function CreateRunPage() {
  const navigate = useNavigate();
  const [draft, setDraft] = useState<CreateComparisonRunInput>(initialDraft);

  const casesQuery = useQuery({
    queryKey: ["evaluation-cases"],
    queryFn: listEvaluationCases,
  });
  const bindingsQuery = useQuery({
    queryKey: ["window-bindings"],
    queryFn: listWindowBindings,
  });
  const createRunMutation = useMutation({
    mutationFn: createComparisonRun,
    onSuccess: async (run) => {
      await startComparisonRun(run.id);
      await navigate(`/runs/${run.id}`);
    },
  });

  const canSubmit = useMemo(() => {
    return (
      draft.evaluation_case_id.length > 0 &&
      draft.title.trim().length > 0 &&
      draft.target_ids.length > 0 &&
      !createRunMutation.isPending
    );
  }, [draft, createRunMutation.isPending]);

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>Create Run</h2>
        <p>Select an evaluation case and target windows to prepare a comparison run.</p>
      </header>

      <form
        className="run-form"
        onSubmit={(event) => {
          event.preventDefault();
          if (!canSubmit) return;
          createRunMutation.mutate({
            evaluation_case_id: draft.evaluation_case_id,
            title: draft.title.trim(),
            target_ids: draft.target_ids,
            notes: draft.notes?.trim() ?? "",
          });
        }}
      >
        <label className="field">
          <span>Run title</span>
          <input
            value={draft.title}
            onChange={(event) => {
              setDraft((current) => ({ ...current, title: event.target.value }));
            }}
            placeholder="Legacy parser benchmark - batch A"
            required
          />
        </label>

        <label className="field">
          <span>Evaluation case</span>
          <select
            value={draft.evaluation_case_id}
            onChange={(event) => {
              setDraft((current) => ({ ...current, evaluation_case_id: event.target.value }));
            }}
            required
          >
            <option value="">Select a saved case</option>
            {(casesQuery.data ?? []).map((item) => (
              <option key={item.id} value={item.id}>
                {item.title}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>Target bindings (multi-select)</span>
          <select
            multiple
            size={Math.max(4, (bindingsQuery.data ?? []).length || 4)}
            value={draft.target_ids}
            onChange={(event) => {
              const selectedValues = Array.from(event.currentTarget.selectedOptions).map(
                (option) => option.value,
              );
              const values =
                selectedValues.length > 0
                  ? selectedValues
                  : event.currentTarget.value
                    ? [event.currentTarget.value]
                    : [];
              setDraft((current) => ({ ...current, target_ids: values }));
            }}
            required
          >
            {(bindingsQuery.data ?? []).map((item) => (
              <option key={item.id} value={item.id}>
                {item.display_name} ({item.iterm_session_id})
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>Run notes</span>
          <textarea
            rows={4}
            value={draft.notes ?? ""}
            onChange={(event) => {
              setDraft((current) => ({ ...current, notes: event.target.value }));
            }}
            placeholder="Why are we running this comparison?"
          />
        </label>

        {casesQuery.isError ? (
          <p className="error-text">Failed to load cases. {String(casesQuery.error)}</p>
        ) : null}
        {bindingsQuery.isError ? (
          <p className="error-text">Failed to load bindings. {String(bindingsQuery.error)}</p>
        ) : null}
        {createRunMutation.isError ? (
          <p className="error-text">Failed to create run. {String(createRunMutation.error)}</p>
        ) : null}

        <button className="primary-btn" disabled={!canSubmit} type="submit">
          {createRunMutation.isPending ? "Starting..." : "Start run"}
        </button>
      </form>
    </section>
  );
}
