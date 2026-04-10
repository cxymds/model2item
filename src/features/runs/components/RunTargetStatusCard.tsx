import type { ReactNode } from "react";

export type RunTargetStatus = "queued" | "running" | "done" | "failed";

type RunTargetStatusCardProps = {
  title: string;
  subtitle: string;
  status: RunTargetStatus;
  preview?: string;
  previewLabel?: string;
  details?: ReactNode;
};

const statusLabelMap: Record<RunTargetStatus, string> = {
  queued: "排队中",
  running: "运行中",
  done: "已完成",
  failed: "失败",
};

export function RunTargetStatusCard({
  title,
  subtitle,
  status,
  preview,
  previewLabel = "最新输出",
  details,
}: RunTargetStatusCardProps) {
  return (
    <article className="status-card">
      <h4>{title}</h4>
      <p>{subtitle}</p>
      {preview ? (
        <div>
          <strong>{previewLabel}</strong>
          <p style={{ whiteSpace: "pre-wrap" }}>{preview}</p>
        </div>
      ) : null}
      {details}
      <span className={`status-pill ${status}`}>{statusLabelMap[status]}</span>
    </article>
  );
}
