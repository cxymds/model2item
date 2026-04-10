export type RunTargetStatus = "queued" | "running" | "done" | "failed";

type RunTargetStatusCardProps = {
  title: string;
  subtitle: string;
  status: RunTargetStatus;
};

const statusLabelMap: Record<RunTargetStatus, string> = {
  queued: "排队中",
  running: "运行中",
  done: "已完成",
  failed: "失败",
};

export function RunTargetStatusCard({ title, subtitle, status }: RunTargetStatusCardProps) {
  return (
    <article className="status-card">
      <h4>{title}</h4>
      <p>{subtitle}</p>
      <span className={`status-pill ${status}`}>{statusLabelMap[status]}</span>
    </article>
  );
}
