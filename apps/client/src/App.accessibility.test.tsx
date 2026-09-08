import { afterEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import axe from "axe-core";
import App from "./App";

afterEach(() => {
  clearMocks();
});

async function expectNoAutomatedViolations(container: HTMLElement) {
  const result = await axe.run(container, {
    rules: {
      // jsdom has no layout or rendered color information. Contrast remains
      // part of the manual light/dark visual protocol.
      "color-contrast": { enabled: false },
    },
  });
  expect(
    result.violations.map((violation) => ({
      id: violation.id,
      impact: violation.impact,
      targets: violation.nodes.flatMap((node) => node.target),
    })),
  ).toEqual([]);
}

describe("automated accessibility", () => {
  it("finds no detectable violation in the empty library", async () => {
    mockIPC((cmd) => {
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten" || cmd === "client_log") return null;
      if (cmd === "list_interviews") return [];
      throw new Error(`unexpected command: ${cmd}`);
    });

    const { container } = render(<App />);

    await screen.findByText("Aucun entretien enregistré");
    await expectNoAutomatedViolations(container);
  });

  it("finds no detectable violation in the destructive confirmation", async () => {
    mockIPC((cmd) => {
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten" || cmd === "client_log") return null;
      if (cmd === "list_interviews")
        return [
          {
            id: 8,
            title: "Entretien accessible",
            language: "fr",
            mode: "posteriori",
            audio_path: "/audio/8.wav",
            status: "transcribed",
            error_message: null,
            created_at: "0",
            updated_at: "0",
          },
        ];
      throw new Error(`unexpected command: ${cmd}`);
    });

    const { container } = render(<App />);
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Supprimer Entretien accessible",
      }),
    );
    await screen.findByRole("dialog", { name: "Supprimer cet entretien ?" });

    await expectNoAutomatedViolations(container);
  });

  it("finds no detectable violation while recording", async () => {
    mockIPC((cmd) => {
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten" || cmd === "client_log") return null;
      if (cmd === "list_interviews") return [];
      if (cmd === "list_input_devices") return ["Microphone de test"];
      if (cmd === "start_recording")
        return {
          id: 9,
          title: "Session accessible",
          language: null,
          mode: "realtime",
          audio_path: "/audio/9.wav",
          status: "transcribing",
          error_message: null,
          created_at: "0",
          updated_at: "0",
        };
      throw new Error(`unexpected command: ${cmd}`);
    });

    const { container } = render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Nouvel entretien/ }));
    await screen.findByRole("option", { name: "Microphone de test" });
    fireEvent.click(
      screen.getByRole("button", { name: "Demarrer l’enregistrement" }),
    );
    await screen.findByRole("status", { name: "Enregistrement en cours" });

    await expectNoAutomatedViolations(container);
  });

  it("finds no detectable violation in a long interview error state", async () => {
    const interview = {
      id: 10,
      title: "Entretien long",
      language: "fr",
      mode: "posteriori",
      audio_path: "/audio/10.wav",
      status: "error",
      error_message: "Le traitement local a ete interrompu",
      created_at: "0",
      updated_at: "0",
    } as const;
    mockIPC((cmd) => {
      if (cmd === "plugin:event|listen") return 1;
      if (cmd === "plugin:event|unlisten" || cmd === "client_log") return null;
      if (cmd === "list_interviews") return [interview];
      if (cmd === "check_doc_export_available") return false;
      if (cmd === "get_interview")
        return {
          interview,
          speakers: [
            {
              id: 1,
              interview_id: 10,
              label: "Intervenant 1",
              color: "#3156a3",
              display_name: null,
            },
          ],
          segments: Array.from({ length: 40 }, (_, index) => ({
            id: index + 1,
            interview_id: 10,
            speaker_id: 1,
            start_ms: index * 2_000,
            end_ms: index * 2_000 + 1_500,
            raw_text: `Segment public synthetique ${index + 1}`,
            current_text: `Segment public synthetique ${index + 1}`,
            confidence: 0.9,
            status: "raw",
          })),
        };
      throw new Error(`unexpected command: ${cmd}`);
    });

    const { container } = render(<App />);
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Entretien long — error",
      }),
    );
    await screen.findByRole("heading", { level: 1, name: "Entretien long" });
    await screen.findByText(/Le traitement local a ete interrompu/);

    await expectNoAutomatedViolations(container);
  }, 15_000);
});
