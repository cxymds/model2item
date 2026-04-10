import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";
import { RunHistoryPage } from "./RunHistoryPage";

vi.mock("../../../lib/tauri", () => {
  return {
    listComparisonRuns: vi.fn().mockResolvedValue([
      {
        id: "run-3",
        evaluation_case_id: "case-1",
        title: "Streaming parser benchmark",
        status: "running",
        prompt_snapshot: "prompt",
        context_snapshot: "{}",
        created_at: "2026-01-03T00:00:00Z",
        started_at: "2026-01-03T00:00:05Z",
        finished_at: null,
        notes: "",
      },
      {
        id: "run-2",
        evaluation_case_id: "case-1",
        title: "Legacy parser benchmark",
        status: "done",
        prompt_snapshot: "prompt",
        context_snapshot: "{}",
        created_at: "2026-01-02T00:00:00Z",
        started_at: "2026-01-02T00:00:05Z",
        finished_at: "2026-01-02T00:02:00Z",
        notes: "",
      },
      {
        id: "run-1",
        evaluation_case_id: "case-1",
        title: "Failure repro benchmark",
        status: "failed",
        prompt_snapshot: "prompt",
        context_snapshot: "{}",
        created_at: "2026-01-01T00:00:00Z",
        started_at: "2026-01-01T00:00:05Z",
        finished_at: "2026-01-01T00:01:00Z",
        notes: "",
      },
    ]),
  };
});

describe("RunHistoryPage", () => {
  it("renders active runs separately and links historical results", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <MemoryRouter initialEntries={["/runs/history"]}>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/runs/history" element={<RunHistoryPage />} />
          </Routes>
        </QueryClientProvider>
      </MemoryRouter>,
    );

    expect(await screen.findByText("历史运行")).toBeInTheDocument();
    expect(await screen.findAllByText("Streaming parser benchmark")).toHaveLength(2);
    expect(screen.getByText("当前运行")).toBeInTheDocument();
    expect(screen.getAllByRole("link", { name: "继续查看 Streaming parser benchmark" })).toHaveLength(2);
    screen
      .getAllByRole("link", { name: "继续查看 Streaming parser benchmark" })
      .forEach((link) => {
        expect(link).toHaveAttribute("href", "/runs/run-3");
      });

    expect(screen.getByText("最近结果")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "查看结果 Legacy parser benchmark" })).toHaveAttribute(
      "href",
      "/runs/run-2/results",
    );
    expect(screen.getByRole("link", { name: "查看结果 Failure repro benchmark" })).toHaveAttribute(
      "href",
      "/runs/run-1/results",
    );
  });
});
