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
  const activeRun = runs.find(isActiveRun) ?? null;
  const completedRuns = runs.filter((run) => !isActiveRun(run));

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
          <p>系统当前只允许一个运行中的任务。</p>
        </header>

        {!activeRun ? <p className="muted">当前没有运行中的任务。</p> : null}

        {activeRun ? (
          <article className="status-card" key={activeRun.id}>
            <h4>{activeRun.title}</h4>
            <p>
              状态：{activeRun.status}，创建时间：{formatDateTime(activeRun.created_at)}
            </p>
            <Link
              aria-label={getRunActionLabel(activeRun)}
              className="secondary-btn"
              to={getRunDestination(activeRun)}
            >
              继续查看
            </Link>
          </article>
        ) : null}
      </section>

      <section className="stack-block">
        <header className="section-header">
          <h3>最近结果</h3>
          <p>最近 20 条运行记录，按创建时间倒序显示。</p>
        </header>

        {completedRuns.length === 0 ? <p className="muted">当前还没有历史运行记录。</p> : null}

        {completedRuns.map((run) => (
          <article className="status-card" key={run.id}>
            <h4>{run.title}</h4>
            <p>
              状态：{run.status}，创建时间：{formatDateTime(run.created_at)}，完成时间：
              {formatDateTime(run.finished_at)}
            </p>
            <Link aria-label={getRunActionLabel(run)} className="secondary-btn" to={getRunDestination(run)}>
              查看结果
            </Link>
          </article>
        ))}
      </section>
    </section>
  );
}
