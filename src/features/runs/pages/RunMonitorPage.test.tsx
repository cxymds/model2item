import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { vi } from "vitest";
import { RunMonitorPage } from "./RunMonitorPage";

const { getComparisonRunMock, listComparisonTargetsMock, sendComparisonRunMessageMock } = vi.hoisted(() => {
  return {
    getComparisonRunMock: vi.fn().mockResolvedValue({
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
    listComparisonTargetsMock: vi.fn().mockResolvedValue([
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
        latest_message_role: "assistant",
        latest_message_content: "最新窗口输出：已经进入交互式 Claude 会话。",
      },
    ]),
    sendComparisonRunMessageMock: vi.fn().mockResolvedValue(undefined),
  };
});

const { listTargetMessagesMock } = vi.hoisted(() => {
  return {
    listTargetMessagesMock: vi.fn().mockResolvedValue([
      {
        id: "msg-1",
        comparison_target_id: "target-1",
        role: "user",
        content: "先总结整体结构",
        message_type: "prompt",
        created_at: "2026-01-01T00:00:01Z",
      },
      {
        id: "msg-2",
        comparison_target_id: "target-1",
        role: "assistant",
        content: "这是完整会话日志里的第一段模型回答。",
        message_type: "response",
        created_at: "2026-01-01T00:00:03Z",
      },
      {
        id: "msg-3",
        comparison_target_id: "target-1",
        role: "assistant",
        content: "这是第二段模型回答，会和上一段合并显示。",
        message_type: "response",
        created_at: "2026-01-01T00:00:04Z",
      },
    ]),
  };
});

vi.mock("../../../lib/tauri", () => {
  return {
    getComparisonRun: getComparisonRunMock,
    listComparisonTargets: listComparisonTargetsMock,
    startComparisonRun: vi.fn().mockResolvedValue(undefined),
    sendComparisonRunMessage: sendComparisonRunMessageMock,
    listTargetMessages: listTargetMessagesMock,
  };
});

describe("RunMonitorPage", () => {
  beforeEach(() => {
    window.localStorage.clear();
    getComparisonRunMock.mockClear();
    getComparisonRunMock.mockResolvedValue({
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
    });
    listComparisonTargetsMock.mockClear();
    listComparisonTargetsMock.mockResolvedValue([
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
        latest_message_role: "assistant",
        latest_message_content: "最新窗口输出：已经进入交互式 Claude 会话。",
      },
    ]);
    sendComparisonRunMessageMock.mockClear();
    listTargetMessagesMock.mockClear();
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

  it("renders the latest target output preview", async () => {
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

    expect(await screen.findByText("最新输出")).toBeInTheDocument();
    expect(screen.getByText("最新窗口输出：已经进入交互式 Claude 会话。")).toBeInTheDocument();
  });

  it("broadcasts follow-up prompts to the active run", async () => {
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

    fireEvent.change(
      await screen.findByPlaceholderText("继续分析异常分支，并比较每个窗口的解释差异"),
      {
        target: { value: "继续比较边界条件" },
      },
    );
    fireEvent.click(screen.getByRole("button", { name: "发送到所有窗口" }));

    await waitFor(() => {
      expect(sendComparisonRunMessageMock).toHaveBeenCalledWith("run-1", "继续比较边界条件");
    });
  });

  it("loads the full conversation drawer for a target", async () => {
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

    expect(await screen.findByRole("button", { name: "展开完整日志 (3 条未读)" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "展开完整日志 (3 条未读)" }));

    await waitFor(() => {
      expect(listTargetMessagesMock).toHaveBeenCalledWith("target-1");
    });
    expect(
      await screen.findByText((content) =>
        content.includes("这是完整会话日志里的第一段模型回答。") &&
        content.includes("这是第二段模型回答，会和上一段合并显示。"),
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("先总结整体结构")).toBeInTheDocument();
    expect(screen.getAllByText("模型输出")).toHaveLength(1);
    expect(
      screen
        .getByText((content) =>
          content.includes("这是完整会话日志里的第一段模型回答。") &&
          content.includes("这是第二段模型回答，会和上一段合并显示。"),
        )
        .closest("article"),
    ).toHaveAttribute(
      "data-is-new",
      "true",
    );
    expect(screen.getByRole("button", { name: "收起完整日志" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "冻结跟随" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "冻结跟随" }));
    expect(screen.getByRole("button", { name: "恢复跟随" })).toBeInTheDocument();
  });

  it("shows the recorded failure reason for failed targets", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    getComparisonRunMock.mockResolvedValueOnce({
      id: "run-1",
      evaluation_case_id: "case-1",
      title: "Legacy parser benchmark",
      status: "failed",
      prompt_snapshot: "prompt",
      context_snapshot: "{}",
      created_at: "2026-01-01T00:00:00Z",
      started_at: "2026-01-01T00:00:01Z",
      finished_at: "2026-01-01T00:00:05Z",
      notes: "",
    });
    listComparisonTargetsMock.mockResolvedValueOnce([
      {
        position: 0,
        id: "target-1",
        run_id: "run-1",
        window_binding_id: "binding-1",
        profile_snapshot_json:
          '{"position":0,"display_name":"Window A","profile_id":"profile-1","execution_mode":"claude_cli","provider":"anthropic","model_name":"glm5.1","base_url":"https://api.example.com"}',
        status: "failed",
        sent_at: "2026-01-01T00:00:01Z",
        first_response_at: null,
        finished_at: "2026-01-01T00:00:05Z",
        duration_ms: 4000,
        response_chars: 0,
        response_lines: 0,
        success_status: "failed",
        error_category: "adapter_error",
        error_detail: "spawned CLI exited immediately: missing auth token",
        latest_message_role: "system",
        latest_message_content: "spawned CLI exited immediately: missing auth token",
      },
    ]);

    render(
      <MemoryRouter initialEntries={["/runs/run-1"]}>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/runs/:runId" element={<RunMonitorPage />} />
          </Routes>
        </QueryClientProvider>
      </MemoryRouter>,
    );

    expect(await screen.findByText("失败原因")).toBeInTheDocument();
    expect(screen.getAllByText("spawned CLI exited immediately: missing auth token").length).toBeGreaterThan(0);
  });

  it("clears the recent run cache when the requested run no longer exists", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    window.localStorage.setItem(
      "recent-comparison-run",
      JSON.stringify({
        id: "run-1",
        title: "Stale run",
        status: "running",
      }),
    );
    getComparisonRunMock.mockRejectedValueOnce(
      new Error("missing dependency: comparison run run-1 not found"),
    );
    listComparisonTargetsMock.mockResolvedValueOnce([]);

    render(
      <MemoryRouter initialEntries={["/runs/run-1"]}>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/runs/:runId" element={<RunMonitorPage />} />
          </Routes>
        </QueryClientProvider>
      </MemoryRouter>,
    );

    expect(
      await screen.findByText("加载运行任务失败。missing dependency: comparison run run-1 not found"),
    ).toBeInTheDocument();

    await waitFor(() => {
      expect(window.localStorage.getItem("recent-comparison-run")).toBeNull();
    });
  });
});
