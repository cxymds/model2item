import type { PropsWithChildren } from "react";
import { NavLink } from "react-router-dom";
import { getRecentRunHref, getRecentRunLabel, useRecentRun } from "../runs/lib/recentRun";

const navItems = [
  { to: "/runs/new", label: "运行任务" },
  { to: "/cases", label: "案例库" },
  { to: "/targets", label: "目标配置" },
  { to: "/settings", label: "设置" }
];

export function AppShell({ children }: PropsWithChildren) {
  const recentRun = useRecentRun();

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div>
          <p className="eyebrow">评测工作台</p>
          <h1>iTerm MCP 工具</h1>
        </div>
        <nav className="nav-list">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) => (isActive ? "nav-link active" : "nav-link")}
            >
              {item.label}
            </NavLink>
          ))}
          {recentRun ? (
            <NavLink
              aria-label={getRecentRunLabel(recentRun.status)}
              to={getRecentRunHref(recentRun)}
              className={({ isActive }) => (isActive ? "nav-link active" : "nav-link")}
            >
              {getRecentRunLabel(recentRun.status)}
              <br />
              {recentRun.title}
            </NavLink>
          ) : null}
        </nav>
      </aside>
      <main className="content">{children}</main>
    </div>
  );
}
