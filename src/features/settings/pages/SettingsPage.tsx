export function SettingsPage() {
  return (
    <section className="page stack-block">
      <header className="section-header">
        <h2>Settings</h2>
        <p>Runtime defaults for local evaluation runs. Save wiring can be added in a later phase.</p>
      </header>

      <div className="two-col">
        <div className="panel">
          <h3>Execution defaults</h3>
          <label className="field">
            <span>Default timeout (ms)</span>
            <input defaultValue="30000" type="number" />
          </label>
          <label className="field">
            <span>Max concurrency</span>
            <input defaultValue="3" type="number" />
          </label>
        </div>
        <div className="panel">
          <h3>Storage and reports</h3>
          <label className="field">
            <span>Database location</span>
            <input defaultValue="./workbench.db" />
          </label>
          <label className="field">
            <span>Default export format</span>
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
