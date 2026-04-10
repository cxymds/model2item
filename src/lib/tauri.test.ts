import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  deleteProfile,
  deleteWindowBinding,
  listProfiles,
  getComparisonRun,
  getComparisonSummary,
  listComparisonRuns,
  listComparisonTargets,
  listTargetMessages,
  sendComparisonRunMessage,
  startComparisonRun,
  updateProfile,
  updateWindowBinding,
} from "./tauri";

const { invokeMock } = vi.hoisted(() => {
  return {
    invokeMock: vi.fn(),
  };
});

vi.mock("@tauri-apps/api", () => {
  return {
    core: {
      invoke: invokeMock,
    },
  };
});

describe("comparison run tauri bindings", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it("passes runId to comparison run commands", async () => {
    await listComparisonRuns();
    await startComparisonRun("run-123");
    await sendComparisonRunMessage("run-123", "follow up");
    await getComparisonRun("run-123");
    await listComparisonTargets("run-123");
    await listTargetMessages("target-456");
    await getComparisonSummary("run-123");

    expect(invokeMock).toHaveBeenNthCalledWith(1, "list_comparison_runs");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "start_comparison_run", { runId: "run-123" });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "send_comparison_run_message", {
      runId: "run-123",
      prompt: "follow up",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(4, "get_comparison_run", { runId: "run-123" });
    expect(invokeMock).toHaveBeenNthCalledWith(5, "list_comparison_targets", { runId: "run-123" });
    expect(invokeMock).toHaveBeenNthCalledWith(6, "list_target_messages", { targetId: "target-456" });
    expect(invokeMock).toHaveBeenNthCalledWith(7, "get_comparison_summary", { runId: "run-123" });
  });
});

describe("target config tauri bindings", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it("passes ids and payloads to profile and binding commands", async () => {
    await listProfiles();
    await updateProfile("profile-1", {
      name: "Profile",
      provider: "openai",
      model_name: "gpt-5.4",
      base_url: "https://api.example.com/v1",
      api_key: "secret",
    });
    await deleteProfile("profile-1");
    await updateWindowBinding("binding-1", {
      display_name: "Window A",
      iterm_session_id: "session-1",
      profile_id: "profile-1",
    });
    await deleteWindowBinding("binding-1");

    expect(invokeMock).toHaveBeenNthCalledWith(1, "list_profiles");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "update_profile", {
      id: "profile-1",
      input: {
        name: "Profile",
        provider: "openai",
        model_name: "gpt-5.4",
        base_url: "https://api.example.com/v1",
        api_key: "secret",
      },
    });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "delete_profile", { id: "profile-1" });
    expect(invokeMock).toHaveBeenNthCalledWith(4, "update_window_binding", {
      id: "binding-1",
      input: {
        display_name: "Window A",
        iterm_session_id: "session-1",
        profile_id: "profile-1",
      },
    });
    expect(invokeMock).toHaveBeenNthCalledWith(5, "delete_window_binding", { id: "binding-1" });
  });
});
