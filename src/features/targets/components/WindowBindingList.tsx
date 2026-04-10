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

  const onlineSessionIds = useMemo(() => {
    return new Set(sessionOptions.map((session) => session.session_id));
  }, [sessionOptions]);

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
          <span>已发现会话</span>
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
            <option value="">请选择已发现会话</option>
            {sessionOptions.map((session) => (
              <option key={session.session_id} value={session.session_id}>
                {session.label}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>会话 ID</span>
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
          <span>显示名称</span>
          <input
            value={form.display_name}
            onChange={(event) => {
              setForm((current) => ({ ...current, display_name: event.target.value }));
            }}
            placeholder="窗口 A - GPT 基线"
            required
          />
        </label>

        <label className="field">
          <span>绑定配置</span>
          <select
            value={form.profile_id}
            onChange={(event) => {
              setForm((current) => ({ ...current, profile_id: event.target.value }));
            }}
            required
          >
            <option value="">请选择配置</option>
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name} ({profile.model_name})
              </option>
            ))}
          </select>
        </label>

        <button className="primary-btn" disabled={isPending || profiles.length === 0} type="submit">
          {isPending ? "绑定中..." : "创建绑定"}
        </button>
        <button className="ghost-btn" disabled={isRefreshingSessions} onClick={onRefreshSessions} type="button">
          {isRefreshingSessions ? "刷新中..." : "刷新会话"}
        </button>
      </form>

      <div className="list-block">
        <h3>当前绑定</h3>
        {bindings.length === 0 ? (
          <p className="muted">还没有绑定，请先在上方创建。</p>
        ) : (
          <ul className="card-list">
            {bindings.map((binding) => {
              const profile = profileMap.get(binding.profile_id);
              const isOnline = onlineSessionIds.has(binding.iterm_session_id);
              return (
                <li className="data-card" key={binding.id}>
                  <strong>{binding.display_name}</strong>
                  <span>会话：{binding.iterm_session_id}</span>
                  <span>
                    配置：
                    {profile ? `${profile.name} (${profile.model_name})` : `未知配置 ${shortenId(binding.profile_id)}`}
                  </span>
                  <span>启用状态：{binding.enabled === 1 ? "已启用" : "已禁用"}</span>
                  <span>连接状态：{isOnline ? "在线" : "离线"}</span>
                  <span>最近在线：{formatDateTime(binding.last_seen_at)}</span>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
