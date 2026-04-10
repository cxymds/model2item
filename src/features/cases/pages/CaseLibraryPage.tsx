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
        <h2>Case Library</h2>
        <p>Create reusable old-code understanding prompts and re-run them across model targets.</p>
      </header>

      <div className="two-col">
        <div className="panel">
          <h3>Create case</h3>
          <CaseForm
            isPending={createCaseMutation.isPending}
            onSubmit={(input) => {
              createCaseMutation.mutate(input);
            }}
          />
        </div>

        <div className="panel">
          <h3>Saved cases</h3>
          {casesQuery.isLoading ? <p className="muted">Loading cases...</p> : null}
          {casesQuery.isError ? (
            <p className="error-text">Failed to load cases. {String(casesQuery.error)}</p>
          ) : null}
          {casesQuery.data && casesQuery.data.length === 0 ? (
            <p className="muted">No evaluation cases yet.</p>
          ) : null}
          {casesQuery.data ? (
            <ul className="card-list">
              {casesQuery.data.map((item) => (
                <li className="data-card" key={item.id}>
                  <strong>{item.title}</strong>
                  <p>{item.prompt.slice(0, 160)}...</p>
                  <span>Created: {formatDateTime(item.created_at)}</span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </div>
    </section>
  );
}
