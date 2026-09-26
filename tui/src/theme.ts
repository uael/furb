import { RGBA, SyntaxStyle } from "@opentui/core";

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
} satisfies Record<string, Palette>;
export type ThemeName = keyof typeof palettes;
export const defaultTheme: ThemeName = "github";
/** The name of a theme as a picker shows it, and whether it is light or dark. */
export const themeLabels: Record<ThemeName, [string, string]> = {
  github: ["GitHub Dark", "Dark"],
  forest: ["Forest", "Dark"],
  paper: ["Paper", "Light"],
  midnight: ["Midnight", "Dark"],
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
/** The time between two frames of the spinner, which the view ticks at. */
export const frame = 100;
/** The frame of the spinner at a time. */
export function spin(now = Date.now()): string {
  return spinner[Math.floor(now / frame) % spinner.length] ?? "⠋";
}
// Each role has its own color object, even when two roles have the same RGB value.
export const theme = Object.fromEntries(
  Object.entries(palettes[defaultTheme]).map(([role, hex]) => [role, RGBA.fromHex(hex)]),
) as Record<keyof Palette, RGBA>;
export function setTheme(name: ThemeName): void {
  for (const role of Object.keys(theme) as (keyof Palette)[])
    theme[role] = RGBA.fromHex(palettes[name][role]);
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
