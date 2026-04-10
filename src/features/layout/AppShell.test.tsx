import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { AppShell } from "./AppShell";

describe("AppShell", () => {
  it("renders the primary navigation items", () => {
    render(
      <MemoryRouter>
        <AppShell>
          <div>content</div>
        </AppShell>
      </MemoryRouter>,
    );

    expect(screen.getByText("运行任务")).toBeInTheDocument();
    expect(screen.getByText("案例库")).toBeInTheDocument();
    expect(screen.getByText("目标配置")).toBeInTheDocument();
    expect(screen.getByText("设置")).toBeInTheDocument();
  });
});
