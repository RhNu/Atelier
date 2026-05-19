import "./App.css";

function App() {
  return (
    <main className="app-shell">
      <section className="workspace-panel" aria-labelledby="workspace-title">
        <p className="eyebrow">NovelAI desktop workspace</p>
        <h1 id="workspace-title">NAI Atelier</h1>
        <p className="summary">
          A thin Tauri shell with a React workbench front end. Feature modules will grow around
          prompt authoring, generation jobs, artifacts, and gallery workflows.
        </p>
      </section>

      <section className="status-grid" aria-label="Scaffold status">
        <article>
          <span>Frontend</span>
          <strong>Vite React TS</strong>
        </article>
        <article>
          <span>Desktop</span>
          <strong>Tauri v2</strong>
        </article>
        <article>
          <span>Rust</span>
          <strong>Workspace ready</strong>
        </article>
      </section>
    </main>
  );
}

export default App;
