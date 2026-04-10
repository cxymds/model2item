import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CaseForm } from "../components/CaseForm";
import { createEvaluationCase, listEvaluationCases } from "../../../lib/tauri";
import { formatDateTime } from "../../../lib/formatters";

export function CaseLibraryPage() {
  const queryClient = useQueryClient();
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
          {casesQuery.data && casesQuery.data.length === 0 ? (
            <p className="muted">还没有评测案例。</p>
          ) : null}
          {casesQuery.data ? (
            <ul className="card-list">
              {casesQuery.data.map((item) => (
                <li className="data-card" key={item.id}>
                  <strong>{item.title}</strong>
                  <p>{item.prompt.slice(0, 160)}...</p>
                  <span>创建时间：{formatDateTime(item.created_at)}</span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </div>
    </section>
  );
}
