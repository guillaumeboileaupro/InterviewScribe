import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import App from "./App";

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
});
