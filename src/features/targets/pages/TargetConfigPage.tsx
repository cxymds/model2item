import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createCustomProvider,
  createWindowBinding,
  deleteCustomProvider,
  deleteWindowBinding,
  listCustomProviders,
  listItermSessions,
  listWindowBindings,
  refreshWindowBindingPresence,
  updateWindowBinding,
} from "../../../lib/tauri";
import { formatDateTime } from "../../../lib/formatters";
import { CustomProviderForm } from "../components/CustomProviderForm";
import { WindowBindingList } from "../components/WindowBindingList";
import { useState } from "react";

function getErrorMessage(error: unknown) {
  const message = String(error);
  return message.startsWith("Error: ") ? message.slice(7) : message;
}

export function TargetConfigPage() {
  const queryClient = useQueryClient();
  const [pendingDeleteProviderId, setPendingDeleteProviderId] = useState<string | null>(null);

  const customProvidersQuery = useQuery({
    queryKey: ["custom-providers"],
    queryFn: listCustomProviders,
  });
  const bindingsQuery = useQuery({
    queryKey: ["window-bindings"],
    queryFn: listWindowBindings,
  });
  const sessionsQuery = useQuery({
    queryKey: ["iterm-sessions"],
    queryFn: listItermSessions,
  });

  const createCustomProviderMutation = useMutation({
    mutationFn: (input: Parameters<typeof createCustomProvider>[0]) =>
      createCustomProvider(input),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["custom-providers"] });
    },
  });
  const createBindingMutation = useMutation({
    mutationFn: (input: Parameters<typeof createWindowBinding>[0]) =>
      createWindowBinding(input),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["window-bindings"] });
    },
  });
  const refreshPresenceMutation = useMutation({
    mutationFn: () => refreshWindowBindingPresence(),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["window-bindings"] }),
        queryClient.invalidateQueries({ queryKey: ["iterm-sessions"] }),
      ]);
    },
  });
  const updateBindingMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: Parameters<typeof updateWindowBinding>[1] }) =>
      updateWindowBinding(id, input),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["window-bindings"] });
    },
  });
  const deleteBindingMutation = useMutation({
    mutationFn: (id: string) => deleteWindowBinding(id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["window-bindings"] });
    },
  });
  const deleteProviderMutation = useMutation({
    mutationFn: (id: string) => deleteCustomProvider(id),
    onSuccess: async () => {
      setPendingDeleteProviderId(null);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["custom-providers"] }),
        queryClient.invalidateQueries({ queryKey: ["window-bindings"] }),
      ]);
    },
  });

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>目标配置</h2>
        <p>将 iTerm2 会话绑定到 Provider 配置，并使用各自独立的密钥与基础地址。</p>
      </header>

      <div className="three-col">
        <div className="panel">
          <h3>创建 Provider</h3>
          <CustomProviderForm
            isPending={createCustomProviderMutation.isPending}
            onSubmit={(input) => {
              createCustomProviderMutation.mutate(input);
            }}
          />
          {createCustomProviderMutation.isError ? (
            <p className="error-text">
              保存 Provider 失败。{getErrorMessage(createCustomProviderMutation.error)}
            </p>
          ) : null}
        </div>

        <div className="panel">
          <h3>Provider 配置</h3>
          {customProvidersQuery.isLoading ? <p className="muted">正在加载 Provider...</p> : null}
          {customProvidersQuery.isError ? (
            <p className="error-text">加载 Provider 失败。{String(customProvidersQuery.error)}</p>
          ) : null}
          {customProvidersQuery.data && customProvidersQuery.data.length === 0 ? (
            <p className="muted">还没有 Provider 配置。</p>
          ) : null}
          {customProvidersQuery.data ? (
            <ul className="card-list">
              {customProvidersQuery.data.map((provider) => (
                <li className="data-card" key={provider.id}>
                  <strong>{provider.name}</strong>
                  <span>
                    {provider.client_type} / {provider.provider_key} / {provider.default_model}
                  </span>
                  <span>{provider.base_url || "未设置 Base URL"}</span>
                  <span>更新时间：{formatDateTime(provider.updated_at)}</span>
                  <div className="stack-inline">
                    <button
                      className="ghost-btn"
                      disabled={deleteProviderMutation.isPending}
                      onClick={() => {
                        setPendingDeleteProviderId(provider.id);
                      }}
                      type="button"
                    >
                      删除 Provider {provider.name}
                    </button>
                    {pendingDeleteProviderId === provider.id ? (
                      <>
                        <button
                          className="primary-btn"
                          disabled={deleteProviderMutation.isPending}
                          onClick={() => {
                            deleteProviderMutation.mutate(provider.id);
                          }}
                          type="button"
                        >
                          确认删除 Provider {provider.name}
                        </button>
                        <button
                          className="ghost-btn"
                          disabled={deleteProviderMutation.isPending}
                          onClick={() => {
                            setPendingDeleteProviderId(null);
                          }}
                          type="button"
                        >
                          取消删除
                        </button>
                      </>
                    ) : null}
                  </div>
                </li>
              ))}
            </ul>
          ) : null}
          {deleteProviderMutation.isError ? (
            <p className="error-text">
              删除 Provider 失败。{getErrorMessage(deleteProviderMutation.error)}
            </p>
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
          {customProvidersQuery.isError ? (
            <p className="error-text">加载 Provider 失败。{String(customProvidersQuery.error)}</p>
          ) : null}
          <WindowBindingList
            bindings={bindingsQuery.data ?? []}
            sessions={sessionsQuery.data ?? []}
            customProviders={customProvidersQuery.data ?? []}
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
