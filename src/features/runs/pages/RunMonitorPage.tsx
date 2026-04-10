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
        <h2>Run Monitor</h2>
        <p>
          {runQuery.data
            ? `${runQuery.data.title} (${runQuery.data.status})`
            : `Run ID: ${normalizedRunId || "Unknown"}`}
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
            {startRunMutation.isPending ? "Starting execution..." : "Start execution"}
          </button>
        ) : null}
      </header>

      {runQuery.isLoading || targetsQuery.isLoading ? <p className="muted">Loading run state...</p> : null}
      {runQuery.isError ? <p className="error-text">Failed to load run. {String(runQuery.error)}</p> : null}
      {targetsQuery.isError ? (
        <p className="error-text">Failed to load targets. {String(targetsQuery.error)}</p>
      ) : null}
      {startRunMutation.isError ? (
        <p className="error-text">Failed to start execution. {String(startRunMutation.error)}</p>
      ) : null}

      <div className="status-grid">
        {targetViewModels.map((target) => (
          <RunTargetStatusCard
            key={target.id}
            status={target.status}
            title={target.label}
            subtitle={`Target ${target.id} currently ${target.statusText}`}
          />
        ))}
      </div>

      {targetsQuery.data && targetsQuery.data.length === 0 ? (
        <p className="muted">No targets found for this run yet.</p>
      ) : null}
    </section>
  );
}
