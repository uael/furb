import { RGBA, SyntaxStyle, type ThemeMode } from "@opentui/core";
import { clock } from "./clock.ts";

/** The roles of a palette. Surfaces go from the background up: the panel of the sidebars and the composer, then the
 * raised surface of a dialog. Text goes from the text down: muted for what supports it, faint for what frames it. */
interface Palette {
  /** The surface of the feed. */
  background: string;
  /** The surface of the sidebars, the composer, a message, and a code block. */
  panel: string;
  /** The surface of a dialog, a hover card, and a list of suggestions. */
  raised: string;
  /** The row that the selection or the pointer stands on. */
  selected: string;
  border: string;
  text: string;
  /** The text of an answer, a step softer than the text of the chrome. */
  prose: string;
  muted: string;
  /** Line numbers, separators, and connectors. */
  faint: string;
  /** The operator: a message, a prompt, the selection, and what a click or a key reaches. */
  accent: string;
  /** Python: the input in Python and the words that it runs. */
  secondary: string;
  success: string;
  warning: string;
  danger: string;
  link: string;
  added: string;
  removed: string;
  /** The veil over the screen behind a dialog, with its alpha. */
  backdrop: string;
  syntaxKeyword: string;
  syntaxString: string;
  syntaxNumber: string;
  syntaxFunction: string;
  syntaxType: string;
  syntaxComment: string;
}

