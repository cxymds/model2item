import { act, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { saveRecentRun } from "../runs/lib/recentRun";
import { AppShell } from "./AppShell";

describe("AppShell", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("renders the primary navigation items", () => {
    render(
      <MemoryRouter>
        <AppShell>
          <div>content</div>
        </AppShell>
      </MemoryRouter>,
    );

    expect(screen.getByText("运行任务")).toBeInTheDocument();
    expect(screen.getByText("历史运行")).toBeInTheDocument();
    expect(screen.getByText("案例库")).toBeInTheDocument();
    expect(screen.getByText("目标配置")).toBeInTheDocument();
    expect(screen.getByText("设置")).toBeInTheDocument();
  });

  it("renders a shortcut back to the most recent active run", () => {
    saveRecentRun({
      id: "run-1",
      title: "Legacy parser benchmark",
      status: "running",
    });

    render(
      <MemoryRouter>
        <AppShell>
          <div>content</div>
        </AppShell>
      </MemoryRouter>,
    );

    const recentRunLink = screen.getByRole("link", { name: "返回运行中任务" });
    expect(recentRunLink).toHaveAttribute("href", "/runs/run-1");
    expect(recentRunLink).toHaveTextContent("Legacy parser benchmark");
  });

  it("updates when the recent run changes after the shell has mounted", () => {
    render(
      <MemoryRouter>
        <AppShell>
          <div>content</div>
        </AppShell>
      </MemoryRouter>,
    );

    expect(screen.queryByRole("link", { name: "返回运行中任务" })).not.toBeInTheDocument();

    act(() => {
      saveRecentRun({
        id: "run-2",
        title: "Session switch benchmark",
        status: "running",
      });
    });

    const recentRunLink = screen.getByRole("link", { name: "返回运行中任务" });
    expect(recentRunLink).toHaveAttribute("href", "/runs/run-2");
    expect(recentRunLink).toHaveTextContent("Session switch benchmark");
  });
});
