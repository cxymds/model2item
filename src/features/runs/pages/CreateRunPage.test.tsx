import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { vi } from "vitest";
import { CreateRunPage } from "./CreateRunPage";

vi.mock("../../../lib/tauri", () => {
  return {
    listEvaluationCases: vi.fn().mockResolvedValue([
      {
        id: "case-1",
        title: "Legacy parser walkthrough",
        prompt: "Explain the parser flow",
        context_payload: "{}",
        expected_checkpoints_json: "[]",
        validation_rules_json: "{}",
        notes: "",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]),
    listWindowBindings: vi.fn().mockResolvedValue([
      {
        id: "binding-1",
        iterm_session_id: "session-1",
        display_name: "Window A",
        profile_id: "profile-1",
        enabled: 1,
        last_seen_at: null,
        metadata_json: "{}",
      },
      {
        id: "binding-2",
        iterm_session_id: "session-2",
        display_name: "Window B",
        profile_id: "profile-1",
        enabled: 1,
        last_seen_at: null,
        metadata_json: "{}",
      },
    ]),
    listItermSessions: vi.fn().mockResolvedValue([
      {
        session_id: "session-1",
        window_id: "window-1",
        window_title: "Project A",
        tab_id: "tab-1",
        tab_title: "Compare",
        session_title: "Pane 1",
      },
    ]),
    createComparisonRun: vi.fn().mockResolvedValue({
      id: "run-1",
      evaluation_case_id: "case-1",
      title: "Legacy parser run",
      status: "queued",
      prompt_snapshot: "prompt",
      context_snapshot: "{}",
      created_at: "2026-01-01T00:00:00Z",
      started_at: null,
      finished_at: null,
      notes: "",
    }),
    startComparisonRun: vi.fn().mockResolvedValue(undefined),
  };
});

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  render(
    <MemoryRouter>
      <QueryClientProvider client={queryClient}>
        <CreateRunPage />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

describe("CreateRunPage", () => {
  it("renders online bindings and disables offline ones", async () => {
    renderPage();

    expect(await screen.findByLabelText("任务标题")).toBeInTheDocument();
    expect(await screen.findByRole("option", { name: "Legacy parser walkthrough" })).toBeInTheDocument();
    const onlineOption = await screen.findByRole("option", { name: /Window A .*在线/ });
    const offlineOption = await screen.findByRole("option", { name: /Window B .*离线/ });

    expect(onlineOption).not.toBeDisabled();
    expect(offlineOption).toBeDisabled();
    expect(screen.getByText(/离线窗口会被禁用/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "开始运行" })).toBeInTheDocument();
  });
});
