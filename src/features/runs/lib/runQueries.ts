import {
  getComparisonRun,
  getComparisonSummary,
  listComparisonRuns,
  listComparisonTargets,
} from "../../../lib/tauri";

export function comparisonRunsQuery() {
  return {
    queryKey: ["comparison-runs"],
    queryFn: listComparisonRuns,
  };
}

export function comparisonRunQuery(runId: string) {
  return {
    queryKey: ["comparison-run", runId],
    queryFn: () => getComparisonRun(runId),
    enabled: runId.length > 0,
  };
}

export function comparisonTargetsQuery(runId: string) {
  return {
    queryKey: ["comparison-targets", runId],
    queryFn: () => listComparisonTargets(runId),
    enabled: runId.length > 0,
  };
}

export function comparisonSummaryQuery(runId: string) {
  return {
    queryKey: ["comparison-summary", runId],
    queryFn: () => getComparisonSummary(runId),
    enabled: runId.length > 0,
  };
}