export const palettes = {
  ink: {
    background: "#16181d",
    panel: "#1b1e24",
    raised: "#22262e",
    selected: "#262c37",
    border: "#2c3039",
    text: "#c8cdd5",
    prose: "#bec4cc",
    muted: "#8f96a1",
    faint: "#70767f",
    accent: "#82a7d8",
    secondary: "#a99ccf",
    success: "#8fae8b",
    warning: "#d9aa62",
    danger: "#d7776c",
    link: "#82a7d8",
    added: "#1d2a22",
    removed: "#33211f",
    backdrop: "#0b0c0fb0",
    syntaxKeyword: "#b49ad0",
    syntaxString: "#a7b89a",
    syntaxNumber: "#c9a979",
    syntaxFunction: "#c8cdd5",
    syntaxType: "#93b2c4",
    syntaxComment: "#7d838d",
  },
  github: {
    background: "#0d1117",
    panel: "#161b22",
    raised: "#1c2330",
    selected: "#1c2d42",
    border: "#30363d",
    text: "#e6edf3",
    prose: "#c9d1d9",
    muted: "#8b949e",
    faint: "#6e7681",
    accent: "#58a6ff",
    secondary: "#bc8cff",
    success: "#3fb950",
    warning: "#d29922",
    danger: "#f85149",
    link: "#58a6ff",
    added: "#12261e",
    removed: "#33191c",
    backdrop: "#010409a0",
    syntaxKeyword: "#ff7b72",
    syntaxString: "#a5d6ff",
    syntaxNumber: "#79c0ff",
    syntaxFunction: "#d2a8ff",
    syntaxType: "#ffa657",
    syntaxComment: "#8b949e",
  },
  forest: {
    background: "#101817",
    panel: "#15211e",
    raised: "#1c2b26",
    selected: "#243a30",
    border: "#2c4238",
    text: "#e3e9df",
    prose: "#c9d1c4",
    muted: "#91a697",
    faint: "#66796b",
    accent: "#b7d89b",
    secondary: "#c7aee0",
    success: "#88cabe",
    warning: "#e5c28a",
    danger: "#eea79a",
    link: "#a5bdd4",
    added: "#1b3326",
    removed: "#392924",
    backdrop: "#060b0aa0",
    syntaxKeyword: "#88cabe",
    syntaxString: "#b7d89b",
    syntaxNumber: "#e5c28a",
    syntaxFunction: "#a5bdd4",
    syntaxType: "#a5bdd4",
    syntaxComment: "#7d9283",
  },
  paper: {
    background: "#f6f4ec",
    panel: "#ece9df",
    raised: "#fffdf6",
    selected: "#dbe5d2",
    border: "#c9cfbf",
    text: "#243329",
    prose: "#324237",
    muted: "#56685a",
    faint: "#86938a",
    accent: "#3c673c",
    secondary: "#7a4fb0",
    success: "#246e68",
    warning: "#896123",
    danger: "#ac4238",
    link: "#325f92",
    added: "#dcedd6",
    removed: "#f4d9d1",
    backdrop: "#24332930",
    syntaxKeyword: "#cf222e",
    syntaxString: "#0a3069",
    syntaxNumber: "#0550ae",
    syntaxFunction: "#8250df",
    syntaxType: "#953800",
    syntaxComment: "#6e7781",
  },
  midnight: {
    background: "#141722",
    panel: "#1b2030",
    raised: "#232a3d",
    selected: "#2e3955",
    border: "#36405a",
    text: "#e4e9f4",
    prose: "#c6cee0",
    muted: "#9daec7",
    faint: "#6d7a93",
    accent: "#bbc3ff",
    secondary: "#e2a8f0",
    success: "#81cfda",
    warning: "#e6c38b",
    danger: "#f09eac",
    link: "#99bfff",
    added: "#1c3136",
    removed: "#392934",
    backdrop: "#07080ea0",
    syntaxKeyword: "#81cfda",
    syntaxString: "#bbc3ff",
    syntaxNumber: "#e6c38b",
    syntaxFunction: "#99bfff",
    syntaxType: "#99bfff",
    syntaxComment: "#8190aa",
  },
  dusk: {
    background: "#1f1e1c",
    panel: "#262523",
    raised: "#2e2d2a",
    selected: "#35332f",
    border: "#3b3935",
    text: "#ecebe6",
    prose: "#cfcdc5",
    muted: "#9a978e",
    faint: "#6b6963",
    accent: "#d4835f",
    secondary: "#a9a4cc",
    success: "#93b58c",
    warning: "#e2b55c",
    danger: "#e07a6f",
    link: "#9db7d9",
    added: "#24301f",
    removed: "#3a2522",
    backdrop: "#0e0d0cb0",
    syntaxKeyword: "#d9a37e",
    syntaxString: "#a9c19d",
    syntaxNumber: "#e2b55c",
    syntaxFunction: "#ecebe6",
    syntaxType: "#a9a4cc",
    syntaxComment: "#8a867d",
  },
  day: {
    background: "#faf9f5",
    panel: "#f2f0e9",
    raised: "#ffffff",
    selected: "#e8e5db",
    border: "#e0ddd2",
    text: "#1f1e1c",
    prose: "#3d3b36",
    muted: "#6e6b63",
    faint: "#a29e94",
    accent: "#b85a3c",
    secondary: "#6b5fa8",
    success: "#4f7a4a",
    warning: "#9a6a12",
    danger: "#b4433a",
    link: "#3d64a0",
    added: "#e3eedc",
    removed: "#f5dfd9",
    backdrop: "#1f1e1c40",
    syntaxKeyword: "#9a4f33",
    syntaxString: "#4f7a4a",
    syntaxNumber: "#9a6a12",
    syntaxFunction: "#1f1e1c",
    syntaxType: "#6b5fa8",
    syntaxComment: "#8d897f",
  },
} satisfies Record<string, Palette>;
/** A theme by its name, or "own": the palette that the terminal gives, from its own colors. */
export type ThemeName = keyof typeof palettes | "own";
/** The tones of the threads, muted so that no tone reads as a state: each thread takes one by its name, on the bar of
 * its row in the sidebar, so that the eye finds it again. */
const threadTones = ["#6f8fb8", "#b08a5e", "#7f9c7b", "#9a7fa6", "#5f9ea2", "#b07b7b", "#8d8fb8", "#a39a62"];
/** The tone of a thread, the same for its name in each life, and the default color of the terminal when the view
 * shows no color. */
