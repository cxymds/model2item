import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { CaseForm } from "../components/CaseForm";
import {
  createEvaluationCase,
  deleteEvaluationCase,
  listEvaluationCases,
  updateEvaluationCase,
} from "../../../lib/tauri";
import { formatDateTime } from "../../../lib/formatters";
import type { CreateEvaluationCaseInput } from "../../../types/api";

function getErrorMessage(error: unknown) {
  const message = String(error);
  return message.startsWith("Error: ") ? message.slice(7) : message;
}

export function CaseLibraryPage() {
  const queryClient = useQueryClient();
  const [editingCaseId, setEditingCaseId] = useState<string | null>(null);
  const casesQuery = useQuery({
    queryKey: ["evaluation-cases"],
    queryFn: listEvaluationCases,
  });

  const createCaseMutation = useMutation({
    mutationFn: createEvaluationCase,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["evaluation-cases"] });
    },
  });
  const updateCaseMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: CreateEvaluationCaseInput }) =>
      updateEvaluationCase(id, input),
    onSuccess: async () => {
      setEditingCaseId(null);
      await queryClient.invalidateQueries({ queryKey: ["evaluation-cases"] });
    },
  });
  const deleteCaseMutation = useMutation({
    mutationFn: deleteEvaluationCase,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["evaluation-cases"] });
    },
  });

  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>案例库</h2>
        <p>创建可复用的旧代码理解提示词，并在多个模型目标之间重复运行。</p>
      </header>

      <div className="two-col">
        <div className="panel">
          <h3>创建案例</h3>
          <CaseForm
            isPending={createCaseMutation.isPending}
            onSubmit={(input) => {
              createCaseMutation.mutate(input);
            }}
          />
        </div>

        <div className="panel">
          <h3>已保存案例</h3>
          {casesQuery.isLoading ? <p className="muted">正在加载案例...</p> : null}
          {casesQuery.isError ? (
            <p className="error-text">加载案例失败。{String(casesQuery.error)}</p>
          ) : null}
          {updateCaseMutation.isError ? (
            <p className="error-text">{getErrorMessage(updateCaseMutation.error)}</p>
          ) : null}
          {deleteCaseMutation.isError ? (
            <p className="error-text">{getErrorMessage(deleteCaseMutation.error)}</p>
          ) : null}
          {casesQuery.data && casesQuery.data.length === 0 ? (
            <p className="muted">还没有评测案例。</p>
          ) : null}
          {casesQuery.data ? (
            <ul className="card-list">
              {casesQuery.data.map((item) => (
                <li className="data-card" key={item.id}>
                  {editingCaseId === item.id ? (
                    <>
                      <CaseForm
                        initialValue={{
                          title: item.title,
                          prompt: item.prompt,
                          context_payload: item.context_payload,
                          notes: item.notes,
                        }}
                        isPending={updateCaseMutation.isPending}
                        onSubmit={(input) => {
                          updateCaseMutation.mutate({ id: item.id, input });
                        }}
                        resetOnSubmit={false}
                        submitLabel="保存修改"
                      />
                      <button
                        className="ghost-btn"
                        onClick={() => {
                          setEditingCaseId(null);
                        }}
                        type="button"
                      >
                        取消编辑
                      </button>
                    </>
                  ) : (
                    <>
                      <strong>{item.title}</strong>
                      <p>{item.prompt.slice(0, 160)}...</p>
                      <span>创建时间：{formatDateTime(item.created_at)}</span>
                      <div className="stack-inline">
                        <button
                          className="ghost-btn"
                          onClick={() => {
                            setEditingCaseId(item.id);
                          }}
                          type="button"
                        >
                          编辑 {item.title}
                        </button>
                        <button
                          className="ghost-btn"
                          onClick={() => {
                            if (!window.confirm(`确认删除案例“${item.title}”吗？`)) return;
                            deleteCaseMutation.mutate(item.id);
                          }}
                          type="button"
                        >
                          删除 {item.title}
                        </button>
                      </div>
                    </>
                  )}
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </div>
    </section>
  );
}
