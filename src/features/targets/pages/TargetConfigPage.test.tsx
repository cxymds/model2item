import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { TargetConfigPage } from "./TargetConfigPage";

vi.mock("../../../lib/tauri", () => {
  return {
    createProfile: vi.fn(),
    createWindowBinding: vi.fn(),
    listProfiles: vi.fn().mockResolvedValue([
      {
        id: "profile-1",
        name: "GPT-5.4 baseline",
        provider: "openai",
        model_name: "gpt-5.4",
        base_url: "https://api.example.com/v1",
        system_prompt: "",
        temperature: null,
        max_tokens: null,
        extra_params_json: "{}",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
    ]),
    listWindowBindings: vi.fn().mockResolvedValue([]),
    listItermSessions: vi.fn().mockResolvedValue([
      {
        session_id: "session-1",
        window_id: "window-1",
        window_title: "Project A",
        tab_id: "tab-1",
        tab_title: "GPT Compare",
        session_title: "Pane 1",
      },
    ]),
  };
});

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  render(
    <QueryClientProvider client={queryClient}>
      <TargetConfigPage />
    </QueryClientProvider>,
  );
}

describe("TargetConfigPage", () => {
  it("renders discovered iTerm sessions in the binding form", async () => {
    renderPage();

    expect((await screen.findAllByText("Discovered sessions")).length).toBeGreaterThan(0);
    expect(await screen.findByRole("option", { name: /Project A \/ GPT Compare \/ Pane 1/ })).toBeInTheDocument();
    expect(screen.getByText("Project A")).toBeInTheDocument();
  });
});
