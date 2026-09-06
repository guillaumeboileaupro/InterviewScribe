import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  ensureWhisperModel,
  exportInterview,
  getInterview,
  importInterview,
  listInterviews,
  transcribeInterview,
  type ExportFormat,
  type Interview,
  type InterviewDetail,
  type ModelStatus,
} from "./api";

const logo = new URL(
  "../../../assets/interviewscribe-logo.svg",
  import.meta.url,
).href;
const example = [
  {
    time: "00:00",
    speaker: "Intervenant 1",
    text: "Pour commencer, pouvez-vous nous présenter votre parcours ?",
  },
  {
    time: "00:06",
    speaker: "Intervenant 2",
    text: "J’ai commencé dans une petite équipe. Cette expérience m’a appris à écouter, à poser des questions et à construire des solutions ensemble.",
  },
  {
    time: "00:18",
    speaker: "Intervenant 1",
    text: "Qu’est-ce qui vous a le plus marqué dans cette expérience ?",
  },
  {
    time: "00:24",
    speaker: "Intervenant 2",
    text: "La qualité des échanges. Prendre le temps de comprendre les autres change vraiment la manière de travailler.",
  },
];
type Page = "library" | "example" | "settings" | "prepare" | "interview";

function formatTimestamp(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

export default function App() {
  const [page, setPage] = useState<Page>("library");
  const [timestamps, setTimestamps] = useState(true);
  const [theme, setTheme] = useState("system");
  const [source, setSource] = useState("microphone");

  const [interviews, setInterviews] = useState<Interview[]>([]);
  const [currentInterview, setCurrentInterview] =
    useState<InterviewDetail | null>(null);

  const [prepareTitle, setPrepareTitle] = useState("");
  const [prepareBusy, setPrepareBusy] = useState(false);
  const [prepareError, setPrepareError] = useState<string | null>(null);

  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null);
  const [modelError, setModelError] = useState<string | null>(null);

  const refreshInterviews = () => {
    listInterviews()
      .then(setInterviews)
      .catch(() => setInterviews([]));
  };

  useEffect(() => {
    refreshInterviews();
  }, []);

  useEffect(() => {
    if (page === "settings") {
      ensureWhisperModel()
        .then(setModelStatus)
        .catch((err) => setModelError(String(err)));
    }
  }, [page]);

  const openInterview = (interviewId: number) => {
    getInterview(interviewId)
      .then((detail) => {
        setCurrentInterview(detail);
        setPage("interview");
      })
      .catch((err) => setPrepareError(String(err)));
  };

  const importAndTranscribe = async () => {
    setPrepareError(null);
    setPrepareBusy(true);
    try {
      const status = await ensureWhisperModel();
      setModelStatus(status);
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Audio",
            extensions: ["wav", "mp3", "m4a", "flac", "ogg", "aac"],
          },
        ],
      });
      if (!selected || Array.isArray(selected)) return;
      const fileName = selected.split(/[/\\]/).pop() ?? selected;
      const interview = await importInterview(
        prepareTitle.trim() || fileName,
        selected,
      );
      refreshInterviews();
      const detail = await transcribeInterview(interview.id);
      setCurrentInterview(detail);
      setPage("interview");
    } catch (err) {
      setPrepareError(String(err));
    } finally {
      refreshInterviews();
      setPrepareBusy(false);
    }
  };

  const handleExport = async (format: ExportFormat) => {
    if (!currentInterview) return;
    const extension = format === "markdown" ? "md" : format;
    const destination = await save({
      defaultPath: `${currentInterview.interview.title}.${extension}`,
    });
    if (!destination) return;
    try {
      await exportInterview(
        currentInterview.interview.id,
        format,
        timestamps,
        destination,
      );
    } catch (err) {
      setPrepareError(String(err));
    }
  };

  return (
    <div className="app" data-theme={theme}>
      <a className="skipLink" href="#content">
        Aller au contenu
      </a>
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(event) => {
            event.preventDefault();
            setPage("library");
          }}
        >
          <span className="logoBox">
            <img src={logo} alt="" />
          </span>
          <span>
            InterviewScribe
            <span className="brandCaption">L’essentiel de vos échanges</span>
          </span>
        </a>
        <nav aria-label="Navigation principale">
          <span className="navLabel">ESPACE DE TRAVAIL</span>
          <button
            className={page === "library" ? "navItem active" : "navItem"}
            aria-current={page === "library" ? "page" : undefined}
            onClick={() => setPage("library")}
          >
            <span aria-hidden="true">▤</span> Mes entretiens
          </button>
          <button
            className={page === "example" ? "navItem active" : "navItem"}
            aria-current={page === "example" ? "page" : undefined}
            onClick={() => setPage("example")}
          >
            <span aria-hidden="true">≡</span> Découvrir l’éditeur
          </button>
          <button
            className={page === "settings" ? "navItem active" : "navItem"}
            aria-current={page === "settings" ? "page" : undefined}
            onClick={() => setPage("settings")}
          >
            <span aria-hidden="true">⚙</span> Réglages
          </button>
        </nav>
        <div className="localNote">
          <span className="statusDot" /> Pensé pour rester local
          <p>Vos échanges méritent de rester privés.</p>
          <span className="version">Version de développement · 0.1</span>
        </div>
      </aside>
      <div className="workspace">
        <header className="topbar">
          <span>Votre espace personnel</span>
          <span className="badge">Traitement local par défaut</span>
        </header>
        <main id="content" tabIndex={-1}>
          {prepareBusy && (
            <div className="notice" role="status">
              Préparation ou transcription locale en cours… L’audio reste sur
              cet appareil.
            </div>
          )}
          {prepareError && (
            <div className="notice" role="alert">
              {prepareError}
            </div>
          )}
          {page === "library" && (
            <>
              <div className="pageHeading">
                <div>
                  <p className="eyebrow">BIBLIOTHÈQUE</p>
                  <h1>Mes entretiens</h1>
                  <p>Un espace pour écouter, relire et retrouver le fil.</p>
                </div>
                <button
                  className="primary"
                  onClick={() => {
                    setSource("microphone");
                    setPrepareTitle("");
                    setPrepareError(null);
                    setPage("prepare");
                  }}
                >
                  + Nouvel entretien
                </button>
              </div>
              <section className="welcome" aria-labelledby="welcome-title">
                <div>
                  <span className="eyebrow">DE LA VOIX AU TEXTE</span>
                  <h2 id="welcome-title">
                    Chaque échange mérite
                    <br />
                    une trace claire.
                  </h2>
                  <p>
                    Préparez votre entretien ou partez d’un enregistrement.
                    Retrouvez ensuite les prises de parole dans un texte
                    structuré.
                  </p>
                  <button
                    className="secondary"
                    onClick={() => {
                      setSource("file");
                      setPrepareTitle("");
                      setPrepareError(null);
                      setPage("prepare");
                    }}
                  >
                    Importer un fichier <span aria-hidden="true">↗</span>
                  </button>
                </div>
                <div className="preview" aria-hidden="true">
                  <div className="previewHeader">
                    APERÇU DE TRANSCRIPTION<span>EXEMPLE</span>
                  </div>
                  <span className="speaker">
                    Intervenant 1 <small>00:00</small>
                  </span>
                  <p>Commençons par ce qui compte pour vous.</p>
                  <div className="previewRule" />
                  <span className="speaker second">
                    Intervenant 2 <small>00:06</small>
                  </span>
                  <p>
                    Prendre le temps d’écouter et garder une trace fidèle de nos
                    échanges.
                  </p>
                </div>
              </section>
              <section className="library" aria-labelledby="recent-title">
                <div className="sectionTitle">
                  <h2 id="recent-title">
                    Entretiens récents{" "}
                    <span className="count">{interviews.length}</span>
                  </h2>
                  <span>
                    {interviews.length === 0
                      ? "Aucun entretien enregistré"
                      : `${interviews.length} entretien(s)`}
                  </span>
                </div>
                {interviews.length === 0 ? (
                  <div className="emptyState">
                    <span className="emptyMark" aria-hidden="true">
                      ▤
                    </span>
                    <h3>Votre prochain échange commence ici.</h3>
                    <p>
                      Vos entretiens apparaîtront dans cet espace.
                      <br />
                      En attendant, explorez un exemple de transcription.
                    </p>
                    <button
                      className="textButton"
                      onClick={() => setPage("example")}
                    >
                      Découvrir l’éditeur <span aria-hidden="true">→</span>
                    </button>
                  </div>
                ) : (
                  <ul>
                    {interviews.map((interview) => (
                      <li key={interview.id}>
                        <button
                          className="textButton"
                          onClick={() => openInterview(interview.id)}
                        >
                          {interview.title} — {interview.status}
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
              <footer>
                Texte brut préservé <span>·</span> Nettoyage réversible{" "}
                <span>·</span> Horodatages conservés
              </footer>
            </>
          )}
          {page === "example" && (
            <>
              <div className="pageHeading">
                <div>
                  <p className="eyebrow">APERÇU DE L’ÉDITEUR</p>
                  <h1>Un échange, une trace.</h1>
                  <p>Exemple fictif · 2 intervenants · Lecture seule</p>
                </div>
                <button
                  className="secondary"
                  onClick={() => setPage("library")}
                >
                  Retour aux entretiens
                </button>
              </div>
              <div className="notice">
                Cet exemple présente la lecture d’une transcription. L’édition
                et le nettoyage seront disponibles avec les phases suivantes.
              </div>
              <section className="transcript">
                <div className="transcriptToolbar">
                  <strong>Transcription brute</strong>
                  <label className="check">
                    <input
                      type="checkbox"
                      checked={timestamps}
                      onChange={(event) => setTimestamps(event.target.checked)}
                    />{" "}
                    Horodatages
                  </label>
                </div>
                {example.map((segment) => (
                  <article className="segment" key={segment.time}>
                    <div className="segmentMeta">
                      {timestamps && <time>{segment.time}</time>}
                      <span
                        className={
                          segment.speaker === "Intervenant 2"
                            ? "speaker second"
                            : "speaker"
                        }
                      >
                        {segment.speaker}
                      </span>
                    </div>
                    <p>{segment.text}</p>
                  </article>
                ))}
              </section>
            </>
          )}
          {page === "interview" && currentInterview && (
            <>
              <div className="pageHeading">
                <div>
                  <p className="eyebrow">ENTRETIEN</p>
                  <h1>{currentInterview.interview.title}</h1>
                  <p>Statut : {currentInterview.interview.status}</p>
                </div>
                <button
                  className="secondary"
                  onClick={() => setPage("library")}
                >
                  Retour aux entretiens
                </button>
              </div>
              {currentInterview.interview.status === "error" && (
                <div className="notice" role="status">
                  Une erreur est survenue :{" "}
                  {currentInterview.interview.error_message}
                </div>
              )}
              <section className="transcript">
                <div className="transcriptToolbar">
                  <strong>Transcription brute</strong>
                  <label className="check">
                    <input
                      type="checkbox"
                      checked={timestamps}
                      onChange={(event) => setTimestamps(event.target.checked)}
                    />{" "}
                    Horodatages
                  </label>
                </div>
                {currentInterview.segments.length === 0 ? (
                  <p className="muted">Aucun segment pour le moment.</p>
                ) : (
                  currentInterview.segments.map((segment) => {
                    const speaker = currentInterview.speakers.find(
                      (candidate) => candidate.id === segment.speaker_id,
                    );
                    return (
                      <article className="segment" key={segment.id}>
                        <div className="segmentMeta">
                          {timestamps && (
                            <time>{formatTimestamp(segment.start_ms)}</time>
                          )}
                          <span className="speaker">
                            {speaker?.display_name ??
                              speaker?.label ??
                              "Intervenant"}
                          </span>
                        </div>
                        <p>{segment.raw_text}</p>
                      </article>
                    );
                  })
                )}
              </section>
              <section className="settingsPanel">
                <h2>Export</h2>
                <div className="buttonRow">
                  <button
                    className="secondary"
                    onClick={() => handleExport("txt")}
                  >
                    Exporter en TXT
                  </button>
                  <button
                    className="secondary"
                    onClick={() => handleExport("markdown")}
                  >
                    Exporter en Markdown
                  </button>
                  <button
                    className="secondary"
                    onClick={() => handleExport("json")}
                  >
                    Exporter en JSON
                  </button>
                </div>
              </section>
            </>
          )}
          {page === "prepare" && (
            <>
              <div className="pageHeading">
                <div>
                  <p className="eyebrow">PRÉPARATION</p>
                  <h1>Nouvel entretien</h1>
                  <p>Choisissez comment commencer votre transcription.</p>
                </div>
                <button
                  className="secondary"
                  onClick={() => setPage("library")}
                >
                  Retour
                </button>
              </div>
              <section className="settingsPanel">
                <h2>Source audio</h2>
                <label className="field">
                  Mode de capture
                  <select
                    value={source}
                    onChange={(event) => setSource(event.target.value)}
                  >
                    <option value="microphone">
                      Enregistrer avec le microphone
                    </option>
                    <option value="file">Importer un fichier audio</option>
                  </select>
                </label>
                {source === "file" ? (
                  <>
                    <label className="field">
                      Titre de l’entretien (optionnel)
                      <input
                        type="text"
                        value={prepareTitle}
                        onChange={(event) =>
                          setPrepareTitle(event.target.value)
                        }
                        placeholder="Sans titre reprend le nom du fichier"
                      />
                    </label>

                    <button
                      className="primary"
                      disabled={prepareBusy}
                      onClick={importAndTranscribe}
                    >
                      {prepareBusy
                        ? "Transcription en cours…"
                        : "Choisir un fichier audio"}
                    </button>
                  </>
                ) : (
                  <div className="notice" role="status">
                    La capture microphone n’est pas encore disponible dans cette
                    version. Aucun enregistrement n’est lancé.
                  </div>
                )}
              </section>
            </>
          )}
          {page === "settings" && (
            <>
              <div className="pageHeading">
                <div>
                  <p className="eyebrow">PRÉFÉRENCES</p>
                  <h1>Réglages</h1>
                  <p>Un espace de lecture adapté à votre confort.</p>
                </div>
              </div>
              <section className="settingsPanel">
                <h2>Apparence</h2>
                <label className="field">
                  Thème
                  <select
                    value={theme}
                    onChange={(event) => setTheme(event.target.value)}
                  >
                    <option value="system">Suivre le système</option>
                    <option value="light">Clair</option>
                    <option value="dark">Sombre</option>
                  </select>
                </label>
                <p className="muted">
                  Ce choix s’applique à la session en cours.
                </p>
              </section>
              <section className="settingsPanel">
                <h2>Modèles et stockage</h2>
                {modelError && (
                  <div className="notice" role="status">
                    {modelError}
                  </div>
                )}
                {modelStatus?.state === "Ready" && (
                  <>
                    <p>
                      <strong>{modelStatus.name}</strong>
                    </p>
                    <p>
                      Inclus dans l’application · {modelStatus.size_mb} Mo ·
                      Prêt à transcrire hors connexion.
                    </p>
                  </>
                )}
                {!modelStatus && !modelError && (
                  <p role="status">Vérification du modèle intégré…</p>
                )}
                <p className="muted">
                  Aucun téléchargement à effectuer. Le modèle est fourni avec
                  l’installation ; vos fichiers audio restent sur cet appareil.
                </p>
              </section>
            </>
          )}
        </main>
      </div>
    </div>
  );
}
