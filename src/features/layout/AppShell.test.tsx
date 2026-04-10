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

    expect(screen.getByRole("link", { name: "新建任务" })).toHaveAttribute("href", "/runs/new");
    expect(screen.getByText("历史运行")).toBeInTheDocument();
    expect(screen.getByText("案例库")).toBeInTheDocument();
    expect(screen.getByText("目标配置")).toBeInTheDocument();
    expect(screen.getByText("设置")).toBeInTheDocument();
  });

  it("renders the current active run entry instead of the new run entry", () => {
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

    expect(screen.queryByRole("link", { name: "新建任务" })).not.toBeInTheDocument();

    const recentRunLink = screen.getByRole("link", { name: "当前进行中的任务" });
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

    expect(screen.getByRole("link", { name: "新建任务" })).toHaveAttribute("href", "/runs/new");

    act(() => {
      saveRecentRun({
        id: "run-2",
        title: "Session switch benchmark",
        status: "running",
      });
    });

    expect(screen.queryByRole("link", { name: "新建任务" })).not.toBeInTheDocument();

    const recentRunLink = screen.getByRole("link", { name: "当前进行中的任务" });
    expect(recentRunLink).toHaveAttribute("href", "/runs/run-2");
    expect(recentRunLink).toHaveTextContent("Session switch benchmark");
  });
});
