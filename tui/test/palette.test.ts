import { afterEach, expect, test } from "bun:test";
import { palettes, setTheme, syntax, terminalPalette, theme } from "../src/theme.ts";

afterEach(() => setTheme("ink"));

const channels = (hex: string) =>
  [1, 3, 5].map((at) => Number.parseInt(hex.slice(at, at + 2), 16) / 255) as [number, number, number];
const light = (hex: string) => {
  const [r, g, b] = channels(hex).map((x) => (x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4)) as [
    number,
    number,
    number,
  ];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
/** The contrast of two colors, as the rule of WCAG counts it. */
const contrast = (a: string, b: string) => {
  const [high, low] = [light(a), light(b)].sort((x, y) => y - x) as [number, number];
  return (high + 0.05) / (low + 0.05);
};
/** The hue in degrees and the saturation of a color, as HSL gives them. */
const hsl = (hex: string) => {
  const [r, g, b] = channels(hex);
  const high = Math.max(r, g, b);
  const low = Math.min(r, g, b);
  const middle = (high + low) / 2;
  const span = high - low;
  if (span === 0) return { hue: 0, saturation: 0 };
  const saturation = span / (1 - Math.abs(2 * middle - 1));
  const hue = high === r ? ((g - b) / span) % 6 : high === g ? (b - r) / span + 2 : (r - g) / span + 4;
  return { hue: (hue * 60 + 360) % 360, saturation };
};
const apart = (a: number, b: number) => Math.min(Math.abs(a - b), 360 - Math.abs(a - b));
const ink = palettes.ink;
const colors = ["syntaxKeyword", "syntaxString", "syntaxNumber", "syntaxFunction", "syntaxType"] as const;

test("the text of Ink is no bright white, and the answer stays a step softer than the chrome", () => {
  for (const role of ["text", "prose"] as const) {
    expect(light(ink[role])).toBeLessThan(0.55);
    expect(Math.max(...channels(ink[role]))).toBeLessThan(0.8);
  }
  expect(light(ink.prose)).toBeLessThan(light(ink.text));
  expect(light(ink.muted)).toBeLessThan(light(ink.prose));
  expect(light(ink.faint)).toBeLessThan(light(ink.muted));
});

test("the syntax colors of Ink are no pastel: each shows its hue, and reads on each surface of the feed", () => {
  for (const role of colors) {
    expect(hsl(ink[role]).saturation).toBeGreaterThanOrEqual(0.35);
    for (const surface of [ink.background, ink.panel, ink.selected])
      expect(contrast(ink[role], surface)).toBeGreaterThanOrEqual(4.5);
  }
  for (const surface of [ink.background, ink.panel, ink.selected])
    expect(contrast(ink.syntaxComment, surface)).toBeGreaterThanOrEqual(4.3);
});

test("the syntax colors of Ink keep five hues apart from each other and from the accent of the operator", () => {
  const hues = [...colors.map((role) => hsl(ink[role]).hue), hsl(ink.accent).hue];
  for (const [at, hue] of hues.entries())
    for (const other of hues.slice(at + 1)) expect(apart(hue, other)).toBeGreaterThanOrEqual(25);
});

test("Python takes its colors from the groups of the parser, and an operator keeps the text color", () => {
  setTheme("ink");
  const style = syntax();
  const fg = (group: string) => style.getStyle(group)?.fg;
  expect(fg("function")?.equals(theme.syntaxFunction)).toBe(true);
  expect(fg("keyword")?.equals(theme.syntaxKeyword)).toBe(true);
  expect(fg("operator")?.equals(theme.text)).toBe(true);
  expect(fg("escape")?.equals(theme.syntaxNumber)).toBe(true);
  expect(fg("constant.builtin")?.equals(theme.syntaxNumber)).toBe(true);
  expect(fg("constructor")?.equals(theme.syntaxType)).toBe(true);
  expect(style.getStyle("comment")?.italic).toBe(true);
});

test("the theme Terminal keeps the color of a function on a dark terminal, and the color of its text on a light one", () => {
  expect(terminalPalette("#d0d0d0", "#101010")?.syntaxFunction).toBe(ink.syntaxFunction);
  const light = terminalPalette("#202020", "#fafafa");
  expect(light?.syntaxFunction).toBe(light?.text);
});
