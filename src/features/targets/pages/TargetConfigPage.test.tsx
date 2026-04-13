import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { TargetConfigPage } from "./TargetConfigPage";

const {
  createCustomProvider,
  createWindowBinding,
  deleteCustomProvider,
  updateWindowBinding,
  deleteWindowBinding,
} = vi.hoisted(() => {
  return {
    createCustomProvider: vi.fn().mockResolvedValue({
      id: "provider-2",
      name: "GLM via Claude CLI",
      provider_key: "glm",
      client_type: "claude_cli",
      base_url: "",
      default_model: "glm-5.1",
      extra_params_json: "{}",
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    }),
    createWindowBinding: vi.fn().mockResolvedValue({
      id: "binding-3",
      iterm_session_id: "session-1",
      display_name: "Window C",
      profile_id: "__provider_binding_profile__",
      custom_provider_id: "provider-2",
      enabled: 1,
      last_seen_at: "2026-01-01T00:00:00Z",
      metadata_json: "{}",
    }),
    deleteCustomProvider: vi.fn().mockResolvedValue(undefined),
    updateWindowBinding: vi.fn().mockResolvedValue({
      id: "binding-1",
      iterm_session_id: "session-1b",
      display_name: "Window A Updated",
      profile_id: "__provider_binding_profile__",
      custom_provider_id: "provider-2",
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
    createCustomProvider,
    listCustomProviders: vi.fn().mockResolvedValue([
      {
        id: "provider-1",
        name: "GLM baseline",
        provider_key: "glm",
        client_type: "claude_cli",
        base_url: "https://gateway.example.com/v1",
        default_model: "glm-4.5",
        extra_params_json: "{}",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      },
      {
        id: "provider-2",
        name: "GLM via Claude CLI",
        provider_key: "glm",
        client_type: "claude_cli",
        base_url: "",
        default_model: "glm-5.1",
        extra_params_json: "{}",
        created_at: "2026-01-02T00:00:00Z",
        updated_at: "2026-01-02T00:00:00Z",
      },
    ]),
    createWindowBinding,
    deleteCustomProvider,
    updateWindowBinding,
    deleteWindowBinding,
    listWindowBindings: vi.fn().mockResolvedValue([
      {
        id: "binding-1",
        iterm_session_id: "session-1",
        display_name: "Window A",
        profile_id: "__provider_binding_profile__",
        custom_provider_id: "provider-1",
        enabled: 1,
        last_seen_at: "2026-01-01T00:00:00Z",
        metadata_json: "{}",
      },
      {
        id: "binding-2",
        iterm_session_id: "session-offline",
        display_name: "Window B",
        profile_id: "__provider_binding_profile__",
        custom_provider_id: "provider-1",
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
    createProfile: vi.fn(),
    updateProfile: vi.fn(),
    getProfileSecret: vi.fn(),
    deleteProfile: vi.fn(),
    listProfiles: vi.fn().mockResolvedValue([]),
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

  it("creates a provider and binds a window to that provider", async () => {
    renderPage();

    fireEvent.change(await screen.findByLabelText("名称"), {
      target: { value: "GLM via Claude CLI" },
    });
    fireEvent.change(screen.getByLabelText("上游标识"), {
      target: { value: "glm" },
    });
    fireEvent.change(screen.getByLabelText("客户端类型"), {
      target: { value: "claude_cli" },
    });
    fireEvent.change(screen.getByLabelText("默认模型"), {
      target: { value: "glm-5.1" },
    });

    fireEvent.click(screen.getByRole("button", { name: "保存 Provider" }));

    await waitFor(() => {
      expect(createCustomProvider).toHaveBeenCalledWith({
        name: "GLM via Claude CLI",
        provider_key: "glm",
        client_type: "claude_cli",
        default_model: "glm-5.1",
        base_url: "",
        api_key: "",
        extra_params_json: "{}",
      });
    });

    fireEvent.change(screen.getByLabelText("已发现会话"), {
      target: { value: "session-1" },
    });
    fireEvent.change(screen.getByLabelText("显示名称"), {
      target: { value: "Window C" },
    });
    fireEvent.change(screen.getByLabelText("绑定 Provider"), {
      target: { value: "provider-2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "创建绑定" }));

    await waitFor(() => {
      expect(createWindowBinding).toHaveBeenCalledWith({
        iterm_session_id: "session-1",
        display_name: "Window C",
        profile_id: "",
        custom_provider_id: "provider-2",
      });
    });
  });

  it("shows provider-first details in the binding cards", async () => {
    renderPage();

    expect(await screen.findByText("Provider 配置")).toBeInTheDocument();
    expect((await screen.findAllByText(/claude_cli\s*\/\s*glm\s*\/\s*glm-4.5/)).length).toBeGreaterThan(0);
    expect(screen.getByText("连接状态：在线")).toBeInTheDocument();
    expect(screen.getByText("连接状态：离线")).toBeInTheDocument();
  });

  it("updates an existing window binding with a new provider", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "编辑 Window A" }));
    fireEvent.change(screen.getByDisplayValue("Window A"), {
      target: { value: "Window A Updated" },
    });
    fireEvent.change(screen.getByDisplayValue("session-1"), {
      target: { value: "session-1b" },
    });
    fireEvent.change((await screen.findAllByLabelText("绑定 Provider"))[1], {
      target: { value: "provider-2" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存绑定" }));

    await waitFor(() => {
      expect(updateWindowBinding).toHaveBeenCalledWith("binding-1", {
        display_name: "Window A Updated",
        iterm_session_id: "session-1b",
        profile_id: "",
        custom_provider_id: "provider-2",
      });
    });
  });

  it("requires inline confirmation before deleting a referenced binding", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "删除 Window A" }));
    expect(await screen.findByRole("button", { name: "确认删除 Window A" })).toBeInTheDocument();
    expect(deleteWindowBinding).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "确认删除 Window A" }));

    await screen.findByText("已被运行任务引用，暂时不能删除");
    expect(deleteWindowBinding.mock.calls[0]?.[0]).toBe("binding-1");
  });

  it("deletes a provider after inline confirmation", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "删除 Provider GLM baseline" }));
    expect(
      await screen.findByRole("button", { name: "确认删除 Provider GLM baseline" }),
    ).toBeInTheDocument();
    expect(deleteCustomProvider).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "确认删除 Provider GLM baseline" }));

    await waitFor(() => {
      expect(deleteCustomProvider).toHaveBeenCalledWith("provider-1");
    });
  });
});
