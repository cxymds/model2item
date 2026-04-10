import { formatDurationMs, parseJsonSafe, shortenId } from "../../../lib/formatters";
import type { ComparisonSummaryTargetResponse, ComparisonTargetResponse } from "../../../types/api";
import type { MetricRow } from "../components/MetricTable";
import type { ResultComparisonColumn } from "../components/ResultComparisonGrid";
import type { RunTargetStatus } from "../components/RunTargetStatusCard";

type ProfileSnapshot = {
  provider: string;
  model_name: string;
};

function isProfileSnapshot(value: unknown): value is ProfileSnapshot {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.provider === "string" &&
    candidate.provider.length > 0 &&
    typeof candidate.model_name === "string" &&
    candidate.model_name.length > 0
  );
}

function getLocalizedRunStatusText(status: string) {
  if (status === "running") return "运行中";
  if (status === "done" || status === "finished" || status === "completed") return "已完成";
  if (status === "failed" || status === "error") return "失败";
  return "排队中";
}

export type RunTargetViewModel = {
  id: string;
  label: string;
  status: RunTargetStatus;
  statusText: string;
  summary: string;
  metricRow: MetricRow;
};

export function mapRunTargetStatus(status: string): RunTargetStatus {
  if (status === "running") return "running";
  if (status === "done" || status === "finished" || status === "completed") return "done";
  if (status === "failed" || status === "error") return "failed";
  return "queued";
}

function buildRunTargetLabel(target: ComparisonTargetResponse): string {
  const parsed = parseJsonSafe<unknown>(target.profile_snapshot_json);
  if (!isProfileSnapshot(parsed)) return `目标 ${shortenId(target.id)}`;
  const snapshot = parsed;
  return `${snapshot.provider} / ${snapshot.model_name}`;
}

export function buildRunTargetViewModel(target: ComparisonTargetResponse): RunTargetViewModel {
  const label = buildRunTargetLabel(target);
  const status = mapRunTargetStatus(target.status);
  const statusText = getLocalizedRunStatusText(target.status);

  return {
    id: target.id,
    label,
    status,
    statusText,
    summary: `状态：${statusText}，行数：${target.response_lines}，字符数：${target.response_chars}`,
    metricRow: {
      model: label,
      passAt1: target.success_status === "success" ? "1" : target.success_status === null ? "-" : "0",
      testRate: "-",
      latency: formatDurationMs(target.duration_ms),
      tokenCost: "-",
      overall: statusText,
    },
  };
}

export function buildRunTargetViewModels(targets: ComparisonTargetResponse[]): RunTargetViewModel[] {
  return targets.map(buildRunTargetViewModel);
}

export function buildRunTargetViewModelFromSummary(
  target: ComparisonSummaryTargetResponse,
): RunTargetViewModel {
  return {
    id: target.target_id,
    label: target.label,
    status: mapRunTargetStatus(target.status),
    statusText: getLocalizedRunStatusText(target.status),
    summary: `状态：${getLocalizedRunStatusText(target.status)}，行数：${target.response_lines}，字符数：${target.response_chars}`,
    metricRow: {
      model: target.label,
      passAt1: target.success_status === "success" ? "1" : target.success_status === null ? "-" : "0",
      testRate: "-",
      latency: formatDurationMs(target.duration_ms),
      tokenCost: "-",
      overall: getLocalizedRunStatusText(target.status),
    },
  };
}

export function buildRunTargetViewModelsFromSummary(
  targets: ComparisonSummaryTargetResponse[],
): RunTargetViewModel[] {
  return targets.map(buildRunTargetViewModelFromSummary);
}

export function toComparisonColumns(items: RunTargetViewModel[]): ResultComparisonColumn[] {
  return items.map((item) => ({
    label: item.label,
    summary: item.summary,
  }));
}

export function toMetricRows(items: RunTargetViewModel[]): MetricRow[] {
  return items.map((item) => item.metricRow);
}
