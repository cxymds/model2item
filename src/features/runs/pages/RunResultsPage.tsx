import { useQuery } from "@tanstack/react-query";
import { useParams } from "react-router-dom";
import { MetricTable } from "../components/MetricTable";
import { ResultComparisonGrid } from "../components/ResultComparisonGrid";
import { comparisonSummaryQuery } from "../lib/runQueries";
import {
  buildRunTargetViewModelsFromSummary,
  toComparisonColumns,
  toMetricRows,
} from "../lib/runViewModel";

export function RunResultsPage() {
  const { runId } = useParams();
  const normalizedRunId = runId ?? "";
  const summaryQuery = useQuery(comparisonSummaryQuery(normalizedRunId));
  const targetViewModels = buildRunTargetViewModelsFromSummary(summaryQuery.data?.targets ?? []);

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>Run Results</h2>
        <p>
          {summaryQuery.data
            ? `Result comparison for ${summaryQuery.data.run.title} (${summaryQuery.data.run.status})`
            : `Side-by-side comparison workspace for run: ${normalizedRunId || "Unknown"}`}
        </p>
      </header>

      {summaryQuery.isLoading ? <p className="muted">Loading run results...</p> : null}
      {summaryQuery.isError ? (
        <p className="error-text">Failed to load summary. {String(summaryQuery.error)}</p>
      ) : null}

      <ResultComparisonGrid columns={toComparisonColumns(targetViewModels)} />

      <MetricTable rows={toMetricRows(targetViewModels)} />

      {summaryQuery.data ? <p className="muted">{summaryQuery.data.summary_text}</p> : null}

      {targetViewModels.length === 0 ? <p className="muted">No result targets captured for this run.</p> : null}
    </section>
  );
}
