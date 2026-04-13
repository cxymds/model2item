import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
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
import type { ComparisonMessageResponse } from "../../../types/api";

function buildTargetPreview(content: string | null) {
  const normalized = (content ?? "").trim();
  if (!normalized) return "";
  if (normalized.length <= 280) return normalized;
  return `${normalized.slice(0, 280)}...`;
}

type TargetConversationDrawerProps = {
  targetId: string;
  expanded: boolean;
  onToggle: () => void;
};

function buildMessageGroups(messages: ComparisonMessageResponse[]) {
  const groups: Array<{
    role: string;
    ids: string[];
    content: string;
  }> = [];

  for (const message of messages) {
    const previous = groups.at(-1);
    if (previous && previous.role === message.role) {
      previous.ids.push(message.id);
      previous.content = `${previous.content}\n\n${message.content}`;
      continue;
    }

    groups.push({
      role: message.role,
      ids: [message.id],
      content: message.content,
    });
  }

  return groups;
}

function TargetConversationDrawer({ targetId, expanded, onToggle }: TargetConversationDrawerProps) {
  const drawerRef = useRef<HTMLDivElement | null>(null);
  const previousMessageIdsRef = useRef<string[] | null>(null);
  const [unreadIds, setUnreadIds] = useState<Set<string>>(new Set());
  const [unreadCount, setUnreadCount] = useState(0);
  const [isFollowPaused, setIsFollowPaused] = useState(false);
  const targetMessagesQuery = useQuery({
    queryKey: ["target-messages", targetId],
    queryFn: () => listTargetMessages(targetId),
    enabled: true,
    refetchInterval: 2000,
  });
  const currentMessageIds = useMemo(
    () => targetMessagesQuery.data?.map((message) => message.id) ?? [],
    [targetMessagesQuery.data],
  );
  const nextHighlightedIds = useMemo(
    () =>
      new Set(
        previousMessageIdsRef.current === null
          ? currentMessageIds
          : currentMessageIds.filter((id) => !previousMessageIdsRef.current?.includes(id)),
      ),
    [currentMessageIds],
  );
  const groups = buildMessageGroups(targetMessagesQuery.data ?? []);

  useEffect(() => {
    if (!expanded || isFollowPaused || !targetMessagesQuery.data) return;

    if (drawerRef.current) {
      drawerRef.current.scrollTop = drawerRef.current.scrollHeight;
    }
  }, [currentMessageIds, expanded, isFollowPaused, targetMessagesQuery.data]);

  useEffect(() => {
    if (!targetMessagesQuery.data) return;

    if (nextHighlightedIds.size > 0) {
      setUnreadIds((current) => new Set([...current, ...nextHighlightedIds]));
    }

    if (expanded && !isFollowPaused) {
      setUnreadCount(0);
    } else if (nextHighlightedIds.size > 0) {
      setUnreadCount((current) => current + nextHighlightedIds.size);
    }

    previousMessageIdsRef.current = currentMessageIds;
  }, [currentMessageIds, expanded, isFollowPaused, nextHighlightedIds, targetMessagesQuery.data]);

  return (
    <div className="stack-block" style={{ gap: 8 }}>
      <div className="inline-actions">
        <button className="ghost-btn" onClick={onToggle} type="button">
          {expanded
            ? "收起完整日志"
            : unreadCount > 0 || (previousMessageIdsRef.current === null && currentMessageIds.length > 0)
              ? `展开完整日志 (${unreadCount || currentMessageIds.length} 条未读)`
              : "展开完整日志"}
        </button>
        {expanded ? (
          <button
            className="ghost-btn"
            onClick={() => {
              setIsFollowPaused((current) => {
                const nextValue = !current;
                if (current) {
                  setUnreadCount(0);
                  setUnreadIds(new Set());
                }
                return nextValue;
              });
            }}
            type="button"
          >
            {isFollowPaused ? "恢复跟随" : "冻结跟随"}
          </button>
        ) : null}
      </div>
      {expanded ? (
        <div className="message-drawer" ref={drawerRef}>
          {targetMessagesQuery.isLoading ? <p className="muted">正在加载会话日志...</p> : null}
          {targetMessagesQuery.isError ? (
            <p className="error-text">加载会话日志失败。{String(targetMessagesQuery.error)}</p>
          ) : null}
          {groups.map((group) => {
            const isNew = group.ids.some((id) => unreadIds.has(id));
            return (
              <article
                className={`message-bubble ${group.role} ${isNew ? "is-new" : ""}`}
                data-is-new={isNew ? "true" : "false"}
                key={group.ids.join("-")}
              >
                <strong>
                  {group.role === "assistant" ? "模型输出" : group.role === "user" ? "输入" : "系统"}
                </strong>
                <p>{group.content}</p>
              </article>
            );
          })}
          {targetMessagesQuery.data && targetMessagesQuery.data.length === 0 ? (
            <p className="muted">这个窗口暂时还没有记录到会话消息。</p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

export function RunMonitorPage() {
  const { runId } = useParams();
  const normalizedRunId = runId ?? "";
  const [followUpPrompt, setFollowUpPrompt] = useState("");
  const [expandedTargetIds, setExpandedTargetIds] = useState<string[]>([]);
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
              <TargetConversationDrawer
                expanded={expandedTargetIds.includes(target.id)}
                onToggle={() => {
                  setExpandedTargetIds((current) =>
                    current.includes(target.id)
                      ? current.filter((id) => id !== target.id)
                      : [...current, target.id],
                  );
                }}
                targetId={target.id}
              />
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
