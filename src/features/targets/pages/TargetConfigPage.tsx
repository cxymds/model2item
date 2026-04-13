import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  createProfile,
  createWindowBinding,
  deleteProfile,
  deleteWindowBinding,
  getProfileSecret,
  listItermSessions,
  listProfiles,
  listWindowBindings,
  refreshWindowBindingPresence,
  updateProfile,
  updateWindowBinding,
} from "../../../lib/tauri";
import { formatDateTime } from "../../../lib/formatters";
import { ProfileForm } from "../components/ProfileForm";
import { WindowBindingList } from "../components/WindowBindingList";
import {
  EXECUTION_MODE_OPTIONS,
  PROVIDER_BY_EXECUTION_MODE,
  getExecutionModeLabel,
  normalizeExecutionMode,
  type UpdateProfileInput,
} from "../../../types/api";

function getErrorMessage(error: unknown) {
  const message = String(error);
  return message.startsWith("Error: ") ? message.slice(7) : message;
}

export function TargetConfigPage() {
  const queryClient = useQueryClient();
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [editingProfileSecretMissing, setEditingProfileSecretMissing] = useState(false);
  const [editingProfileSecretVisible, setEditingProfileSecretVisible] = useState(false);
  const [editingProfileSecretError, setEditingProfileSecretError] = useState<string | null>(null);
  const [isLoadingEditingProfileSecret, setIsLoadingEditingProfileSecret] = useState(false);
  const [pendingDeleteProfileId, setPendingDeleteProfileId] = useState<string | null>(null);
  const [editingProfileForm, setEditingProfileForm] = useState<UpdateProfileInput>({
    name: "",
    provider: "anthropic",
    execution_mode: "claude_cli",
    model_name: "",
    base_url: "",
    api_key: "",
  });
  const profilesQuery = useQuery({
    queryKey: ["profiles"],
    queryFn: listProfiles,
  });
  const bindingsQuery = useQuery({
    queryKey: ["window-bindings"],
    queryFn: listWindowBindings,
  });
  const sessionsQuery = useQuery({
    queryKey: ["iterm-sessions"],
    queryFn: listItermSessions,
  });

  const createProfileMutation = useMutation({
    mutationFn: createProfile,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["profiles"] });
    },
  });
  const updateProfileMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateProfileInput }) =>
      updateProfile(id, input),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["profiles"] }),
        queryClient.invalidateQueries({ queryKey: ["window-bindings"] }),
      ]);
    },
  });
  const deleteProfileMutation = useMutation({
    mutationFn: deleteProfile,
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["profiles"] }),
        queryClient.invalidateQueries({ queryKey: ["window-bindings"] }),
      ]);
    },
  });

  const createBindingMutation = useMutation({
    mutationFn: createWindowBinding,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["window-bindings"] });
    },
  });
  const refreshPresenceMutation = useMutation({
    mutationFn: refreshWindowBindingPresence,
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["window-bindings"] }),
        queryClient.invalidateQueries({ queryKey: ["iterm-sessions"] }),
      ]);
    },
  });
  const updateBindingMutation = useMutation({
    mutationFn: ({ id, input }: Parameters<typeof updateWindowBinding>[0] extends never
      ? never
      : { id: string; input: Parameters<typeof updateWindowBinding>[1] }) =>
      updateWindowBinding(id, input),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["window-bindings"] });
    },
  });
  const deleteBindingMutation = useMutation({
    mutationFn: deleteWindowBinding,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["window-bindings"] });
    },
  });

  async function startEditingProfile(profile: {
    id: string;
    name: string;
    provider: string;
    execution_mode: string;
    model_name: string;
    base_url: string;
  }) {
    const executionMode = normalizeExecutionMode(profile.execution_mode);
    setEditingProfileId(profile.id);
    setEditingProfileSecretMissing(false);
    setEditingProfileSecretVisible(false);
    setEditingProfileSecretError(null);
    setIsLoadingEditingProfileSecret(true);
    setEditingProfileForm({
      name: profile.name,
      provider: PROVIDER_BY_EXECUTION_MODE[executionMode],
      execution_mode: executionMode,
      model_name: profile.model_name,
      base_url: profile.base_url,
      api_key: "",
    });

    try {
      const secret = await getProfileSecret(profile.id);
      setEditingProfileForm((current) => ({
        ...current,
        api_key: secret.api_key ?? "",
      }));
      setEditingProfileSecretMissing(secret.api_key == null);
    } catch (error) {
      setEditingProfileSecretError(getErrorMessage(error));
      setEditingProfileSecretMissing(true);
    } finally {
      setIsLoadingEditingProfileSecret(false);
    }
  }

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>目标配置</h2>
        <p>将 iTerm2 会话绑定到不同模型配置，并使用各自独立的密钥与基础地址。</p>
      </header>

      <div className="three-col">
        <div className="panel">
          <h3>创建配置</h3>
          <ProfileForm
            isPending={createProfileMutation.isPending}
            onSubmit={(input) => {
              createProfileMutation.mutate(input);
            }}
          />
        </div>

        <div className="panel">
          <h3>模型配置</h3>
          {profilesQuery.isLoading ? <p className="muted">正在加载配置...</p> : null}
          {profilesQuery.isError ? (
            <p className="error-text">加载配置失败。{String(profilesQuery.error)}</p>
          ) : null}
          {profilesQuery.data && profilesQuery.data.length === 0 ? (
            <p className="muted">还没有模型配置。</p>
          ) : null}
          {profilesQuery.data ? (
            <ul className="card-list">
              {profilesQuery.data.map((profile) => (
                <li className="data-card" key={profile.id}>
                  {editingProfileId === profile.id ? (
                    <>
                      <label className="field">
                        <span>名称</span>
                        <input
                          value={editingProfileForm.name}
                          onChange={(event) => {
                            setEditingProfileForm((current) => ({
                              ...current,
                              name: event.target.value,
                            }));
                          }}
                        />
                      </label>
                      <label className="field">
                        <span>提供方</span>
                        <input
                          value={editingProfileForm.provider}
                          onChange={(event) => {
                            setEditingProfileForm((current) => ({
                              ...current,
                              provider: event.target.value,
                            }));
                          }}
                        />
                      </label>
                      <label className="field">
                        <span>执行模式</span>
                        <select
                          value={editingProfileForm.execution_mode}
                          onChange={(event) => {
                            const executionMode = normalizeExecutionMode(event.target.value);
                            setEditingProfileForm((current) => ({
                              ...current,
                              execution_mode: executionMode,
                              provider:
                                current.provider.trim() ||
                                PROVIDER_BY_EXECUTION_MODE[executionMode],
                            }));
                          }}
                        >
                          {EXECUTION_MODE_OPTIONS.map((option) => (
                            <option key={option.value} value={option.value}>
                              {option.label}
                            </option>
                          ))}
                        </select>
                      </label>
                      <label className="field">
                        <span>模型名称</span>
                        <input
                          value={editingProfileForm.model_name}
                          onChange={(event) => {
                            setEditingProfileForm((current) => ({
                              ...current,
                              model_name: event.target.value,
                            }));
                          }}
                        />
                      </label>
                      <label className="field">
                        <span>基础地址</span>
                        <input
                          value={editingProfileForm.base_url}
                          onChange={(event) => {
                            setEditingProfileForm((current) => ({
                              ...current,
                              base_url: event.target.value,
                            }));
                          }}
                        />
                      </label>
                      <label className="field">
                        <span>API key</span>
                        <div className="stack-inline">
                          <input
                            placeholder={
                              editingProfileSecretMissing
                                ? "当前未保存 API key，请重新输入"
                                : "正在加载已保存的 API key..."
                            }
                            type={editingProfileSecretVisible ? "text" : "password"}
                            value={editingProfileForm.api_key}
                            onChange={(event) => {
                              setEditingProfileForm((current) => ({
                                ...current,
                                api_key: event.target.value,
                              }));
                              if (event.target.value.trim()) {
                                setEditingProfileSecretMissing(false);
                              }
                            }}
                          />
                          <button
                            className="ghost-btn"
                            onClick={() => {
                              setEditingProfileSecretVisible((current) => !current);
                            }}
                            type="button"
                          >
                            {editingProfileSecretVisible ? "隐藏 API key" : "显示 API key"}
                          </button>
                        </div>
                      </label>
                      {isLoadingEditingProfileSecret ? (
                        <p className="muted">正在加载已保存的 API key...</p>
                      ) : null}
                      {editingProfileSecretMissing ? (
                        <p className="error-text">当前未找到已保存的 API key，请重新输入后保存。</p>
                      ) : null}
                      {editingProfileSecretError ? (
                        <p className="error-text">读取当前 API key 失败。{editingProfileSecretError}</p>
                      ) : null}
                      <div className="stack-inline">
                        <button
                          className="primary-btn"
                          disabled={
                            updateProfileMutation.isPending ||
                            isLoadingEditingProfileSecret ||
                            (editingProfileSecretMissing && !editingProfileForm.api_key.trim())
                          }
                          onClick={() => {
                            const executionMode = normalizeExecutionMode(
                              editingProfileForm.execution_mode,
                            );
                            updateProfileMutation.mutate({
                              id: profile.id,
                              input: {
                                ...editingProfileForm,
                                execution_mode: executionMode,
                                provider:
                                  editingProfileForm.provider.trim() ||
                                  PROVIDER_BY_EXECUTION_MODE[executionMode],
                              },
                            });
                            setEditingProfileId(null);
                          }}
                          type="button"
                        >
                          保存配置
                        </button>
                        <button
                          className="ghost-btn"
                          onClick={() => {
                            setEditingProfileId(null);
                            setEditingProfileSecretMissing(false);
                            setEditingProfileSecretVisible(false);
                            setEditingProfileSecretError(null);
                            setIsLoadingEditingProfileSecret(false);
                          }}
                          type="button"
                        >
                          取消
                        </button>
                      </div>
                    </>
                  ) : (
                    <>
                      <strong>{profile.name}</strong>
                      <span>
                        {getExecutionModeLabel(normalizeExecutionMode(profile.execution_mode))} /{" "}
                        {profile.model_name}
                      </span>
                      <span>{profile.base_url}</span>
                      <span>更新时间：{formatDateTime(profile.updated_at)}</span>
                      <div className="stack-inline">
                        <button
                          className="ghost-btn"
                          onClick={() => {
                            void startEditingProfile(profile);
                          }}
                          type="button"
                        >
                          编辑配置 {profile.name}
                        </button>
                        <button
                          className="ghost-btn"
                          disabled={deleteProfileMutation.isPending}
                          onClick={() => {
                            setPendingDeleteProfileId(profile.id);
                          }}
                          type="button"
                        >
                          删除配置 {profile.name}
                        </button>
                        {pendingDeleteProfileId === profile.id ? (
                          <>
                            <button
                              className="primary-btn"
                              disabled={deleteProfileMutation.isPending}
                              onClick={() => {
                                deleteProfileMutation.mutate(profile.id);
                                setPendingDeleteProfileId(null);
                              }}
                              type="button"
                            >
                              确认删除配置 {profile.name}
                            </button>
                            <button
                              className="ghost-btn"
                              disabled={deleteProfileMutation.isPending}
                              onClick={() => {
                                setPendingDeleteProfileId(null);
                              }}
                              type="button"
                            >
                              取消删除
                            </button>
                          </>
                        ) : null}
                      </div>
                    </>
                  )}
                </li>
              ))}
            </ul>
          ) : null}
          {updateProfileMutation.isError ? (
            <p className="error-text">更新配置失败。{getErrorMessage(updateProfileMutation.error)}</p>
          ) : null}
          {deleteProfileMutation.isError ? (
            <p className="error-text">删除配置失败。{getErrorMessage(deleteProfileMutation.error)}</p>
          ) : null}
        </div>

        <div className="panel">
          <h3>窗口绑定</h3>
          {bindingsQuery.isError ? (
            <p className="error-text">加载绑定失败。{String(bindingsQuery.error)}</p>
          ) : null}
          {sessionsQuery.isError ? (
            <p className="error-text">发现 iTerm 会话失败。{String(sessionsQuery.error)}</p>
          ) : null}
          <WindowBindingList
            bindings={bindingsQuery.data ?? []}
            sessions={sessionsQuery.data ?? []}
            profiles={profilesQuery.data ?? []}
            isPending={createBindingMutation.isPending}
            isRefreshingSessions={sessionsQuery.isFetching || refreshPresenceMutation.isPending}
            onRefreshSessions={() => {
              refreshPresenceMutation.mutate();
            }}
            onCreate={(input) => {
              createBindingMutation.mutate(input);
            }}
            isUpdatingBinding={updateBindingMutation.isPending}
            isDeletingBinding={deleteBindingMutation.isPending}
            actionError={
              (createBindingMutation.isError && getErrorMessage(createBindingMutation.error)) ||
              (updateBindingMutation.isError && getErrorMessage(updateBindingMutation.error)) ||
              (deleteBindingMutation.isError && getErrorMessage(deleteBindingMutation.error)) ||
              undefined
            }
            onUpdate={(id, input) => {
              updateBindingMutation.mutate({ id, input });
            }}
            onDelete={(id) => {
              deleteBindingMutation.mutate(id);
            }}
          />
        </div>
      </div>
    </section>
  );
}
