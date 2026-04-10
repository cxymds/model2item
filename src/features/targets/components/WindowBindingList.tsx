import { useMemo, useState } from "react";
import type {
  CreateWindowBindingInput,
  ItermSessionResponse,
  ProfileResponse,
  WindowBindingResponse,
} from "../../../types/api";
import { formatDateTime, shortenId } from "../../../lib/formatters";

type WindowBindingListProps = {
  bindings: WindowBindingResponse[];
  sessions: ItermSessionResponse[];
  profiles: ProfileResponse[];
  isPending: boolean;
  isRefreshingSessions: boolean;
  onRefreshSessions: () => void;
  onCreate: (input: CreateWindowBindingInput) => void;
};

export function WindowBindingList({
  bindings,
  sessions,
  profiles,
  isPending,
  isRefreshingSessions,
  onRefreshSessions,
  onCreate,
}: WindowBindingListProps) {
  const [form, setForm] = useState<CreateWindowBindingInput>({
    iterm_session_id: "",
    display_name: "",
    profile_id: "",
  });

  const profileMap = useMemo(() => {
    return new Map(profiles.map((item) => [item.id, item]));
  }, [profiles]);

  const sessionOptions = useMemo(() => {
    return sessions.map((session) => ({
      ...session,
      label: `${session.window_title} / ${session.tab_title} / ${session.session_title}`,
    }));
  }, [sessions]);

  return (
    <div className="stack-block">
      <form
        className="stack-form"
        onSubmit={(event) => {
          event.preventDefault();
          if (!form.profile_id || !form.iterm_session_id.trim() || !form.display_name.trim()) return;
          onCreate(form);
          setForm({
            iterm_session_id: "",
            display_name: "",
            profile_id: "",
          });
        }}
      >
        <label className="field">
          <span>Discovered sessions</span>
          <select
            value={form.iterm_session_id}
            onChange={(event) => {
              const selectedSession = sessionOptions.find(
                (session) => session.session_id === event.target.value,
              );
              setForm((current) => ({
                ...current,
                iterm_session_id: event.target.value,
                display_name:
                  current.display_name.trim().length > 0
                    ? current.display_name
                    : selectedSession?.label ?? "",
              }));
            }}
          >
            <option value="">Select discovered session</option>
            {sessionOptions.map((session) => (
              <option key={session.session_id} value={session.session_id}>
                {session.label}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>Session id</span>
          <input
            value={form.iterm_session_id}
            onChange={(event) => {
              setForm((current) => ({ ...current, iterm_session_id: event.target.value }));
            }}
            placeholder="session-001"
            required
          />
        </label>

        <label className="field">
          <span>Display name</span>
          <input
            value={form.display_name}
            onChange={(event) => {
              setForm((current) => ({ ...current, display_name: event.target.value }));
            }}
            placeholder="Window A - GPT baseline"
            required
          />
        </label>

        <label className="field">
          <span>Bound profile</span>
          <select
            value={form.profile_id}
            onChange={(event) => {
              setForm((current) => ({ ...current, profile_id: event.target.value }));
            }}
            required
          >
            <option value="">Select profile</option>
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name} ({profile.model_name})
              </option>
            ))}
          </select>
        </label>

        <button className="primary-btn" disabled={isPending || profiles.length === 0} type="submit">
          {isPending ? "Binding..." : "Create binding"}
        </button>
        <button className="ghost-btn" disabled={isRefreshingSessions} onClick={onRefreshSessions} type="button">
          {isRefreshingSessions ? "Refreshing..." : "Refresh sessions"}
        </button>
      </form>

      <div className="list-block">
        <h3>Discovered sessions</h3>
        {sessionOptions.length === 0 ? (
          <p className="muted">No iTerm2 sessions discovered yet. Open iTerm2 and refresh.</p>
        ) : (
          <ul className="card-list">
            {sessionOptions.map((session) => (
              <li className="data-card" key={session.session_id}>
                <strong>{session.window_title}</strong>
                <span>{session.tab_title}</span>
                <span>{session.session_title}</span>
                <span>Session: {session.session_id}</span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="list-block">
        <h3>Current bindings</h3>
        {bindings.length === 0 ? (
          <p className="muted">No bindings yet. Create one above.</p>
        ) : (
          <ul className="card-list">
            {bindings.map((binding) => {
              const profile = profileMap.get(binding.profile_id);
              return (
                <li className="data-card" key={binding.id}>
                  <strong>{binding.display_name}</strong>
                  <span>Session: {binding.iterm_session_id}</span>
                  <span>
                    Profile:{" "}
                    {profile ? `${profile.name} (${profile.model_name})` : `Unknown ${shortenId(binding.profile_id)}`}
                  </span>
                  <span>Status: {binding.enabled === 1 ? "Enabled" : "Disabled"}</span>
                  <span>Last seen: {formatDateTime(binding.last_seen_at)}</span>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
