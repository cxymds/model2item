import type { PropsWithChildren } from "react";
import { NavLink } from "react-router-dom";
import { getRecentRunHref, getRecentRunLabel, useRecentRun } from "../runs/lib/recentRun";

const navItems = [
  { to: "/runs/history", label: "历史运行" },
  { to: "/cases", label: "案例库" },
  { to: "/targets", label: "目标配置" },
  { to: "/settings", label: "设置" }
];

export function AppShell({ children }: PropsWithChildren) {
  const recentRun = useRecentRun();
  const runEntry =
    recentRun && (recentRun.status === "queued" || recentRun.status === "running")
      ? {
          to: getRecentRunHref(recentRun),
          label: getRecentRunLabel(recentRun.status),
          title: recentRun.title,
        }
      : {
          to: "/runs/new",
          label: "新建任务",
        };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div>
          <p className="eyebrow">评测工作台</p>
          <h1>iTerm MCP 工具</h1>
        </div>
        <nav className="nav-list">
          <NavLink
            aria-label={runEntry.label}
            to={runEntry.to}
            className={({ isActive }) => (isActive ? "nav-link active" : "nav-link")}
          >
            {runEntry.label}
            {"title" in runEntry ? (
              <>
                <br />
                {runEntry.title}
              </>
            ) : null}
          </NavLink>
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) => (isActive ? "nav-link active" : "nav-link")}
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
      </aside>
      <main className="content">{children}</main>
    </div>
  );
}
