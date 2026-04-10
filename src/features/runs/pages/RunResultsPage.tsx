import { useMutation, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { exportComparisonRunReport } from "../../../lib/tauri";
import { MetricTable } from "../components/MetricTable";
import { ResultComparisonGrid } from "../components/ResultComparisonGrid";
import { comparisonSummaryQuery } from "../lib/runQueries";
import { saveRecentRun } from "../lib/recentRun";
import {
  buildRunTargetViewModelsFromSummary,
  toComparisonColumns,
  toMetricRows,
} from "../lib/runViewModel";

function buildSummaryText(
  runId: string,
  targets: Array<{ target_id: string; label: string }>,
  fastestTargetId: string | null,
  longestTargetId: string | null,
  queuedCount: number,
) {
  if (targets.length === 0) return "当前运行任务尚未采集到结果目标。";

  const labelById = new Map(targets.map((item) => [item.target_id, item.label]));

  return `最快目标：${labelById.get(fastestTargetId ?? "") ?? "暂无"}；最长输出：${labelById.get(longestTargetId ?? "") ?? "暂无"}；排队中目标数：${queuedCount}；运行 ID：${runId || "未知"}`;
}

export function RunResultsPage() {
  const { runId } = useParams();
  const normalizedRunId = runId ?? "";
  const [exportedPath, setExportedPath] = useState<string | null>(null);
  const summaryQuery = useQuery(comparisonSummaryQuery(normalizedRunId));
  const exportReportMutation = useMutation({
    mutationFn: (runId: string) => exportComparisonRunReport(runId),
    onSuccess: (path) => {
      setExportedPath(path);
    },
  });
  const targetViewModels = buildRunTargetViewModelsFromSummary(summaryQuery.data?.targets ?? []);

  useEffect(() => {
    if (!summaryQuery.data) return;

    saveRecentRun({
      id: summaryQuery.data.run.id,
      title: summaryQuery.data.run.title,
      status: summaryQuery.data.run.status,
    });
  }, [summaryQuery.data]);

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>运行结果</h2>
        <p>
          {summaryQuery.data
            ? `${summaryQuery.data.run.title} 的结果对比（${summaryQuery.data.run.status}）`
            : `运行任务 ${normalizedRunId || "未知"} 的并排对比视图`}
        </p>
        <button
          className="ghost-btn"
          disabled={exportReportMutation.isPending || normalizedRunId.length === 0}
          onClick={() => {
            setExportedPath(null);
            exportReportMutation.mutate(normalizedRunId);
          }}
          type="button"
        >
          {exportReportMutation.isPending ? "导出中..." : "导出 Markdown 报告"}
        </button>
      </header>

      {summaryQuery.isLoading ? <p className="muted">正在加载运行结果...</p> : null}
      {summaryQuery.isError ? (
        <p className="error-text">加载汇总结果失败。{String(summaryQuery.error)}</p>
      ) : null}
      {exportReportMutation.isError ? (
        <p className="error-text">导出报告失败。{String(exportReportMutation.error)}</p>
      ) : null}
      {exportedPath ? <p className="muted">报告已导出到 {exportedPath}</p> : null}

      <ResultComparisonGrid columns={toComparisonColumns(targetViewModels)} />

      <MetricTable rows={toMetricRows(targetViewModels)} />

      {summaryQuery.data ? (
        <p className="muted">
          {buildSummaryText(
            normalizedRunId,
            summaryQuery.data.targets,
            summaryQuery.data.fastest_target_id,
            summaryQuery.data.longest_target_id,
            summaryQuery.data.queued_count,
          )}
        </p>
      ) : null}

      {targetViewModels.length === 0 ? <p className="muted">当前运行任务尚未采集到结果目标。</p> : null}
    </section>
  );
}
