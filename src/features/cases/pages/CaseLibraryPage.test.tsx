import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { CaseLibraryPage } from "./CaseLibraryPage";

const { updateEvaluationCase, deleteEvaluationCase } = vi.hoisted(() => {
  return {
    updateEvaluationCase: vi.fn().mockResolvedValue({
      id: "case-1",
      title: "Updated Case",
      prompt: "Updated prompt",
      context_payload: "{}",
      expected_checkpoints_json: "[]",
      validation_rules_json: "{}",
      notes: "Focus parser",
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-02T00:00:00Z",
    }),
    deleteEvaluationCase: vi
      .fn()
      .mockRejectedValue(new Error("已被运行任务引用，暂时不能删除")),
  };
});

vi.mock("../../../lib/tauri", () => {
  return {
    createEvaluationCase: vi.fn(),
    updateEvaluationCase,
    deleteEvaluationCase,
    listEvaluationCases: vi.fn().mockResolvedValue([
      {
        id: "case-1",
        title: "Legacy parser walkthrough",
        prompt: "Explain parser",
        context_payload: "{}",
        expected_checkpoints_json: "[]",
        validation_rules_json: "{}",
        notes: "Focus parser",
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
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
      <CaseLibraryPage />
    </QueryClientProvider>,
  );
}

describe("CaseLibraryPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("updates an existing evaluation case inline", async () => {
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "编辑 Legacy parser walkthrough" }));
    fireEvent.change(screen.getByDisplayValue("Legacy parser walkthrough"), {
      target: { value: "Updated Case" },
    });
    fireEvent.change(screen.getByDisplayValue("Explain parser"), {
      target: { value: "Updated prompt" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存修改" }));

    await waitFor(() => {
      expect(updateEvaluationCase).toHaveBeenCalledWith("case-1", {
        title: "Updated Case",
        prompt: "Updated prompt",
        context_payload: "{}",
        notes: "Focus parser",
      });
    });
  });

  it("shows an error when deleting a referenced evaluation case", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    renderPage();

    fireEvent.click(await screen.findByRole("button", { name: "删除 Legacy parser walkthrough" }));

    await screen.findByText("已被运行任务引用，暂时不能删除");
    expect(deleteEvaluationCase).toHaveBeenCalled();
    expect(deleteEvaluationCase.mock.calls[0]?.[0]).toBe("case-1");
  });
});
