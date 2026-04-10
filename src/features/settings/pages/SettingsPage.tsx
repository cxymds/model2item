export function SettingsPage() {
  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>设置</h2>
        <p>本地评测运行的默认参数设置，后续阶段可补充持久化保存能力。</p>
      </header>

      <div className="two-col">
        <div className="panel">
          <h3>执行默认值</h3>
          <label className="field">
            <span>默认超时时间（毫秒）</span>
            <input defaultValue="30000" type="number" />
          </label>
          <label className="field">
            <span>最大并发数</span>
            <input defaultValue="3" type="number" />
          </label>
        </div>
        <div className="panel">
          <h3>存储与报告</h3>
          <label className="field">
            <span>数据库位置</span>
            <input defaultValue="./workbench.db" />
          </label>
          <label className="field">
            <span>默认导出格式</span>
            <select defaultValue="csv">
              <option value="csv">CSV</option>
              <option value="json">JSON</option>
              <option value="md">Markdown</option>
            </select>
          </label>
        </div>
      </div>
    </section>
  );
}
