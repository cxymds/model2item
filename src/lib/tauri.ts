import { core } from "@tauri-apps/api";
import type {
  ComparisonRunResponse,
  ComparisonSummaryResponse,
  ComparisonTargetResponse,
  CreateComparisonRunInput,
  CreateEvaluationCaseInput,
  CreateProfileInput,
  CreateWindowBindingInput,
  EvaluationCaseResponse,
  ItermSessionResponse,
  ProfileResponse,
  WindowBindingResponse,
} from "../types/api";

export function listProfiles() {
  return core.invoke<ProfileResponse[]>("list_profiles");
}

export function createProfile(input: CreateProfileInput) {
  return core.invoke<ProfileResponse>("create_profile", { input });
}

export function listWindowBindings() {
  return core.invoke<WindowBindingResponse[]>("list_window_bindings");
}

export function refreshWindowBindingPresence() {
  return core.invoke<WindowBindingResponse[]>("refresh_window_binding_presence");
}

export function listItermSessions() {
  return core.invoke<ItermSessionResponse[]>("list_iterm_sessions");
}

export function createWindowBinding(input: CreateWindowBindingInput) {
  return core.invoke<WindowBindingResponse>("create_window_binding", { input });
}

export function listEvaluationCases() {
  return core.invoke<EvaluationCaseResponse[]>("list_evaluation_cases");
}

export function createEvaluationCase(input: CreateEvaluationCaseInput) {
  return core.invoke<EvaluationCaseResponse>("create_evaluation_case", { input });
}

export function createComparisonRun(input: CreateComparisonRunInput) {
  return core.invoke<ComparisonRunResponse>("create_comparison_run", { input });
}

export function startComparisonRun(runId: string) {
  return core.invoke<void>("start_comparison_run", { runId });
}

export function getComparisonRun(runId: string) {
  return core.invoke<ComparisonRunResponse>("get_comparison_run", { runId });
}

export function listComparisonTargets(runId: string) {
  return core.invoke<ComparisonTargetResponse[]>("list_comparison_targets", { runId });
}

export function getComparisonSummary(runId: string) {
  return core.invoke<ComparisonSummaryResponse>("get_comparison_summary", { runId });
}
