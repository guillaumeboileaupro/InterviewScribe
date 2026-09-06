import { afterEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
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
});