export function threadTone(name: string): RGBA {
  if (colorless.on) return RGBA.defaultForeground();
  let hash = 0;
  for (const char of name) hash = (hash * 31 + (char.codePointAt(0) ?? 0)) >>> 0;
  return RGBA.fromHex(threadTones[hash % threadTones.length] ?? "#6f8fb8");
}
/** The theme that a view shows when no choice and no terminal names one. */
export const defaultTheme: keyof typeof palettes = "ink";
/** The palette that the colors of the terminal give, once the terminal said them. */
export const own: { palette?: Palette } = {};
/** The palette of a theme. */
export function paletteOf(name: ThemeName): Palette {
  return name === "own" ? (own.palette ?? palettes[defaultTheme]) : palettes[name];
}
/** A color between two colors of six hex digits, at a share of the way from the first to the second. */
function blend(from: string, to: string, share: number): string {
  const channel = (hex: string, at: number) => Number.parseInt(hex.slice(1 + at * 2, 3 + at * 2), 16);
  return `#${[0, 1, 2]
    .map((at) => Math.round(channel(from, at) + (channel(to, at) - channel(from, at)) * share))
    .map((value) => value.toString(16).padStart(2, "0"))
    .join("")}`;
}
/** Whether a color of six hex digits is dark. */
function dark(hex: string): boolean {
  const channel = (at: number) => Number.parseInt(hex.slice(1 + at * 2, 3 + at * 2), 16) / 255;
  return 0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2) < 0.5;
}
/** The palette of the terminal: its own background, the surfaces and the tones of the text between its background and
 * its foreground, a step softer than the foreground, and the colors of each meaning from the theme of its mode. The
 * colors of the terminal are those it says, as #rrggbb, or nothing when it says none. */
export function terminalPalette(foreground: string | null, background: string | null): Palette | undefined {
  const hex = /^#[0-9a-f]{6}$/i;
  if (!foreground || !background || !hex.test(foreground) || !hex.test(background)) return undefined;
  const base = palettes[dark(background) ? "ink" : "day"];
  const toward = (share: number) => blend(background, foreground, share);
  return {
    ...base,
    background,
    panel: toward(0.035),
    raised: toward(0.065),
    selected: blend(background, base.accent, 0.16),
    border: toward(0.14),
    text: toward(0.84),
    prose: toward(0.8),
    muted: toward(0.56),
    faint: toward(0.44),
    added: blend(background, base.success, 0.16),
    removed: blend(background, base.danger, 0.16),
    backdrop: `${background}b0`,
    syntaxFunction: toward(0.84),
  };
}
/** A choice of a theme: a theme by its name, or the one that follows the terminal. */
export type ThemeChoice = keyof typeof palettes | "terminal";
/** The themes that the choice "terminal" shows when the terminal does not say its colors: a dark one on a dark
 * terminal, and on a terminal that does not say its mode, and a light one on a light terminal. */
export const terminalThemes: Record<ThemeMode, keyof typeof palettes> = { dark: "ink", light: "day" };
/** Whether a value names a choice of a theme. */
export function isThemeChoice(value: unknown): value is ThemeChoice {
  return value === "terminal" || (typeof value === "string" && Object.hasOwn(palettes, value));
}
/** The theme that a choice shows on a terminal in a mode, or on a terminal that does not say its mode: the choice
 * "terminal" shows the palette of the colors of the terminal once it said them. */
export function themeOf(choice: ThemeChoice, mode: ThemeMode | null): ThemeName {
  if (choice !== "terminal") return choice;
  return own.palette ? "own" : terminalThemes[mode ?? "dark"];
}
/** The name of a theme as a picker shows it, and whether it is light or dark. */
export const themeLabels: Record<ThemeName, [string, string]> = {
  ink: ["Ink", "Dark"],
  own: ["Terminal", "The colors of the terminal"],
  github: ["GitHub Dark", "Dark"],
  forest: ["Forest", "Dark"],
  paper: ["Paper", "Light"],
  midnight: ["Midnight", "Dark"],
  dusk: ["Dusk", "Dark"],
  day: ["Day", "Light"],
};
/** The rhythm of the layout, in cells: the inset of a panel, the gutter of the feed, the space between the parts of a
 * stack, between sections, and between items of a row, and the height of a bar. */
