import { afterEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import App from "./App";

afterEach(() => {
  clearMocks();
});

describe("App", () => {
  it("opens the library without pretending to contain saved interviews", () => {
    render(<App />);
    expect(
      screen.getByRole("heading", { level: 1, name: "Mes entretiens" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Aucun entretien enregistré")).toBeInTheDocument();
  });
  it("hides timestamps without removing the transcript", () => {
    render(<App />);
    fireEvent.click(
      screen.getAllByRole("button", { name: /Découvrir l’éditeur/ })[0],
    );
    expect(screen.getByText("00:18")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("checkbox", { name: "Horodatages" }));
    expect(screen.queryByText("00:18")).not.toBeInTheDocument();
    expect(
      screen.getByText(
        "Qu’est-ce qui vous a le plus marqué dans cette expérience ?",
      ),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("checkbox", { name: "Horodatages" }));
    expect(screen.getByText("00:18")).toBeInTheDocument();
  });
  it("explains that capture is unavailable before any recording", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Nouvel entretien/ }));
    expect(screen.getByRole("status")).toHaveTextContent(
      "Aucun enregistrement n’est lancé",
    );
  });

  it("lists real interviews from the backend instead of the empty state", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_interviews") {
        return [
          {
            id: 1,
            title: "Entretien recruteur",
            language: "fr",
            mode: "posteriori",
            audio_path: "/audio/1.wav",
            status: "transcribed",
            error_message: null,
            created_at: "0",
            updated_at: "0",
          },
        ];
      }
      throw new Error(`unexpected command: ${cmd}`);
    });

    render(<App />);

    expect(
      await screen.findByText("Entretien recruteur — transcribed"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Aucun entretien enregistré"),
    ).not.toBeInTheDocument();
  });

  it("shows the bundled Turbo model without any download action", async () => {
    const commands: string[] = [];
    mockIPC((cmd) => {
      commands.push(cmd);
      if (cmd === "list_interviews") return [];
      if (cmd === "ensure_whisper_model")
        return {
          state: "Ready",
          path: "/resources/models/ggml-large-v3-turbo-q5_0.bin",
          name: "Whisper Large v3 Turbo (Q5_0)",
          size_mb: 575,
        };
      throw new Error(`unexpected command: ${cmd}`);
    });
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Réglages" }));
    expect(
      await screen.findByText("Whisper Large v3 Turbo (Q5_0)"),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Télécharger/ }),
    ).not.toBeInTheDocument();
    expect(commands).not.toContain("download_whisper_model");
  });

  it("imports and transcribes a file, then shows the real segments", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [];
        case "ensure_whisper_model":
          return {
            state: "Ready",
            path: "/models/ggml-large-v3-turbo-q5_0.bin",
          };
        case "plugin:dialog|open":
          return "/home/user/entretien.wav";
        case "import_interview":
          return {
            id: 7,
            title: "entretien.wav",
            language: null,
            mode: "posteriori",
            audio_path: "/audio/7.wav",
            status: "imported",
            error_message: null,
            created_at: "0",
            updated_at: "0",
          };
        case "transcribe_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: [
              {
                id: 1,
                interview_id: 7,
                label: "Intervenant 1",
                color: "#3156a3",
                display_name: null,
              },
            ],
            segments: [
              {
                id: 1,
                interview_id: 7,
                speaker_id: 1,
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour et merci d’être venu.",
                current_text: "Bonjour et merci d’être venu.",
                confidence: 0.9,
                status: "raw",
              },
            ],
          };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Nouvel entretien/ }));
    fireEvent.change(
      screen.getByRole("combobox", { name: "Mode de capture" }),
      {
        target: { value: "file" },
      },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Choisir un fichier audio" }),
    );

    expect(
      await screen.findByText("Bonjour et merci d’être venu."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { level: 1, name: "entretien.wav" }),
    ).toBeInTheDocument();
  });

  it("cleans a segment and shows the removed hesitation struck through", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [];
        case "ensure_whisper_model":
          return { state: "Ready", path: "/models/model.bin" };
        case "plugin:dialog|open":
          return "/home/user/entretien.wav";
        case "import_interview":
          return {
            id: 7,
            title: "entretien.wav",
            language: null,
            mode: "posteriori",
            audio_path: "/audio/7.wav",
            status: "imported",
            error_message: null,
            created_at: "0",
            updated_at: "0",
          };
        case "transcribe_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: [
              {
                id: 1,
                interview_id: 7,
                label: "Intervenant 1",
                color: "#3156a3",
                display_name: null,
              },
            ],
            segments: [
              {
                id: 1,
                interview_id: 7,
                speaker_id: 1,
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Alors, euh, je pense.",
                current_text: "Alors, euh, je pense.",
                confidence: 0.9,
                status: "raw",
              },
            ],
          };
        case "apply_segment_cleanup":
          return {
            segment: {
              id: 1,
              interview_id: 7,
              speaker_id: 1,
              start_ms: 0,
              end_ms: 1000,
              raw_text: "Alors, euh, je pense.",
              current_text: "Alors, je pense.",
              confidence: 0.9,
              status: "raw",
            },
            outcome: {
              cleaned_text: "Alors, je pense.",
              parts: [
                { kept: true, text: "Alors", reason: null },
                { kept: true, text: ", ", reason: null },
                { kept: false, text: "euh", reason: "hesitation" },
                { kept: true, text: "je", reason: null },
                { kept: true, text: " ", reason: null },
                { kept: true, text: "pense", reason: null },
                { kept: true, text: ".", reason: null },
              ],
            },
          };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Nouvel entretien/ }));
    fireEvent.change(
      screen.getByRole("combobox", { name: "Mode de capture" }),
      { target: { value: "file" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Choisir un fichier audio" }),
    );
    await screen.findByText("Alors, euh, je pense.");

    fireEvent.click(screen.getByRole("button", { name: "Nettoyer" }));
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Version nettoyée" }),
    );

    expect(await screen.findByText("euh")).toHaveClass("removedSpan");
    expect(screen.getByRole("button", { name: "Annuler" })).toBeEnabled();
  });

  it("undoes the last edit on a segment", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [
            {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
          ];
        case "get_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: [],
            segments: [
              {
                id: 1,
                interview_id: 7,
                speaker_id: null,
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Alors, euh, je pense.",
                current_text: "Alors, euh, je pense.",
                confidence: 0.9,
                status: "raw",
              },
            ],
          };
        case "apply_segment_cleanup":
          return {
            segment: {
              id: 1,
              interview_id: 7,
              speaker_id: null,
              start_ms: 0,
              end_ms: 1000,
              raw_text: "Alors, euh, je pense.",
              current_text: "Alors, je pense.",
              confidence: 0.9,
              status: "raw",
            },
            outcome: { cleaned_text: "Alors, je pense.", parts: [] },
          };
        case "undo_segment_edit":
          return {
            id: 1,
            interview_id: 7,
            speaker_id: null,
            start_ms: 0,
            end_ms: 1000,
            raw_text: "Alors, euh, je pense.",
            current_text: "Alors, euh, je pense.",
            confidence: 0.9,
            status: "raw",
          };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(
      await screen.findByRole("button", { name: /entretien.wav/ }),
    );
    const cleanupButton = await screen.findByRole("button", {
      name: "Nettoyer",
    });
    fireEvent.click(cleanupButton);

    const undoButton = await screen.findByRole("button", { name: "Annuler" });
    expect(undoButton).toBeEnabled();

    fireEvent.click(undoButton);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Annuler" })).toBeDisabled(),
    );
  });

  it("saves a manual edit to a segment", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [
            {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
          ];
        case "get_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: [],
            segments: [
              {
                id: 1,
                interview_id: 7,
                speaker_id: null,
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour.",
                current_text: "Bonjour.",
                confidence: 0.9,
                status: "raw",
              },
            ],
          };
        case "save_segment_edit":
          return {
            id: 1,
            interview_id: 7,
            speaker_id: null,
            start_ms: 0,
            end_ms: 1000,
            raw_text: "Bonjour.",
            current_text: "Salut !",
            confidence: 0.9,
            status: "raw",
          };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(
      await screen.findByRole("button", { name: /entretien.wav/ }),
    );
    fireEvent.click(await screen.findByRole("button", { name: "Modifier" }));
    fireEvent.change(screen.getByRole("textbox"), {
      target: { value: "Salut !" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    fireEvent.click(
      await screen.findByRole("checkbox", { name: "Version nettoyée" }),
    );
    expect(await screen.findByText("Salut !")).toBeInTheDocument();
  });

  it("renames a speaker without inventing one automatically", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [
            {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
          ];
        case "get_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: [
              {
                id: 1,
                interview_id: 7,
                label: "Intervenant 1",
                color: "#3156a3",
                display_name: null,
              },
            ],
            segments: [],
          };
        case "rename_speaker":
          return {
            id: 1,
            interview_id: 7,
            label: "Intervenant 1",
            color: "#3156a3",
            display_name: "Marie",
          };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(
      await screen.findByRole("button", { name: /entretien.wav/ }),
    );
    fireEvent.click(await screen.findByRole("button", { name: "Renommer" }));
    fireEvent.change(screen.getByPlaceholderText("Intervenant 1"), {
      target: { value: "Marie" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Marie")).toBeInTheDocument();
    expect(screen.queryByText("Intervenant 1")).not.toBeInTheDocument();
  });

  it("merges a speaker into another and refreshes the segment list", async () => {
    let merged = false;
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [
            {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
          ];
        case "get_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: merged
              ? [
                  {
                    id: 1,
                    interview_id: 7,
                    label: "Intervenant 1",
                    color: "#3156a3",
                    display_name: null,
                  },
                ]
              : [
                  {
                    id: 1,
                    interview_id: 7,
                    label: "Intervenant 1",
                    color: "#3156a3",
                    display_name: null,
                  },
                  {
                    id: 2,
                    interview_id: 7,
                    label: "Intervenant 2",
                    color: "#a33131",
                    display_name: null,
                  },
                ],
            segments: [
              {
                id: 1,
                interview_id: 7,
                speaker_id: 1,
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour.",
                current_text: "Bonjour.",
                confidence: 0.9,
                status: "raw",
              },
            ],
          };
        case "merge_speakers":
          merged = true;
          return [
            {
              id: 1,
              interview_id: 7,
              label: "Intervenant 1",
              color: "#3156a3",
              display_name: null,
            },
          ];
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(
      await screen.findByRole("button", { name: /entretien.wav/ }),
    );
    await screen.findByText("Intervenant 2");

    fireEvent.change(
      screen.getByRole("combobox", { name: /Fusionner Intervenant 1 avec/ }),
      { target: { value: "2" } },
    );
    fireEvent.click(screen.getAllByRole("button", { name: "Fusionner" })[0]);

    await waitFor(() =>
      expect(screen.queryByText("Intervenant 2")).not.toBeInTheDocument(),
    );
  });

  it("reassigns a segment to a different speaker", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [
            {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
          ];
        case "get_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: [
              {
                id: 1,
                interview_id: 7,
                label: "Intervenant 1",
                color: "#3156a3",
                display_name: null,
              },
              {
                id: 2,
                interview_id: 7,
                label: "Intervenant 2",
                color: "#a33131",
                display_name: null,
              },
            ],
            segments: [
              {
                id: 1,
                interview_id: 7,
                speaker_id: 1,
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour.",
                current_text: "Bonjour.",
                confidence: 0.9,
                status: "raw",
              },
            ],
          };
        case "reassign_segment_speaker":
          return {
            id: 1,
            interview_id: 7,
            speaker_id: 2,
            start_ms: 0,
            end_ms: 1000,
            raw_text: "Bonjour.",
            current_text: "Bonjour.",
            confidence: 0.9,
            status: "raw",
          };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(
      await screen.findByRole("button", { name: /entretien.wav/ }),
    );
    const segmentSpeaker = await screen.findByRole("combobox", {
      name: "Locuteur du segment",
    });
    fireEvent.change(segmentSpeaker, { target: { value: "2" } });

    await waitFor(() =>
      expect((segmentSpeaker as HTMLSelectElement).value).toBe("2"),
    );
  });

  it("flags a low-confidence speaker attribution as uncertain", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [
            {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
          ];
        case "get_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: [
              {
                id: 1,
                interview_id: 7,
                label: "Intervenant 1",
                color: "#3156a3",
                display_name: null,
              },
            ],
            segments: [
              {
                id: 1,
                interview_id: 7,
                speaker_id: 1,
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour.",
                current_text: "Bonjour.",
                confidence: 0.9,
                status: "uncertain",
              },
            ],
          };
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(
      await screen.findByRole("button", { name: /entretien.wav/ }),
    );
    expect(await screen.findByText("incertain")).toBeInTheDocument();
  });
});

