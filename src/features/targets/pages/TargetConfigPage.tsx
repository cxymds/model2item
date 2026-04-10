import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createProfile,
  createWindowBinding,
  listItermSessions,
  listProfiles,
  listWindowBindings,
  refreshWindowBindingPresence,
} from "../../../lib/tauri";
import { formatDateTime } from "../../../lib/formatters";
import { ProfileForm } from "../components/ProfileForm";
import { WindowBindingList } from "../components/WindowBindingList";

export function TargetConfigPage() {
  const queryClient = useQueryClient();
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
                  <strong>{profile.name}</strong>
                  <span>
                    {profile.provider} / {profile.model_name}
                  </span>
                  <span>{profile.base_url}</span>
                  <span>更新时间：{formatDateTime(profile.updated_at)}</span>
                </li>
              ))}
            </ul>
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
          />
        </div>
      </div>
    </section>
  );
}
