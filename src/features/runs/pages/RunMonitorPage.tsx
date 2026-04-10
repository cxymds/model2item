import { useMutation, useQuery } from "@tanstack/react-query";
import { useParams } from "react-router-dom";
import { startComparisonRun } from "../../../lib/tauri";
import { RunTargetStatusCard } from "../components/RunTargetStatusCard";
import { comparisonRunQuery, comparisonTargetsQuery } from "../lib/runQueries";
import { buildRunTargetViewModels } from "../lib/runViewModel";

export function RunMonitorPage() {
  const { runId } = useParams();
  const normalizedRunId = runId ?? "";
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
  const targetViewModels = buildRunTargetViewModels(targetsQuery.data ?? []);

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

      <div className="status-grid">
        {targetViewModels.map((target) => (
          <RunTargetStatusCard
            key={target.id}
            status={target.status}
            title={target.label}
            subtitle={`目标 ${target.id} 当前${target.statusText}`}
          />
        ))}
      </div>

      {targetsQuery.data && targetsQuery.data.length === 0 ? (
        <p className="muted">当前运行任务还没有目标。</p>
      ) : null}
    </section>
  );
}
