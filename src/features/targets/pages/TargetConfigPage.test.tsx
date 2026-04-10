import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { TargetConfigPage } from "./TargetConfigPage";

const { updateWindowBinding, deleteWindowBinding } = vi.hoisted(() => {
  return {
    updateWindowBinding: vi.fn().mockResolvedValue({
      id: "binding-1",
      iterm_session_id: "session-1b",
      display_name: "Window A Updated",
      profile_id: "profile-1",
      enabled: 1,
      last_seen_at: "2026-01-01T00:00:00Z",
      metadata_json: "{}",
    }),
    deleteWindowBinding: vi
      .fn()
      .mockRejectedValue(new Error("已被运行任务引用，暂时不能删除")),
  };
});

vi.mock("../../../lib/tauri", () => {
  return {
    createProfile: vi.fn(),
    createWindowBinding: vi.fn(),
    updateWindowBinding,
    deleteWindowBinding,
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
    listWindowBindings: vi.fn().mockResolvedValue([
      {
        id: "binding-1",
        iterm_session_id: "session-1",
        display_name: "Window A",
        profile_id: "profile-1",
        enabled: 1,
        last_seen_at: "2026-01-01T00:00:00Z",
        metadata_json: "{}",
      },
      {
        id: "binding-2",
        iterm_session_id: "session-offline",
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
        tab_title: "GPT Compare",
        session_title: "Pane 1",
      },
    ]),
    refreshWindowBindingPresence: vi.fn().mockResolvedValue([]),
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
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows discovered iTerm sessions only in the binding form selector", async () => {
    renderPage();

    expect(await screen.findByText("已发现会话")).toBeInTheDocument();
    expect(screen.getByText("系统会自动清理已关闭且未被运行任务引用的绑定。")).toBeInTheDocument();
    expect(await screen.findByRole("option", { name: /Project A \/ GPT Compare \/ Pane 1/ })).toBeInTheDocument();
    expect(screen.queryByText("Project A")).not.toBeInTheDocument();
    expect(screen.getByText("连接状态：在线")).toBeInTheDocument();
    expect(screen.getByText("连接状态：离线")).toBeInTheDocument();
  });

  it("updates an existing window binding", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "编辑 Window A" }));
    fireEvent.change(screen.getByDisplayValue("Window A"), {
      target: { value: "Window A Updated" },
    });
    fireEvent.change(screen.getByDisplayValue("session-1"), {
      target: { value: "session-1b" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存绑定" }));

    await waitFor(() => {
      expect(updateWindowBinding).toHaveBeenCalledWith("binding-1", {
        display_name: "Window A Updated",
        iterm_session_id: "session-1b",
        profile_id: "profile-1",
      });
    });
  });

  it("shows an error when deleting a referenced binding", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "删除 Window A" }));

    await screen.findByText("已被运行任务引用，暂时不能删除");
    expect(deleteWindowBinding).toHaveBeenCalled();
    expect(deleteWindowBinding.mock.calls[0]?.[0]).toBe("binding-1");
  });
});
