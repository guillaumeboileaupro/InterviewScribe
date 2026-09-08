import { useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import {
  applySegmentCleanup,
  checkDocExportAvailable,
  createSpeaker,
  deleteInterview,
  ensureWhisperModel,
  exportInterview,
  exportInterviewDoc,
  getInterview,
  importInterview,
  listAvailableModels,
  listInputDevices,
  listInterviews,
  mergeSpeakers,
  pauseRecording,
  readRecentLogs,
  reassignSegmentSpeaker,
  renameSpeaker,
  resumeRecording,
  saveSegmentEdit,
  startRecording,
  stopRecording,
  transcribeInterview,
  undoSegmentEdit,
  type DiffPart,
  type ExportFormat,
  type Interview,
  type InterviewDetail,
  type ModelOption,
  type ModelStatus,
  type Segment,
  type Speaker,
} from "./api";
import { breadcrumb, installGlobalErrorLogging } from "./diagnostics";

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
  const [expectedSpeakerCount, setExpectedSpeakerCount] = useState("");

  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null);
  const [modelError, setModelError] = useState<string | null>(null);

  const [showCleaned, setShowCleaned] = useState(false);
  const [cleanupDiffs, setCleanupDiffs] = useState<Record<number, DiffPart[]>>(
    {},
  );
  const [editingSegmentId, setEditingSegmentId] = useState<number | null>(null);
  const [draftText, setDraftText] = useState("");
  const [interviewError, setInterviewError] = useState<string | null>(null);
  const [docAvailable, setDocAvailable] = useState(false);
  const [deleteCandidate, setDeleteCandidate] = useState<Interview | null>(
    null,
  );
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const deleteCancelRef = useRef<HTMLButtonElement | null>(null);

  const [editingSpeakerId, setEditingSpeakerId] = useState<number | null>(null);
  const [speakerDraftName, setSpeakerDraftName] = useState("");
  const [mergeTarget, setMergeTarget] = useState<Record<number, string>>({});

  const [availableModels, setAvailableModels] = useState<ModelOption[]>([]);
  const [selectedModelId, setSelectedModelId] = useState("");

  const [inputDevices, setInputDevices] = useState<string[]>([]);
  const [selectedDevice, setSelectedDevice] = useState("");
  const [recordingStatus, setRecordingStatus] = useState<
    "idle" | "recording" | "paused"
  >("idle");
  const [recordingInterviewId, setRecordingInterviewId] = useState<
    number | null
  >(null);
  const [recordingSegments, setRecordingSegments] = useState<Segment[]>([]);
  const [recordingLevel, setRecordingLevel] = useState(0);
  const [recordingSeconds, setRecordingSeconds] = useState(0);
  const [recordingError, setRecordingError] = useState<string | null>(null);
  const recordingInterviewIdRef = useRef<number | null>(null);

  const [diagnosticsText, setDiagnosticsText] = useState("");
  const [diagnosticsCopied, setDiagnosticsCopied] = useState(false);

  const refreshInterviews = () => {
    listInterviews()
      .then(setInterviews)
      .catch(() => setInterviews([]));
  };

  useEffect(() => {
    refreshInterviews();
  }, []);

  useEffect(() => {
    installGlobalErrorLogging();
  }, []);

  useEffect(() => {
    if (!deleteCandidate) return;
    deleteCancelRef.current?.focus();
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !deleteBusy) {
        setDeleteCandidate(null);
        setDeleteError(null);
      }
    };
    document.addEventListener("keydown", closeOnEscape);
    return () => document.removeEventListener("keydown", closeOnEscape);
  }, [deleteCandidate, deleteBusy]);

  useEffect(() => {
    if (page === "settings") {
      ensureWhisperModel()
        .then(setModelStatus)
        .catch((err) => setModelError(String(err)));
      readRecentLogs()
        .then((text) => {
          setDiagnosticsText(text);
          setDiagnosticsCopied(false);
        })
        .catch(() => setDiagnosticsText(""));
    }
    if (page === "interview") {
      checkDocExportAvailable()
        .then(setDocAvailable)
        .catch(() => setDocAvailable(false));
    }
    if (page === "prepare") {
      listInputDevices()
        .then(setInputDevices)
        .catch(() => setInputDevices([]));
      listAvailableModels()
        .then((models) => {
          setAvailableModels(models);
          setSelectedModelId((current) => current || models[0]?.id || "");
        })
        .catch(() => setAvailableModels([]));
    }
  }, [page]);

  useEffect(() => {
    recordingInterviewIdRef.current = recordingInterviewId;
  }, [recordingInterviewId]);

  useEffect(() => {
    const unlistenLevel = listen<number>("recording-level", (event) => {
      setRecordingLevel(event.payload);
    });
    const unlistenSegments = listen<number>("segments-updated", (event) => {
      if (event.payload !== recordingInterviewIdRef.current) return;
      getInterview(event.payload)
        .then((detail) => setRecordingSegments(detail.segments))
        .catch(() => {});
    });
    const unlistenError = listen<string>("recording-error", (event) => {
      setRecordingError(event.payload);
    });
    return () => {
      // Fire-and-forget: nothing meaningful to do if unregistering a
      // listener fails during teardown (e.g. the window is already closing).
      unlistenLevel.then((unlisten) => unlisten()).catch(() => {});
      unlistenSegments.then((unlisten) => unlisten()).catch(() => {});
      unlistenError.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    if (recordingStatus !== "recording") return;
    const interval = setInterval(() => {
      setRecordingSeconds((seconds) => seconds + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [recordingStatus]);

  const openInterview = (interviewId: number) => {
    getInterview(interviewId)
      .then((detail) => {
        setCurrentInterview(detail);
        setPage("interview");
      })
      .catch((err) => setPrepareError(String(err)));
  };

  const requestInterviewDeletion = (interview: Interview) => {
    setDeleteError(null);
    setDeleteCandidate(interview);
  };

  const confirmInterviewDeletion = async () => {
    if (!deleteCandidate) return;
    setDeleteBusy(true);
    setDeleteError(null);
    try {
      await deleteInterview(deleteCandidate.id);
      setInterviews((items) =>
        items.filter((item) => item.id !== deleteCandidate.id),
      );
      setDeleteCandidate(null);
      refreshInterviews();
    } catch (err) {
      setDeleteError(String(err));
    } finally {
      setDeleteBusy(false);
    }
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
      // On Android, `selected` is a `content://` URI, not a path - its last
      // segment is an opaque, percent-encoded document id, not a filename.
      // A generic, honestly-labeled title beats showing that garbled id.
      const fileName = /^[a-z][a-z0-9+.-]*:\/\//i.test(selected)
        ? `Entretien du ${new Date().toLocaleDateString()}`
        : (selected.split(/[/\\]/).pop() ?? selected);
      const interview = await importInterview(
        prepareTitle.trim() || fileName,
        selected,
      );
      refreshInterviews();
      const parsedCount = Number.parseInt(expectedSpeakerCount, 10);
      const detail = await transcribeInterview(
        interview.id,
        Number.isFinite(parsedCount) && parsedCount > 0
          ? parsedCount
          : undefined,
        selectedModelId || undefined,
      );
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
        showCleaned,
        destination,
      );
    } catch (err) {
      setPrepareError(String(err));
    }
  };

  const handleExportDoc = async () => {
    if (!currentInterview) return;
    const destination = await save({
      defaultPath: `${currentInterview.interview.title}.doc`,
    });
    if (!destination) return;
    try {
      await exportInterviewDoc(
        currentInterview.interview.id,
        timestamps,
        showCleaned,
        destination,
      );
    } catch (err) {
      setPrepareError(String(err));
    }
  };

  const updateSegmentInPlace = (updated: Segment) => {
    setCurrentInterview((detail) =>
      detail
        ? {
            ...detail,
            segments: detail.segments.map((segment) =>
              segment.id === updated.id ? updated : segment,
            ),
          }
        : detail,
    );
  };

  const handleCleanup = async (segmentId: number) => {
    setInterviewError(null);
    try {
      const { segment, outcome } = await applySegmentCleanup(segmentId);
      updateSegmentInPlace(segment);
      setCleanupDiffs((diffs) => ({ ...diffs, [segmentId]: outcome.parts }));
    } catch (err) {
      setInterviewError(String(err));
    }
  };

  const handleUndo = async (segmentId: number) => {
    setInterviewError(null);
    try {
      const segment = await undoSegmentEdit(segmentId);
      updateSegmentInPlace(segment);
      setCleanupDiffs((diffs) => {
        const next = { ...diffs };
        delete next[segmentId];
        return next;
      });
    } catch (err) {
      setInterviewError(String(err));
    }
  };

  const startEditingSegment = (segment: Segment) => {
    setEditingSegmentId(segment.id);
    setDraftText(segment.current_text);
  };

  const cancelEditingSegment = () => {
    setEditingSegmentId(null);
    setDraftText("");
  };

  const saveEditingSegment = async () => {
    if (editingSegmentId === null) return;
    setInterviewError(null);
    try {
      const segment = await saveSegmentEdit(editingSegmentId, draftText);
      updateSegmentInPlace(segment);
      setCleanupDiffs((diffs) => {
        const next = { ...diffs };
        delete next[segment.id];
        return next;
      });
      cancelEditingSegment();
    } catch (err) {
      setInterviewError(String(err));
    }
  };

  const updateSpeakersInPlace = (speakers: Speaker[]) => {
    setCurrentInterview((detail) =>
      detail ? { ...detail, speakers } : detail,
    );
  };

  const startEditingSpeaker = (speaker: Speaker) => {
    setEditingSpeakerId(speaker.id);
    setSpeakerDraftName(speaker.display_name ?? "");
  };

  const saveEditingSpeaker = async () => {
    if (editingSpeakerId === null) return;
    setInterviewError(null);
    try {
      const speaker = await renameSpeaker(editingSpeakerId, speakerDraftName);
      updateSpeakersInPlace(
        (currentInterview?.speakers ?? []).map((candidate) =>
          candidate.id === speaker.id ? speaker : candidate,
        ),
      );
      setEditingSpeakerId(null);
      setSpeakerDraftName("");
    } catch (err) {
      setInterviewError(String(err));
    }
  };

  const handleMergeSpeakers = async (keepId: number) => {
    if (!currentInterview) return;
    const removeId = Number.parseInt(mergeTarget[keepId] ?? "", 10);
    if (!Number.isFinite(removeId)) return;
    setInterviewError(null);
    try {
      const speakers = await mergeSpeakers(
        currentInterview.interview.id,
        keepId,
        removeId,
      );
      updateSpeakersInPlace(speakers);
      const detail = await getInterview(currentInterview.interview.id);
      setCurrentInterview(detail);
      setMergeTarget((targets) => ({ ...targets, [keepId]: "" }));
    } catch (err) {
      setInterviewError(String(err));
    }
  };

  const handleAddSpeaker = async () => {
    if (!currentInterview) return;
    const nextIndex = currentInterview.speakers.length + 1;
    setInterviewError(null);
    try {
      const speaker = await createSpeaker(
        currentInterview.interview.id,
        `Intervenant ${nextIndex}`,
      );
      updateSpeakersInPlace([...currentInterview.speakers, speaker]);
    } catch (err) {
      setInterviewError(String(err));
    }
  };

  const handleReassignSegment = async (
    segmentId: number,
    speakerId: number | null,
  ) => {
    setInterviewError(null);
    try {
      const segment = await reassignSegmentSpeaker(segmentId, speakerId);
      updateSegmentInPlace(segment);
    } catch (err) {
      setInterviewError(String(err));
    }
  };

  const handleStartRecording = async () => {
    breadcrumb('clic "Demarrer l\'enregistrement"');
    setRecordingError(null);
    try {
      const parsedCount = Number.parseInt(expectedSpeakerCount, 10);
      const interview = await startRecording(
        prepareTitle.trim() || "Session en direct",
        selectedDevice || undefined,
        Number.isFinite(parsedCount) && parsedCount > 0
          ? parsedCount
          : undefined,
        selectedModelId || undefined,
      );
      setRecordingInterviewId(interview.id);
      setRecordingSegments([]);
      setRecordingSeconds(0);
      setRecordingStatus("recording");
    } catch (err) {
      setRecordingError(String(err));
    }
  };

  const handlePauseRecording = async () => {
    breadcrumb('clic "Pause"');
    try {
      await pauseRecording();
      setRecordingStatus("paused");
    } catch (err) {
      setRecordingError(String(err));
    }
  };

  const handleResumeRecording = async () => {
    breadcrumb('clic "Reprendre"');
    try {
      await resumeRecording(selectedDevice || undefined);
      setRecordingStatus("recording");
    } catch (err) {
      setRecordingError(String(err));
    }
  };

  const handleStopRecording = async () => {
    breadcrumb('clic "Arreter et terminer"');
    try {
      const detail = await stopRecording();
      setRecordingStatus("idle");
      setRecordingInterviewId(null);
      setRecordingSegments([]);
      setRecordingSeconds(0);
      refreshInterviews();
      setCurrentInterview(detail);
      setPage("interview");
    } catch (err) {
      setRecordingError(String(err));
    }
  };

  const handleCopyDiagnostics = async () => {
    try {
      await navigator.clipboard.writeText(diagnosticsText);
    } catch {
      const textarea = document.createElement("textarea");
      textarea.value = diagnosticsText;
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.appendChild(textarea);
      textarea.select();
      try {
        document.execCommand("copy");
      } catch {
        // Nothing more we can do; the text is still visible on screen for
        // the user to select and copy by hand.
      }
      document.body.removeChild(textarea);
    }
    setDiagnosticsCopied(true);
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
                      <li className="interviewRow" key={interview.id}>
                        <button
                          className="textButton"
                          onClick={() => openInterview(interview.id)}
                        >
                          {interview.title} — {interview.status}
                        </button>
                        <button
                          className="dangerTextButton"
                          aria-label={`Supprimer ${interview.title}`}
                          onClick={() => requestInterviewDeletion(interview)}
                        >
                          Supprimer
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
          {deleteCandidate && (
            <div className="dialogBackdrop">
              <section
                className="confirmDialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="delete-dialog-title"
                aria-describedby="delete-dialog-description"
              >
                <p className="eyebrow">SUPPRESSION DÉFINITIVE</p>
                <h2 id="delete-dialog-title">Supprimer cet entretien ?</h2>
                <p id="delete-dialog-description">
                  « {deleteCandidate.title} » et sa copie audio privée seront
                  supprimés de cet appareil. Les exports enregistrés ailleurs ne
                  seront pas supprimés.
                </p>
                {deleteError && (
                  <div className="notice" role="alert">
                    Suppression impossible : {deleteError}
                  </div>
                )}
                <div className="buttonRow dialogActions">
                  <button
                    ref={deleteCancelRef}
                    className="secondary"
                    disabled={deleteBusy}
                    onClick={() => {
                      setDeleteCandidate(null);
                      setDeleteError(null);
                    }}
                  >
                    Conserver l’entretien
                  </button>
                  <button
                    className="dangerButton"
                    disabled={deleteBusy}
                    onClick={confirmInterviewDeletion}
                  >
                    {deleteBusy ? "Suppression…" : "Supprimer définitivement"}
                  </button>
                </div>
              </section>
            </div>
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
              {interviewError && (
                <div className="notice" role="alert">
                  {interviewError}
                </div>
              )}
              <section className="transcript">
                <div className="transcriptToolbar">
                  <strong>
                    {showCleaned ? "Version nettoyée" : "Transcription brute"}
                  </strong>
                  <label className="check">
                    <input
                      type="checkbox"
                      checked={timestamps}
                      onChange={(event) => setTimestamps(event.target.checked)}
                    />{" "}
                    Horodatages
                  </label>
                  <label className="check">
                    <input
                      type="checkbox"
                      checked={showCleaned}
                      onChange={(event) => setShowCleaned(event.target.checked)}
                    />{" "}
                    Version nettoyée
                  </label>
                </div>
                {currentInterview.segments.length === 0 ? (
                  <p className="muted">Aucun segment pour le moment.</p>
                ) : (
                  currentInterview.segments.map((segment) => {
                    const isEdited = segment.current_text !== segment.raw_text;
                    const diffParts = cleanupDiffs[segment.id];
                    return (
                      <article className="segment" key={segment.id}>
                        <div className="segmentMeta">
                          {timestamps && (
                            <time>{formatTimestamp(segment.start_ms)}</time>
                          )}
                          <select
                            className="speaker"
                            aria-label="Locuteur du segment"
                            value={segment.speaker_id ?? ""}
                            onChange={(event) =>
                              handleReassignSegment(
                                segment.id,
                                event.target.value
                                  ? Number(event.target.value)
                                  : null,
                              )
                            }
                          >
                            <option value="">Sans locuteur</option>
                            {currentInterview.speakers.map((candidate) => (
                              <option key={candidate.id} value={candidate.id}>
                                {candidate.display_name ?? candidate.label}
                              </option>
                            ))}
                          </select>
                          {segment.status === "uncertain" && (
                            <span
                              className="muted"
                              title="Confiance faible sur l'attribution du locuteur"
                            >
                              incertain
                            </span>
                          )}
                        </div>
                        {editingSegmentId === segment.id ? (
                          <>
                            <textarea
                              value={draftText}
                              onChange={(event) =>
                                setDraftText(event.target.value)
                              }
                              rows={3}
                              style={{ width: "100%" }}
                            />
                            <div className="buttonRow">
                              <button
                                className="primary"
                                onClick={saveEditingSegment}
                              >
                                Enregistrer
                              </button>
                              <button
                                className="secondary"
                                onClick={cancelEditingSegment}
                              >
                                Annuler la saisie
                              </button>
                            </div>
                          </>
                        ) : (
                          <>
                            {showCleaned && diffParts ? (
                              <p>
                                {diffParts.map((part, index) =>
                                  part.kept ? (
                                    <span key={index}>{part.text}</span>
                                  ) : (
                                    <del
                                      key={index}
                                      className="removedSpan"
                                      title={part.reason ?? undefined}
                                    >
                                      {part.text}
                                    </del>
                                  ),
                                )}
                              </p>
                            ) : (
                              <p>
                                {showCleaned
                                  ? segment.current_text
                                  : segment.raw_text}
                              </p>
                            )}
                            <div className="buttonRow">
                              <button
                                className="textButton"
                                onClick={() => startEditingSegment(segment)}
                              >
                                Modifier
                              </button>
                              <button
                                className="textButton"
                                onClick={() => handleCleanup(segment.id)}
                              >
                                Nettoyer
                              </button>
                              <button
                                className="textButton"
                                disabled={!isEdited}
                                onClick={() => handleUndo(segment.id)}
                              >
                                Annuler
                              </button>
                            </div>
                          </>
                        )}
                      </article>
                    );
                  })
                )}
              </section>
              <section className="settingsPanel">
                <h2>Locuteurs</h2>
                {currentInterview.speakers.map((speaker) => (
                  <div className="buttonRow" key={speaker.id}>
                    <span
                      className="speakerDot"
                      style={{ backgroundColor: speaker.color }}
                      aria-hidden="true"
                    />
                    {editingSpeakerId === speaker.id ? (
                      <>
                        <input
                          type="text"
                          value={speakerDraftName}
                          onChange={(event) =>
                            setSpeakerDraftName(event.target.value)
                          }
                          placeholder={speaker.label}
                        />
                        <button
                          className="primary"
                          onClick={saveEditingSpeaker}
                        >
                          Enregistrer
                        </button>
                        <button
                          className="secondary"
                          onClick={() => setEditingSpeakerId(null)}
                        >
                          Annuler
                        </button>
                      </>
                    ) : (
                      <>
                        <span>{speaker.display_name ?? speaker.label}</span>
                        <button
                          className="textButton"
                          onClick={() => startEditingSpeaker(speaker)}
                        >
                          Renommer
                        </button>
                        {currentInterview.speakers.length > 1 && (
                          <>
                            <select
                              aria-label={`Fusionner ${speaker.display_name ?? speaker.label} avec`}
                              value={mergeTarget[speaker.id] ?? ""}
                              onChange={(event) =>
                                setMergeTarget((targets) => ({
                                  ...targets,
                                  [speaker.id]: event.target.value,
                                }))
                              }
                            >
                              <option value="">Fusionner avec…</option>
                              {currentInterview.speakers
                                .filter((other) => other.id !== speaker.id)
                                .map((other) => (
                                  <option key={other.id} value={other.id}>
                                    {other.display_name ?? other.label}
                                  </option>
                                ))}
                            </select>
                            <button
                              className="textButton"
                              disabled={!mergeTarget[speaker.id]}
                              onClick={() => handleMergeSpeakers(speaker.id)}
                            >
                              Fusionner
                            </button>
                          </>
                        )}
                      </>
                    )}
                  </div>
                ))}
                <button className="secondary" onClick={handleAddSpeaker}>
                  + Ajouter un locuteur
                </button>
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
                  <button
                    className="secondary"
                    onClick={() => handleExport("srt")}
                  >
                    Exporter en SRT
                  </button>
                  <button
                    className="secondary"
                    onClick={() => handleExport("vtt")}
                  >
                    Exporter en VTT
                  </button>
                  <button
                    className="secondary"
                    onClick={() => handleExport("docx")}
                  >
                    Exporter en DOCX
                  </button>
                  <button
                    className="secondary"
                    onClick={() => handleExport("pdf")}
                  >
                    Exporter en PDF
                  </button>
                  <button
                    className="secondary"
                    disabled={!docAvailable}
                    onClick={handleExportDoc}
                  >
                    Exporter en DOC
                  </button>
                </div>
                {!docAvailable && (
                  <p className="muted">
                    Export DOC indisponible : LibreOffice n’est pas installé sur
                    cet appareil.
                  </p>
                )}
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
                <label className="field">
                  Modèle de transcription
                  <select
                    value={selectedModelId}
                    onChange={(event) => setSelectedModelId(event.target.value)}
                  >
                    {availableModels.map((model) => (
                      <option key={model.id} value={model.id}>
                        {model.name} · {model.size_mb} Mo
                      </option>
                    ))}
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
                    <label className="field">
                      Nombre de personnes (optionnel)
                      <input
                        type="number"
                        min={1}
                        value={expectedSpeakerCount}
                        onChange={(event) =>
                          setExpectedSpeakerCount(event.target.value)
                        }
                        placeholder="Estimation automatique si vide"
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
                  <>
                    <label className="field">
                      Titre de l’entretien (optionnel)
                      <input
                        type="text"
                        value={prepareTitle}
                        onChange={(event) =>
                          setPrepareTitle(event.target.value)
                        }
                        placeholder="Session en direct"
                        disabled={recordingStatus !== "idle"}
                      />
                    </label>
                    <label className="field">
                      Microphone
                      <select
                        value={selectedDevice}
                        onChange={(event) =>
                          setSelectedDevice(event.target.value)
                        }
                        disabled={recordingStatus !== "idle"}
                      >
                        <option value="">Peripherique par defaut</option>
                        {inputDevices.map((device) => (
                          <option key={device} value={device}>
                            {device}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="field">
                      Nombre de personnes (optionnel)
                      <input
                        type="number"
                        min={1}
                        value={expectedSpeakerCount}
                        onChange={(event) =>
                          setExpectedSpeakerCount(event.target.value)
                        }
                        placeholder="Estimation automatique si vide"
                        disabled={recordingStatus !== "idle"}
                      />
                    </label>

                    {recordingError && (
                      <div className="notice" role="alert">
                        {recordingError}
                      </div>
                    )}

                    {recordingStatus !== "idle" && (
                      <div
                        className="notice"
                        role="status"
                        aria-label={
                          recordingStatus === "recording"
                            ? "Enregistrement en cours"
                            : "Enregistrement en pause"
                        }
                      >
                        <strong>
                          {recordingStatus === "recording"
                            ? "● Enregistrement"
                            : "‖ Pause"}
                        </strong>{" "}
                        · {formatTimestamp(recordingSeconds * 1000)}
                        <div className="levelMeter" aria-hidden="true">
                          <div
                            className="levelMeterFill"
                            style={{
                              width: `${Math.min(100, Math.round(recordingLevel * 400))}%`,
                            }}
                          />
                        </div>
                      </div>
                    )}

                    <div className="buttonRow">
                      {recordingStatus === "idle" && (
                        <button
                          className="primary"
                          onClick={handleStartRecording}
                        >
                          Demarrer l’enregistrement
                        </button>
                      )}
                      {recordingStatus === "recording" && (
                        <button
                          className="secondary"
                          onClick={handlePauseRecording}
                        >
                          Pause
                        </button>
                      )}
                      {recordingStatus === "paused" && (
                        <button
                          className="secondary"
                          onClick={handleResumeRecording}
                        >
                          Reprendre
                        </button>
                      )}
                      {recordingStatus !== "idle" && (
                        <button
                          className="primary"
                          onClick={handleStopRecording}
                        >
                          Arreter et terminer
                        </button>
                      )}
                    </div>

                    {recordingStatus !== "idle" && (
                      <section className="transcript">
                        <div className="transcriptToolbar">
                          <strong>Transcription en direct</strong>
                        </div>
                        {recordingSegments.length === 0 ? (
                          <p className="muted">
                            En attente du premier segment stabilise…
                          </p>
                        ) : (
                          recordingSegments.map((segment) => (
                            <article className="segment" key={segment.id}>
                              <div className="segmentMeta">
                                <time>{formatTimestamp(segment.start_ms)}</time>
                                {segment.status === "uncertain" && (
                                  <span className="muted">incertain</span>
                                )}
                              </div>
                              <p>{segment.raw_text}</p>
                            </article>
                          ))
                        )}
                      </section>
                    )}
                  </>
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
              <section className="settingsPanel">
                <h2>Diagnostics</h2>
                <p className="muted">
                  Journal technique local (jamais l’audio ni le texte transcrit)
                  : utile pour signaler un problème. Rien n’est envoyé
                  automatiquement, ce n’est que sur cet écran.
                </p>
                <pre className="diagnosticsLog">
                  {diagnosticsText ||
                    "Aucun evenement journalise pour l’instant."}
                </pre>
                <button className="secondary" onClick={handleCopyDiagnostics}>
                  {diagnosticsCopied ? "Copié ✓" : "Copier les journaux"}
                </button>
              </section>
            </>
          )}
        </main>
      </div>
    </div>
  );
}
