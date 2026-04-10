import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";
import { RunResultsPage } from "./RunResultsPage";

vi.mock("../../../lib/tauri", () => {
  return {
    getComparisonSummary: vi.fn().mockResolvedValue({
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
        },
      ],
      fastest_target_id: "target-1",
      longest_target_id: "target-1",
      queued_count: 1,
      summary_text: "fastest=openai / gpt-5.4; longest=openai / gpt-5.4; queued=1",
    }),
  };
});

describe("RunResultsPage", () => {
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

    expect(await screen.findByText(/fastest=openai/i)).toBeInTheDocument();
  });
});
