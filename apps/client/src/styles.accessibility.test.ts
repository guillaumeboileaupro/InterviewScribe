import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const stylesheet = readFileSync(
  join(process.cwd(), "apps/client/src/styles.css"),
  "utf8",
);

function cssBlock(selector: string) {
  const start = stylesheet.indexOf(`${selector} {`);
  expect(start, `missing CSS block ${selector}`).toBeGreaterThanOrEqual(0);
  const openingBrace = stylesheet.indexOf("{", start);
  const closingBrace = stylesheet.indexOf("}", openingBrace);
  return stylesheet.slice(openingBrace + 1, closingBrace);
}

function themeTokens(selector: string) {
  return Object.fromEntries(
    [...cssBlock(selector).matchAll(/--([\w-]+):\s*(#[\da-f]+);/gi)].map(
      ([, name, value]) => [name, value],
    ),
  );
}

function luminance(hex: string) {
  const normalized =
    hex.length === 4
      ? `#${hex
          .slice(1)
          .split("")
          .map((digit) => digit.repeat(2))
          .join("")}`
      : hex;
  const channels = normalized
    .slice(1)
    .match(/.{2}/g)!
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) =>
      channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4,
    );
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrast(foreground: string, background: string) {
  const foregroundLuminance = luminance(foreground);
  const backgroundLuminance = luminance(background);
  return (
    (Math.max(foregroundLuminance, backgroundLuminance) + 0.05) /
    (Math.min(foregroundLuminance, backgroundLuminance) + 0.05)
  );
}

describe("theme contrast", () => {
  const themes = [
    ["light", themeTokens(".app")],
    ["dark", themeTokens('.app[data-theme="dark"]')],
    ["system dark", themeTokens('.app[data-theme="system"]')],
  ] as const;

  it.each(themes)("keeps %s text tokens above WCAG AA", (_name, tokens) => {
    for (const foreground of ["text", "muted", "accent-text", "second"]) {
      for (const background of ["bg", "surface"]) {
        expect(
          contrast(tokens[foreground], tokens[background]),
          `${foreground} on ${background}`,
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it("keeps fixed action colors above WCAG AA", () => {
    const pairs = [
      ["#ffffff", "#3156a3"],
      ["#a12b2b", "#ffffff"],
      ["#ffffff", "#a12b2b"],
      ["#ffabab", "#191c23"],
    ];

    for (const [foreground, background] of pairs) {
      expect(contrast(foreground, background)).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("uses the audited dark palette when the system prefers dark mode", () => {
    expect(themeTokens('.app[data-theme="system"]')).toEqual(
      themeTokens('.app[data-theme="dark"]'),
    );
  });
});