export const spacing = { inset: 1, gutter: 2, stack: 0, section: 1, bar: 1, between: 2 } as const;
/** The glyphs of the TUI, so that one shape says one thing everywhere. */
export const glyph = {
  open: "▾",
  closed: "▸",
  dot: "●",
  running: "◉",
  ring: "○",
  held: "◌",
  asks: "◆",
  notice: "◇",
  done: "✓",
  failed: "✗",
  cancelled: "⊘",
  pointer: "❯",
  prompt: "›",
  crumb: "›",
  bar: "┃",
  barTop: "╻",
  barBottom: "╹",
  halfTop: "▄",
  halfBottom: "▀",
  branch: "└",
  meter: "━",
  rule: "─",
  chip: "■",
  mark: "▎",
  rename: "✎",
  remove: "×",
} as const;
const spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
/** The frames that the renderer draws in a second, which bun run bench holds each frame to. */
export const fps = 60;
/** The time between two frames of the spinner, which the view ticks at. */
export const frame = 100;
/** Whether the view holds still, as reduced motion and the mode of a screen reader ask: no spinner turns, and no
 * part of the view moves with time. */
export const motion = { still: false };
/** The frame of the spinner at a time, or the mark of work that runs when the view holds still. */
export function spin(now = clock()): string {
  if (motion.still) return glyph.running;
  return spinner[Math.floor(now / frame) % spinner.length] ?? "⠋";
}
/** Whether the view shows no color, as NO_COLOR asks when it is set and not empty: each role takes the default color
 * of the terminal, and a state shows by its glyph, its words and its weight alone. */
export const colorless = { on: Boolean(process.env.NO_COLOR) };
/** The roles that color a ground, which take the default background of the terminal when the view shows no color. */
const grounds = new Set<keyof Palette>([
  "background",
  "panel",
  "raised",
  "selected",
  "added",
  "removed",
  "backdrop",
]);
/** The color of a role of a theme. Each role has its own color object, even when two roles have the same value. */
function colorOf(name: ThemeName, role: keyof Palette): RGBA {
  if (!colorless.on) return RGBA.fromHex(paletteOf(name)[role]);
  return grounds.has(role) ? RGBA.defaultBackground() : RGBA.defaultForeground();
}
export const theme = Object.fromEntries(
  (Object.keys(paletteOf(defaultTheme)) as (keyof Palette)[]).map((role) => [
    role,
    colorOf(defaultTheme, role),
  ]),
) as Record<keyof Palette, RGBA>;
export function setTheme(name: ThemeName): void {
  for (const role of Object.keys(theme) as (keyof Palette)[]) theme[role] = colorOf(name, role);
}
export function syntax(): SyntaxStyle {
  return SyntaxStyle.fromStyles({
    default: { fg: theme.text },
    keyword: { fg: theme.syntaxKeyword },
    string: { fg: theme.syntaxString },
    comment: { fg: theme.syntaxComment, italic: true },
    number: { fg: theme.syntaxNumber },
    function: { fg: theme.syntaxFunction },
    type: { fg: theme.syntaxType },
    operator: { fg: theme.syntaxKeyword },
    punctuation: { fg: theme.muted },
    diagnostic: { fg: theme.danger, bg: theme.removed, underline: true },
    fold: { fg: theme.link, underline: true },
    matching: { fg: theme.accent, bg: theme.selected, bold: true },
    conceal: { fg: theme.faint },
    "markup.heading": { fg: theme.text, bold: true },
    // A heading takes the style of its level, and no level falls back to the style of a heading.
    "markup.heading.1": { fg: theme.text, bold: true },
    "markup.heading.2": { fg: theme.text, bold: true },
    "markup.heading.3": { fg: theme.text, bold: true },
    "markup.heading.4": { fg: theme.muted, bold: true },
    "markup.heading.5": { fg: theme.muted, bold: true },
    "markup.heading.6": { fg: theme.muted, bold: true },
    "markup.strong": { fg: theme.text, bold: true },
    "markup.italic": { fg: theme.text, italic: true },
    "markup.strikethrough": { fg: theme.muted },
    "markup.list": { fg: theme.accent },
    "markup.raw": { fg: theme.syntaxType },
    "markup.link": { fg: theme.link, underline: true },
    "markup.link.label": { fg: theme.link },
    "markup.link.url": { fg: theme.faint, underline: true },
    "markup.quote": { fg: theme.muted, italic: true },
  });
}
