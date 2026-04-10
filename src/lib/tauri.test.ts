import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getComparisonRun,
  getComparisonSummary,
  listComparisonRuns,
  listComparisonTargets,
  startComparisonRun,
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
    await getComparisonRun("run-123");
    await listComparisonTargets("run-123");
    await getComparisonSummary("run-123");

    expect(invokeMock).toHaveBeenNthCalledWith(1, "list_comparison_runs");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "start_comparison_run", { runId: "run-123" });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "get_comparison_run", { runId: "run-123" });
    expect(invokeMock).toHaveBeenNthCalledWith(4, "list_comparison_targets", { runId: "run-123" });
    expect(invokeMock).toHaveBeenNthCalledWith(5, "get_comparison_summary", { runId: "run-123" });
  });
});
