import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";
import { RunMonitorPage } from "./RunMonitorPage";

vi.mock("../../../lib/tauri", () => {
  return {
    getComparisonRun: vi.fn().mockResolvedValue({
      id: "run-1",
      evaluation_case_id: "case-1",
      title: "Legacy parser benchmark",
      status: "running",
      prompt_snapshot: "prompt",
      context_snapshot: "{}",
      created_at: "2026-01-01T00:00:00Z",
      started_at: "2026-01-01T00:00:01Z",
      finished_at: null,
      notes: "",
    }),
    listComparisonTargets: vi.fn().mockResolvedValue([
      {
        position: 0,
        id: "target-1",
        run_id: "run-1",
        window_binding_id: "binding-1",
        profile_snapshot_json:
          '{"position":0,"display_name":"Window A","profile_id":"profile-1","provider":"openai","model_name":"gpt-5.4","base_url":"https://api.openai.com"}',
        status: "running",
        sent_at: "2026-01-01T00:00:01Z",
        first_response_at: null,
        finished_at: null,
        duration_ms: null,
        response_chars: 0,
        response_lines: 0,
        success_status: null,
        error_category: null,
        error_detail: null,
      },
    ]),
    startComparisonRun: vi.fn().mockResolvedValue(undefined),
  };
});

describe("RunMonitorPage", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("stores the current run so users can return after navigating away", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <MemoryRouter initialEntries={["/runs/run-1"]}>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/runs/:runId" element={<RunMonitorPage />} />
          </Routes>
        </QueryClientProvider>
      </MemoryRouter>,
    );

    expect(await screen.findByText("运行监控")).toBeInTheDocument();

    await waitFor(() => {
      expect(JSON.parse(window.localStorage.getItem("recent-comparison-run") ?? "null")).toEqual({
        id: "run-1",
        title: "Legacy parser benchmark",
        status: "running",
      });
    });
  });
});
