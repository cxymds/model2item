import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  createProfile,
  createWindowBinding,
  deleteProfile,
  deleteWindowBinding,
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
import type { UpdateProfileInput } from "../../../types/api";

function getErrorMessage(error: unknown) {
  const message = String(error);
  return message.startsWith("Error: ") ? message.slice(7) : message;
}

export function TargetConfigPage() {
  const queryClient = useQueryClient();
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [editingProfileForm, setEditingProfileForm] = useState<UpdateProfileInput>({
    name: "",
    provider: "",
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
                        <span>新 API key</span>
                        <input
                          placeholder="留空则保持当前密钥"
                          type="password"
                          value={editingProfileForm.api_key}
                          onChange={(event) => {
                            setEditingProfileForm((current) => ({
                              ...current,
                              api_key: event.target.value,
                            }));
                          }}
                        />
                      </label>
                      <div className="stack-inline">
                        <button
                          className="primary-btn"
                          disabled={updateProfileMutation.isPending}
                          onClick={() => {
                            updateProfileMutation.mutate({
                              id: profile.id,
                              input: editingProfileForm,
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
                        {profile.provider} / {profile.model_name}
                      </span>
                      <span>{profile.base_url}</span>
                      <span>更新时间：{formatDateTime(profile.updated_at)}</span>
                      <div className="stack-inline">
                        <button
                          className="ghost-btn"
                          onClick={() => {
                            setEditingProfileId(profile.id);
                            setEditingProfileForm({
                              name: profile.name,
                              provider: profile.provider,
                              model_name: profile.model_name,
                              base_url: profile.base_url,
                              api_key: "",
                            });
                          }}
                          type="button"
                        >
                          编辑配置 {profile.name}
                        </button>
                        <button
                          className="ghost-btn"
                          disabled={deleteProfileMutation.isPending}
                          onClick={() => {
                            if (!window.confirm(`确认删除配置“${profile.name}”吗？`)) return;
                            deleteProfileMutation.mutate(profile.id);
                          }}
                          type="button"
                        >
                          删除配置 {profile.name}
                        </button>
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
