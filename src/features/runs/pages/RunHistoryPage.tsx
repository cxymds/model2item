import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { formatDateTime } from "../../../lib/formatters";
import type { ComparisonRunResponse } from "../../../types/api";
import { comparisonRunsQuery } from "../lib/runQueries";

function isActiveRun(run: ComparisonRunResponse) {
  return run.status === "queued" || run.status === "running";
}

function getRunDestination(run: ComparisonRunResponse) {
  if (isActiveRun(run)) {
    return `/runs/${run.id}`;
  }

  return `/runs/${run.id}/results`;
}

function getRunActionLabel(run: ComparisonRunResponse) {
  if (isActiveRun(run)) {
    return `继续查看 ${run.title}`;
  }

  return `查看结果 ${run.title}`;
}

export function RunHistoryPage() {
  const runsQuery = useQuery({
    ...comparisonRunsQuery(),
    refetchInterval: 2000,
  });

  const runs = runsQuery.data ?? [];
  const activeRuns = runs.filter(isActiveRun);

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>历史运行</h2>
        <p>查看最近 20 次对比运行，并快速回到当前仍在执行的任务。</p>
      </header>

      {runsQuery.isLoading ? <p className="muted">正在加载历史运行...</p> : null}
      {runsQuery.isError ? <p className="error-text">加载历史运行失败。{String(runsQuery.error)}</p> : null}

      <section className="stack-block">
        <header className="section-header">
          <h3>当前运行</h3>
          <p>这里会展示仍处于排队中或运行中的任务。</p>
        </header>

        {activeRuns.length === 0 ? <p className="muted">当前没有运行中的任务。</p> : null}

        {activeRuns.map((run) => (
          <article className="status-card" key={run.id}>
            <h4>{run.title}</h4>
            <p>
              状态：{run.status}，创建时间：{formatDateTime(run.created_at)}
            </p>
            <Link aria-label={getRunActionLabel(run)} className="secondary-btn" to={getRunDestination(run)}>
              继续查看
            </Link>
          </article>
        ))}
      </section>

      <section className="stack-block">
        <header className="section-header">
          <h3>最近结果</h3>
          <p>最近 20 条运行记录，按创建时间倒序显示。</p>
        </header>

        {runs.length === 0 ? <p className="muted">当前还没有历史运行记录。</p> : null}

        {runs.map((run) => (
          <article className="status-card" key={run.id}>
            <h4>{run.title}</h4>
            <p>
              状态：{run.status}，创建时间：{formatDateTime(run.created_at)}，完成时间：
              {formatDateTime(run.finished_at)}
            </p>
            <Link aria-label={getRunActionLabel(run)} className="secondary-btn" to={getRunDestination(run)}>
              {isActiveRun(run) ? "继续查看" : "查看结果"}
            </Link>
          </article>
        ))}
      </section>
    </section>
  );
}
