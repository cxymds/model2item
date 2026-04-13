import { describe, expect, it } from "vitest";
import type { ComparisonTargetResponse } from "../../../types/api";
import { buildRunTargetViewModel, mapRunTargetStatus } from "./runViewModel";

describe("runViewModel", () => {
  it("derives consistent label and summary from profile snapshot json", () => {
    const target: ComparisonTargetResponse = {
      position: 0,
      id: "target-1",
      run_id: "run-1",
      window_binding_id: "binding-1",
      profile_snapshot_json:
        '{"profile_id":"profile-1","execution_mode":"openai_chat","provider":"openai","model_name":"gpt-5.4","base_url":"https://api.example.com/v1"}',
      status: "queued",
      sent_at: null,
      first_response_at: null,
      finished_at: null,
      duration_ms: null,
      response_chars: 12,
      response_lines: 2,
      success_status: null,
      error_category: null,
      error_detail: null,
      latest_message_role: null,
      latest_message_content: null,
    };

    const vm = buildRunTargetViewModel(target);
    expect(vm.label).toBe("OpenAI Chat / gpt-5.4");
    expect(vm.summary).toContain("状态：排队中");
    expect(vm.status).toBe("queued");
  });

  it("maps unknown statuses to queued and failed status correctly", () => {
    expect(mapRunTargetStatus("error")).toBe("failed");
    expect(mapRunTargetStatus("whatever")).toBe("queued");
  });

  it("falls back to a stable target label when snapshot shape is invalid", () => {
    const target: ComparisonTargetResponse = {
      position: 0,
      id: "target-xyz-1234",
      run_id: "run-1",
      window_binding_id: "binding-1",
      profile_snapshot_json: '{"provider":"openai"}',
      status: "queued",
      sent_at: null,
      first_response_at: null,
      finished_at: null,
      duration_ms: null,
      response_chars: 0,
      response_lines: 0,
      success_status: null,
      error_category: null,
      error_detail: null,
      latest_message_role: null,
      latest_message_content: null,
    };

    const vm = buildRunTargetViewModel(target);
    expect(vm.label).toBe("目标 target...1234");
  });

  it("includes failure details in the target summary when available", () => {
    const target: ComparisonTargetResponse = {
      position: 0,
      id: "target-err-1",
      run_id: "run-1",
      window_binding_id: "binding-1",
      profile_snapshot_json:
        '{"profile_id":"profile-1","execution_mode":"claude_cli","provider":"anthropic","model_name":"glm5.1","base_url":"https://api.example.com"}',
      status: "failed",
      sent_at: null,
      first_response_at: null,
      finished_at: null,
      duration_ms: 13790,
      response_chars: 0,
      response_lines: 0,
      success_status: "failed",
      error_category: "adapter_error",
      error_detail: "spawned CLI exited immediately: missing auth token",
      latest_message_role: "system",
      latest_message_content: "spawned CLI exited immediately: missing auth token",
    };

    const vm = buildRunTargetViewModel(target);
    expect(vm.summary).toContain("失败原因：spawned CLI exited immediately: missing auth token");
  });
});
