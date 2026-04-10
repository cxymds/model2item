const headers = [
  "模型",
  "Pass@1",
  "单测通过率",
  "耗时",
  "Token 成本",
  "综合评分",
];

export type MetricRow = {
  model: string;
  passAt1: string;
  testRate: string;
  latency: string;
  tokenCost: string;
  overall: string;
};

type MetricTableProps = {
  rows: MetricRow[];
};

export function MetricTable({ rows }: MetricTableProps) {
  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            {headers.map((header) => (
              <th key={header}>{header}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.model}>
              <td>{row.model}</td>
              <td>{row.passAt1}</td>
              <td>{row.testRate}</td>
              <td>{row.latency}</td>
              <td>{row.tokenCost}</td>
              <td>{row.overall}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
