export type RunTargetStatus = "queued" | "running" | "done" | "failed";

type RunTargetStatusCardProps = {
  title: string;
  subtitle: string;
  status: RunTargetStatus;
};

export function RunTargetStatusCard({ title, subtitle, status }: RunTargetStatusCardProps) {
  return (
    <article className="status-card">
      <h4>{title}</h4>
      <p>{subtitle}</p>
      <span className={`status-pill ${status}`}>{status.toUpperCase()}</span>
    </article>
  );
}