describe("model prerequisites", () => {
  it("does not download or import automatically when the model is missing", async () => {
    const commands: string[] = [];
    mockIPC((cmd) => {
      commands.push(cmd);
      if (cmd === "list_interviews") return [];
      if (cmd === "ensure_whisper_model")
        throw new Error(
          "Modèle intégré absent : réinstallez le paquet complet.",
        );
      throw new Error(`unexpected command: ${cmd}`);
    });
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Nouvel entretien/ }));
    fireEvent.change(
      screen.getByRole("combobox", { name: "Mode de capture" }),
      { target: { value: "file" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Choisir un fichier audio" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Modèle intégré absent",
    );
    expect(commands).not.toContain("download_whisper_model");
    expect(commands).not.toContain("import_interview");
    expect(commands).not.toContain("plugin:dialog|open");
  });

  it("reports file picker errors and allows another attempt", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_interviews") return [];
      if (cmd === "ensure_whisper_model")
        return { state: "Ready", path: "/models/model.bin" };
      if (cmd === "plugin:dialog|open")
        throw new Error("Sélection indisponible");
      throw new Error(`unexpected command: ${cmd}`);
    });
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Nouvel entretien/ }));
    fireEvent.change(
      screen.getByRole("combobox", { name: "Mode de capture" }),
      { target: { value: "file" } },
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Choisir un fichier audio" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Sélection indisponible",
    );
    expect(
      screen.getByRole("button", { name: "Choisir un fichier audio" }),
    ).toBeEnabled();
  });

  it("disables the DOC export button when LibreOffice is unavailable", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_interviews":
          return [
            {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
          ];
        case "get_interview":
          return {
            interview: {
              id: 7,
              title: "entretien.wav",
              language: null,
              mode: "posteriori",
              audio_path: "/audio/7.wav",
              status: "transcribed",
              error_message: null,
              created_at: "0",
              updated_at: "0",
            },
            speakers: [],
            segments: [],
          };
        case "check_doc_export_available":
          return false;
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    render(<App />);
    fireEvent.click(
      await screen.findByRole("button", { name: /entretien.wav/ }),
    );
    expect(
      await screen.findByRole("button", { name: "Exporter en DOC" }),
    ).toBeDisabled();
    expect(
      screen.getByText(/LibreOffice n’est pas installé/),
    ).toBeInTheDocument();
  });
});
