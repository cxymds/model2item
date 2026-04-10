import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  createComparisonRun,
  listEvaluationCases,
  listItermSessions,
  listWindowBindings,
  refreshWindowBindingPresence,
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
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<CreateComparisonRunInput>(initialDraft);

  const casesQuery = useQuery({
    queryKey: ["evaluation-cases"],
    queryFn: listEvaluationCases,
  });
  const bindingsQuery = useQuery({
    queryKey: ["window-bindings"],
    queryFn: listWindowBindings,
  });
  const sessionsQuery = useQuery({
    queryKey: ["iterm-sessions"],
    queryFn: listItermSessions,
  });
  const createRunMutation = useMutation({
    mutationFn: async (input: CreateComparisonRunInput) => {
      const run = await createComparisonRun(input);
      await startComparisonRun(run.id);
      return run;
    },
    onSuccess: async (run) => {
      await navigate(`/runs/${run.id}`);
    },
  });
  const refreshPresenceMutation = useMutation({
    mutationFn: refreshWindowBindingPresence,
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["window-bindings"] }),
        queryClient.invalidateQueries({ queryKey: ["iterm-sessions"] }),
      ]);
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

  const onlineSessionIds = useMemo(() => {
    return new Set((sessionsQuery.data ?? []).map((session) => session.session_id));
  }, [sessionsQuery.data]);

  const bindingOptions = useMemo(() => {
    return (bindingsQuery.data ?? []).map((binding) => {
      const isOnline = onlineSessionIds.has(binding.iterm_session_id);
      return {
        ...binding,
        isOnline,
        label: `${binding.display_name} (${binding.iterm_session_id}) ${isOnline ? "在线" : "离线"}`,
      };
    });
  }, [bindingsQuery.data, onlineSessionIds]);

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>创建运行任务</h2>
        <p>选择评测案例和在线目标窗口，准备一次对比运行。</p>
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
          <span>任务标题</span>
          <input
            value={draft.title}
            onChange={(event) => {
              setDraft((current) => ({ ...current, title: event.target.value }));
            }}
            placeholder="旧代码解析基准测试 - A 组"
            required
          />
        </label>

        <label className="field">
          <span>评测案例</span>
          <select
            value={draft.evaluation_case_id}
            onChange={(event) => {
              setDraft((current) => ({ ...current, evaluation_case_id: event.target.value }));
            }}
            required
          >
            <option value="">请选择已保存案例</option>
            {(casesQuery.data ?? []).map((item) => (
              <option key={item.id} value={item.id}>
                {item.title}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>目标绑定（可多选）</span>
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
            {bindingOptions.map((item) => (
              <option disabled={!item.isOnline} key={item.id} value={item.id}>
                {item.label}
              </option>
            ))}
          </select>
        </label>
        <div className="stack-inline">
          <button
            className="secondary-btn"
            disabled={refreshPresenceMutation.isPending}
            onClick={() => {
              refreshPresenceMutation.mutate();
            }}
            type="button"
          >
            {refreshPresenceMutation.isPending ? "刷新中..." : "刷新窗口状态"}
          </button>
          <p className="muted">离线窗口会被禁用；可在当前页面刷新窗口在线状态。</p>
        </div>

        <label className="field">
          <span>运行备注</span>
          <textarea
            rows={4}
            value={draft.notes ?? ""}
            onChange={(event) => {
              setDraft((current) => ({ ...current, notes: event.target.value }));
            }}
            placeholder="这次对比运行希望验证什么？"
          />
        </label>

        {casesQuery.isError ? (
          <p className="error-text">加载案例失败。{String(casesQuery.error)}</p>
        ) : null}
        {bindingsQuery.isError ? (
          <p className="error-text">加载绑定失败。{String(bindingsQuery.error)}</p>
        ) : null}
        {sessionsQuery.isError ? (
          <p className="error-text">加载 iTerm2 会话失败。{String(sessionsQuery.error)}</p>
        ) : null}
        {refreshPresenceMutation.isError ? (
          <p className="error-text">
            刷新窗口状态失败。{String(refreshPresenceMutation.error)}
          </p>
        ) : null}
        {createRunMutation.isError ? (
          <p className="error-text">创建或启动运行任务失败。{String(createRunMutation.error)}</p>
        ) : null}

        <button className="primary-btn" disabled={!canSubmit} type="submit">
          {createRunMutation.isPending ? "启动中..." : "开始运行"}
        </button>
      </form>
    </section>
  );
}
