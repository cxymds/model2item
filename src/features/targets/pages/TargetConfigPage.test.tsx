import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { TargetConfigPage } from "./TargetConfigPage";

const {
  createProfile,
  updateWindowBinding,
  deleteWindowBinding,
  updateProfile,
  deleteProfile,
  getProfileSecret,
} =
  vi.hoisted(() => {
  return {
    createProfile: vi.fn().mockResolvedValue({
      id: "profile-2",
      name: "Claude Sonnet",
      provider: "anthropic",
      execution_mode: "claude_cli",
      model_name: "claude-sonnet-4",
      base_url: "https://api.anthropic.example.com",
      system_prompt: "",
      temperature: null,
      max_tokens: null,
      extra_params_json: "{}",
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    }),
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
    updateProfile: vi.fn().mockResolvedValue({
      id: "profile-1",
      name: "GPT-5.4 updated",
      provider: "openai",
      execution_mode: "openai_chat",
      model_name: "gpt-5.4-mini",
      base_url: "https://api.example.com/v2",
      system_prompt: "",
      temperature: null,
      max_tokens: null,
      extra_params_json: "{}",
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-02T00:00:00Z",
    }),
    deleteProfile: vi.fn().mockResolvedValue(undefined),
    getProfileSecret: vi.fn().mockResolvedValue({
      api_key: "saved-secret",
    }),
  };
  });

vi.mock("../../../lib/tauri", () => {
  return {
    createProfile,
    updateProfile,
    getProfileSecret,
    deleteProfile,
    createWindowBinding: vi.fn(),
    updateWindowBinding,
    deleteWindowBinding,
    listProfiles: vi.fn().mockResolvedValue([
      {
        id: "profile-1",
        name: "GPT-5.4 baseline",
        provider: "openai",
        execution_mode: "openai_chat",
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

  it("prefers execution mode labels in profile list", async () => {
    renderPage();

    expect(await screen.findByText("OpenAI Chat / gpt-5.4")).toBeInTheDocument();
    expect(screen.queryByText("openai / gpt-5.4")).not.toBeInTheDocument();
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

  it("updates an existing profile", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "编辑配置 GPT-5.4 baseline" }));
    expect(getProfileSecret).toHaveBeenCalledWith("profile-1");
    expect(await screen.findByDisplayValue("saved-secret")).toBeInTheDocument();
    fireEvent.change(screen.getByDisplayValue("GPT-5.4 baseline"), {
      target: { value: "GPT-5.4 updated" },
    });
    fireEvent.change(screen.getByDisplayValue("gpt-5.4"), {
      target: { value: "gpt-5.4-mini" },
    });
    fireEvent.change(screen.getByDisplayValue("https://api.example.com/v1"), {
      target: { value: "https://api.example.com/v2" },
    });
    fireEvent.change(screen.getByDisplayValue("saved-secret"), {
      target: { value: "new-secret" },
    });
    fireEvent.change((await screen.findAllByLabelText("执行模式"))[1], {
      target: { value: "claude_cli" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    await waitFor(() => {
      expect(updateProfile).toHaveBeenCalledWith("profile-1", {
        name: "GPT-5.4 updated",
        provider: "anthropic",
        execution_mode: "claude_cli",
        model_name: "gpt-5.4-mini",
        base_url: "https://api.example.com/v2",
        api_key: "new-secret",
      });
    });
  });

  it("shows and hides the saved api key while editing", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "编辑配置 GPT-5.4 baseline" }));
    const apiKeyInput = (await screen.findByDisplayValue("saved-secret")) as HTMLInputElement;
    expect(apiKeyInput.type).toBe("password");

    fireEvent.click(screen.getByRole("button", { name: "显示 API key" }));
    expect(apiKeyInput.type).toBe("text");

    fireEvent.click(screen.getByRole("button", { name: "隐藏 API key" }));
    expect(apiKeyInput.type).toBe("password");
  });

  it("requires re-entering the api key when the saved one is missing", async () => {
    getProfileSecret.mockResolvedValueOnce({ api_key: null });
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "编辑配置 GPT-5.4 baseline" }));

    expect(await screen.findByText("当前未找到已保存的 API key，请重新输入后保存。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "保存配置" })).toBeDisabled();
  });

  it("creates a profile with selected execution_mode in payload", async () => {
    renderPage();

    expect(await screen.findByRole("option", { name: "Claude CLI" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "OpenAI Chat" })).toBeInTheDocument();

    fireEvent.change(await screen.findByPlaceholderText("GPT-5.4 基线"), {
      target: { value: "Claude Sonnet" },
    });
    fireEvent.change(screen.getByLabelText("执行模式"), {
      target: { value: "openai_chat" },
    });
    fireEvent.change(screen.getByPlaceholderText("gpt-5.4"), {
      target: { value: "gpt-5.4-mini" },
    });
    fireEvent.change(screen.getByPlaceholderText("https://api.example.com/v1"), {
      target: { value: "https://api.openai.example.com" },
    });
    fireEvent.change(screen.getByPlaceholderText("sk-..."), {
      target: { value: "secret-new" },
    });
    fireEvent.click(screen.getByRole("button", { name: "创建配置" }));

    await waitFor(() => {
      expect(createProfile).toHaveBeenCalled();
      expect(createProfile.mock.calls[0]?.[0]).toEqual({
        name: "Claude Sonnet",
        provider: "openai",
        execution_mode: "openai_chat",
        model_name: "gpt-5.4-mini",
        base_url: "https://api.openai.example.com",
        api_key: "secret-new",
      });
    });
  });

  it("keeps the existing api key when saving profile changes with an empty key field", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "编辑配置 GPT-5.4 baseline" }));
    await screen.findByDisplayValue("saved-secret");
    fireEvent.change(screen.getByDisplayValue("gpt-5.4"), {
      target: { value: "gpt-5.4-turbo" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));

    await waitFor(() => {
      expect(updateProfile).toHaveBeenCalledWith("profile-1", {
        name: "GPT-5.4 baseline",
        provider: "openai",
        execution_mode: "openai_chat",
        model_name: "gpt-5.4-turbo",
        base_url: "https://api.example.com/v1",
        api_key: "saved-secret",
      });
    });
  });

  it("deletes an unused profile", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "删除配置 GPT-5.4 baseline" }));

    await waitFor(() => {
      expect(deleteProfile.mock.calls[0]?.[0]).toBe("profile-1");
    });
  });
});
