import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import {
  listTargetMessages,
  sendComparisonRunMessage,
  startComparisonRun,
} from "../../../lib/tauri";
import { saveRecentRun } from "../lib/recentRun";
import { RunTargetStatusCard } from "../components/RunTargetStatusCard";
import { comparisonRunQuery, comparisonTargetsQuery } from "../lib/runQueries";
import { buildRunTargetViewModels } from "../lib/runViewModel";

function buildTargetPreview(content: string | null) {
  const normalized = (content ?? "").trim();
  if (!normalized) return "";
  if (normalized.length <= 280) return normalized;
  return `${normalized.slice(0, 280)}...`;
}

export function RunMonitorPage() {
  const { runId } = useParams();
  const normalizedRunId = runId ?? "";
  const [followUpPrompt, setFollowUpPrompt] = useState("");
  const [expandedTargetId, setExpandedTargetId] = useState<string | null>(null);
  const runQuery = useQuery({
    ...comparisonRunQuery(normalizedRunId),
    refetchInterval: 2000,
  });
  const targetsQuery = useQuery({
    ...comparisonTargetsQuery(normalizedRunId),
    refetchInterval: 2000,
  });
  const startRunMutation = useMutation({
    mutationFn: startComparisonRun,
  });
  const sendPromptMutation = useMutation({
    mutationFn: ({ runId, prompt }: { runId: string; prompt: string }) =>
      sendComparisonRunMessage(runId, prompt),
    onSuccess: async () => {
      setFollowUpPrompt("");
      await Promise.all([runQuery.refetch(), targetsQuery.refetch()]);
    },
  });
  const targetMessagesQuery = useQuery({
    queryKey: ["target-messages", expandedTargetId],
    queryFn: () => listTargetMessages(expandedTargetId ?? ""),
    enabled: expandedTargetId !== null,
    refetchInterval: expandedTargetId ? 2000 : false,
  });
  const targetViewModels = buildRunTargetViewModels(targetsQuery.data ?? []);

  useEffect(() => {
    if (!runQuery.data) return;

    saveRecentRun({
      id: runQuery.data.id,
      title: runQuery.data.title,
      status: runQuery.data.status,
    });
  }, [runQuery.data]);

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>运行监控</h2>
        <p>
          {runQuery.data
            ? `${runQuery.data.title}（${runQuery.data.status}）`
            : `运行 ID：${normalizedRunId || "未知"}`}
        </p>
        {runQuery.data?.status === "queued" ? (
          <button
            className="primary-btn"
            disabled={startRunMutation.isPending}
            onClick={() => {
              startRunMutation.mutate(normalizedRunId);
            }}
            type="button"
          >
            {startRunMutation.isPending ? "启动执行中..." : "开始执行"}
          </button>
        ) : null}
      </header>

      {runQuery.isLoading || targetsQuery.isLoading ? <p className="muted">正在加载运行状态...</p> : null}
      {runQuery.isError ? <p className="error-text">加载运行任务失败。{String(runQuery.error)}</p> : null}
      {targetsQuery.isError ? (
        <p className="error-text">加载目标失败。{String(targetsQuery.error)}</p>
      ) : null}
      {startRunMutation.isError ? (
        <p className="error-text">启动执行失败。{String(startRunMutation.error)}</p>
      ) : null}
      {sendPromptMutation.isError ? (
        <p className="error-text">发送后续提示词失败。{String(sendPromptMutation.error)}</p>
      ) : null}

      {runQuery.data?.status === "running" ? (
        <form
          className="panel stack-form"
          onSubmit={(event) => {
            event.preventDefault();
            if (!followUpPrompt.trim()) return;
            sendPromptMutation.mutate({
              runId: normalizedRunId,
              prompt: followUpPrompt.trim(),
            });
          }}
        >
          <label className="field">
            <span>广播后续提示词</span>
            <textarea
              value={followUpPrompt}
              onChange={(event) => {
                setFollowUpPrompt(event.target.value);
              }}
              placeholder="继续分析异常分支，并比较每个窗口的解释差异"
              rows={4}
            />
          </label>
          <button
            className="primary-btn"
            disabled={sendPromptMutation.isPending || !followUpPrompt.trim()}
            type="submit"
          >
            {sendPromptMutation.isPending ? "发送中..." : "发送到所有窗口"}
          </button>
        </form>
      ) : null}

      <div className="status-grid">
        {targetViewModels.map((target) => (
          <RunTargetStatusCard
            key={target.id}
            status={target.status}
            title={target.label}
            subtitle={`目标 ${target.id} 当前${target.statusText}`}
            preview={buildTargetPreview(
              targetsQuery.data?.find((item) => item.id === target.id)?.latest_message_content ?? null,
            )}
            previewLabel={
              targetsQuery.data?.find((item) => item.id === target.id)?.latest_message_role === "user"
                ? "最新输入"
                : "最新输出"
            }
            details={
              <div className="stack-block" style={{ gap: 8 }}>
                <button
                  className="ghost-btn"
                  onClick={() => {
                    setExpandedTargetId((current) => (current === target.id ? null : target.id));
                  }}
                  type="button"
                >
                  {expandedTargetId === target.id ? "收起完整日志" : "展开完整日志"}
                </button>
                {expandedTargetId === target.id ? (
                  <div className="message-drawer">
                    {targetMessagesQuery.isLoading ? <p className="muted">正在加载会话日志...</p> : null}
                    {targetMessagesQuery.isError ? (
                      <p className="error-text">加载会话日志失败。{String(targetMessagesQuery.error)}</p>
                    ) : null}
                    {targetMessagesQuery.data?.map((message) => (
                      <article className={`message-bubble ${message.role}`} key={message.id}>
                        <strong>{message.role === "assistant" ? "模型输出" : message.role === "user" ? "输入" : "系统"}</strong>
                        <p>{message.content}</p>
                      </article>
                    ))}
                    {targetMessagesQuery.data && targetMessagesQuery.data.length === 0 ? (
                      <p className="muted">这个窗口暂时还没有记录到会话消息。</p>
                    ) : null}
                  </div>
                ) : null}
              </div>
            }
          />
        ))}
      </div>

      {targetsQuery.data && targetsQuery.data.length === 0 ? (
        <p className="muted">当前运行任务还没有目标。</p>
      ) : null}
    </section>
  );
}
