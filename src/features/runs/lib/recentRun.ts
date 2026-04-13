import { useEffect, useState } from "react";

export type RecentRunRecord = {
  id: string;
  title: string;
  status: string;
};

const RECENT_RUN_STORAGE_KEY = "recent-comparison-run";
const RECENT_RUN_EVENT = "recent-comparison-run-updated";

export function loadRecentRun(): RecentRunRecord | null {
  if (typeof window === "undefined") return null;

  const rawValue = window.localStorage.getItem(RECENT_RUN_STORAGE_KEY);
  if (!rawValue) return null;

  try {
    const parsed = JSON.parse(rawValue) as Partial<RecentRunRecord>;
    if (
      typeof parsed.id !== "string" ||
      parsed.id.length === 0 ||
      typeof parsed.title !== "string" ||
      parsed.title.length === 0 ||
      typeof parsed.status !== "string" ||
      parsed.status.length === 0
    ) {
      return null;
    }

    return {
      id: parsed.id,
      title: parsed.title,
      status: parsed.status,
    };
  } catch {
    return null;
  }
}

export function saveRecentRun(run: RecentRunRecord) {
  if (typeof window === "undefined") return;

  window.localStorage.setItem(RECENT_RUN_STORAGE_KEY, JSON.stringify(run));
  window.dispatchEvent(new Event(RECENT_RUN_EVENT));
}

export function clearRecentRun() {
  if (typeof window === "undefined") return;

  window.localStorage.removeItem(RECENT_RUN_STORAGE_KEY);
  window.dispatchEvent(new Event(RECENT_RUN_EVENT));
}

export function useRecentRun() {
  const [recentRun, setRecentRun] = useState<RecentRunRecord | null>(() => loadRecentRun());

  useEffect(() => {
    if (typeof window === "undefined") return undefined;

    const handleChange = () => {
      setRecentRun(loadRecentRun());
    };

    window.addEventListener("storage", handleChange);
    window.addEventListener(RECENT_RUN_EVENT, handleChange);

    return () => {
      window.removeEventListener("storage", handleChange);
      window.removeEventListener(RECENT_RUN_EVENT, handleChange);
    };
  }, []);

  return recentRun;
}

export function getRecentRunLabel(status: string) {
  if (status === "queued" || status === "running") return "当前进行中的任务";
  return "查看最近运行结果";
}

export function getRecentRunHref(run: RecentRunRecord) {
  if (run.status === "queued" || run.status === "running") {
    return `/runs/${run.id}`;
  }

  return `/runs/${run.id}/results`;
}
