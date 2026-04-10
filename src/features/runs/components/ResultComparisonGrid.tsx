export type ResultComparisonColumn = {
  label: string;
  summary: string;
};

type ResultComparisonGridProps = {
  columns: ResultComparisonColumn[];
};

export function ResultComparisonGrid({ columns }: ResultComparisonGridProps) {
  return (
    <section className="comparison-grid">
      {columns.map((item) => (
        <article className="comparison-card" key={item.label}>
          <h4>{item.label}</h4>
          <p>{item.summary}</p>
        </article>
      ))}
    </section>
  );
}
