import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";
import { RunResultsPage } from "./RunResultsPage";

const { getComparisonSummaryMock, exportComparisonRunReportMock } = vi.hoisted(() => {
  return {
    getComparisonSummaryMock: vi.fn().mockResolvedValue({
      run: {
        id: "run-1",
        evaluation_case_id: "case-1",
        title: "Legacy parser benchmark",
        status: "queued",
        prompt_snapshot: "prompt",
        context_snapshot: "{}",
        created_at: "2026-01-01T00:00:00Z",
        started_at: null,
        finished_at: null,
        notes: "",
      },
      targets: [
        {
          target_id: "target-1",
          label: "openai / gpt-5.4",
          status: "queued",
          success_status: null,
          duration_ms: 1400,
          response_chars: 120,
          response_lines: 10,
          error_detail: null,
        },
      ],
      fastest_target_id: "target-1",
      longest_target_id: "target-1",
      queued_count: 1,
      summary_text: "fastest=openai / gpt-5.4; longest=openai / gpt-5.4; queued=1",
    }),
    exportComparisonRunReportMock: vi.fn().mockResolvedValue("/tmp/run-1-report.md"),
  };
});

vi.mock("../../../lib/tauri", () => {
  return {
    getComparisonSummary: getComparisonSummaryMock,
    exportComparisonRunReport: exportComparisonRunReportMock,
  };
});

describe("RunResultsPage", () => {
  beforeEach(() => {
    getComparisonSummaryMock.mockClear();
    getComparisonSummaryMock.mockResolvedValue({
      run: {
        id: "run-1",
        evaluation_case_id: "case-1",
        title: "Legacy parser benchmark",
        status: "queued",
        prompt_snapshot: "prompt",
        context_snapshot: "{}",
        created_at: "2026-01-01T00:00:00Z",
        started_at: null,
        finished_at: null,
        notes: "",
      },
      targets: [
        {
          target_id: "target-1",
          label: "openai / gpt-5.4",
          status: "queued",
          success_status: null,
          duration_ms: 1400,
          response_chars: 120,
          response_lines: 10,
          error_detail: null,
        },
      ],
      fastest_target_id: "target-1",
      longest_target_id: "target-1",
      queued_count: 1,
      summary_text: "fastest=openai / gpt-5.4; longest=openai / gpt-5.4; queued=1",
    });
    exportComparisonRunReportMock.mockClear();
  });

  it("renders summary text from comparison summary endpoint", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <MemoryRouter initialEntries={["/runs/run-1/results"]}>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/runs/:runId/results" element={<RunResultsPage />} />
          </Routes>
        </QueryClientProvider>
      </MemoryRouter>,
    );

    expect(await screen.findByText("运行结果")).toBeInTheDocument();
    expect(await screen.findByText(/最快目标：openai \/ gpt-5.4/)).toBeInTheDocument();
  });

  it("exports a markdown report for the current run", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <MemoryRouter initialEntries={["/runs/run-1/results"]}>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/runs/:runId/results" element={<RunResultsPage />} />
          </Routes>
        </QueryClientProvider>
      </MemoryRouter>,
    );

    fireEvent.click(await screen.findByRole("button", { name: "导出 Markdown 报告" }));

    await waitFor(() => {
      expect(exportComparisonRunReportMock).toHaveBeenCalledWith("run-1");
    });
    expect(await screen.findByText("报告已导出到 /tmp/run-1-report.md")).toBeInTheDocument();
  });

  it("renders failure reasons in the comparison cards when present", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    getComparisonSummaryMock.mockResolvedValueOnce({
      run: {
        id: "run-1",
        evaluation_case_id: "case-1",
        title: "Legacy parser benchmark",
        status: "failed",
        prompt_snapshot: "prompt",
        context_snapshot: "{}",
        created_at: "2026-01-01T00:00:00Z",
        started_at: "2026-01-01T00:00:01Z",
        finished_at: "2026-01-01T00:00:05Z",
        notes: "",
      },
      targets: [
        {
          target_id: "target-1",
          label: "Claude CLI / glm5.1",
          status: "failed",
          success_status: "failed",
          duration_ms: 13790,
          response_chars: 0,
          response_lines: 0,
          error_detail: "spawned CLI exited immediately: missing auth token",
        },
      ],
      fastest_target_id: "target-1",
      longest_target_id: "target-1",
      queued_count: 0,
      summary_text: "fastest=target-1; longest=target-1; queued=0",
    });

    render(
      <MemoryRouter initialEntries={["/runs/run-1/results"]}>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/runs/:runId/results" element={<RunResultsPage />} />
          </Routes>
        </QueryClientProvider>
      </MemoryRouter>,
    );

    expect(await screen.findByText(/失败原因：spawned CLI exited immediately: missing auth token/)).toBeInTheDocument();
  });
});
