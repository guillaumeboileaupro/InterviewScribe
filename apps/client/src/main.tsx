import React from "react";
import ReactDOM from "react-dom/client";
import "./styles.css";

function App() {
  return (
    <main className="shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">InterviewScribe</p>
          <h1>Vos entretiens, clairement retranscrits.</h1>
        </div>
        <button className="secondary" type="button">Reglages</button>
      </header>

      <section className="actions" aria-labelledby="start-title">
        <div>
          <h2 id="start-title">Commencer</h2>
          <p>Enregistrez une discussion ou importez un fichier existant.</p>
        </div>
        <div className="buttonRow">
          <button className="primary" type="button">Nouvel entretien</button>
          <button className="secondary" type="button">Importer un fichier</button>
        </div>
      </section>

      <section className="recent" aria-labelledby="recent-title">
        <div className="sectionTitle">
          <h2 id="recent-title">Entretiens recents</h2>
          <span>Stockage local</span>
        </div>
        <div className="emptyState">
          <div className="emptyIcon" aria-hidden="true">T</div>
          <h3>Aucun entretien pour le moment</h3>
          <p>Votre premiere transcription apparaitra ici.</p>
        </div>
      </section>
    </main>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><App /></React.StrictMode>,
);

