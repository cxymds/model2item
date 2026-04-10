import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createProfile,
  createWindowBinding,
  listItermSessions,
  listProfiles,
  listWindowBindings,
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

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>Target Configuration</h2>
        <p>Bind iTerm2 sessions to model profiles with independent keys and base URLs.</p>
      </header>

      <div className="three-col">
        <div className="panel">
          <h3>Create profile</h3>
          <ProfileForm
            isPending={createProfileMutation.isPending}
            onSubmit={(input) => {
              createProfileMutation.mutate(input);
            }}
          />
        </div>

        <div className="panel">
          <h3>Profiles</h3>
          {profilesQuery.isLoading ? <p className="muted">Loading profiles...</p> : null}
          {profilesQuery.isError ? (
            <p className="error-text">Failed to load profiles. {String(profilesQuery.error)}</p>
          ) : null}
          {profilesQuery.data && profilesQuery.data.length === 0 ? (
            <p className="muted">No model profiles yet.</p>
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
                  <span>Updated: {formatDateTime(profile.updated_at)}</span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>

        <div className="panel">
          <h3>Window bindings</h3>
          {bindingsQuery.isError ? (
            <p className="error-text">Failed to load bindings. {String(bindingsQuery.error)}</p>
          ) : null}
          {sessionsQuery.isError ? (
            <p className="error-text">Failed to discover iTerm sessions. {String(sessionsQuery.error)}</p>
          ) : null}
          <WindowBindingList
            bindings={bindingsQuery.data ?? []}
            sessions={sessionsQuery.data ?? []}
            profiles={profilesQuery.data ?? []}
            isPending={createBindingMutation.isPending}
            isRefreshingSessions={sessionsQuery.isFetching}
            onRefreshSessions={() => {
              void sessionsQuery.refetch();
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
