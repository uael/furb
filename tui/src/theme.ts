import { RGBA, SyntaxStyle } from "@opentui/core";
import { mix } from "./ui.ts";

/** The roles of a palette. Surfaces go from the ground up: the sidebar, then a block, the composer, and a dialog. Text
 * goes from bright down: what someone said, the steps of a model, then the chrome. Four hues name who acts and how it
 * ends: the operator, a model, what needs a look, and what is done. */
interface Palette {
  /** The surface of the feed. */
  ground: string;
  /** The surface of the sidebar. */
  surface1: string;
  /** The surface of a block, the composer, and a dialog. */
  surface2: string;
  /** What someone said: a message, a question, an answer, and a number. */
  bright: string;
  /** The steps of a model, and the text that supports what someone said. */
  prose: string;
  /** The chrome: a key, a unit, a rule, and a line that frames the rest. */
  faint: string;
  /** The operator: a message, what Enter does, the selection, and what a click or a key reaches. */
  operator: string;
  /** A model, and Python, the language that a model writes. */
  model: string;
  /** What needs a look: the caret, an effort, a question, and a raise. */
  warm: string;
  /** What is done. */
  done: string;
}

export const palettes = {
  furb: {
    ground: "#0a0c10",
    surface1: "#0e1116",
    surface2: "#13171d",
    bright: "#eef1f5",
    prose: "#c4cad2",
    faint: "#5d6672",
    operator: "#6aa9ff",
    model: "#b69cff",
    warm: "#f0a574",
    done: "#59c07a",
  },
  github: {
    ground: "#0d1117",
    surface1: "#161b22",
    surface2: "#1c2330",
    bright: "#e6edf3",
    prose: "#c9d1d9",
    faint: "#6e7681",
    operator: "#58a6ff",
    model: "#bc8cff",
    warm: "#f0883e",
    done: "#3fb950",
  },
  forest: {
    ground: "#101817",
    surface1: "#15211e",
    surface2: "#1c2b26",
    bright: "#e3e9df",
    prose: "#c9d1c4",
    faint: "#66796b",
    operator: "#b7d89b",
    model: "#c7aee0",
    warm: "#e5b37a",
    done: "#88cabe",
  },
  paper: {
    ground: "#f6f4ec",
    surface1: "#eeebe1",
    surface2: "#e5e1d5",
    bright: "#1f2a22",
    prose: "#3a473e",
    faint: "#86938a",
    operator: "#2f5f9a",
    model: "#7a4fb0",
    warm: "#b05a22",
    done: "#2f7d4f",
  },
  midnight: {
    ground: "#141722",
    surface1: "#1b2030",
    surface2: "#232a3d",
    bright: "#e4e9f4",
    prose: "#c6cee0",
    faint: "#6d7a93",
    operator: "#99bfff",
    model: "#e2a8f0",
    warm: "#eab080",
    done: "#81cfda",
  },
} satisfies Record<string, Palette>;
export type ThemeName = keyof typeof palettes;
export const defaultTheme: ThemeName = "furb";
/** The name of a theme as a picker shows it, and whether it is light or dark. */
export const themeLabels: Record<ThemeName, [string, string]> = {
  furb: ["Furb", "Dark"],
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
  small: "·",
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
/** The motion of the TUI: the frames of the spinner and the time of each, the cells of the bar that walks while a
 * reply streams and the time of each step, the time a new step takes to settle, and the time the check of a closed
 * thread stands. */
export const motion = {
  spinner: ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
  frame: 80,
  bar: 8,
  walk: 90,
  settle: 240,
  check: 1200,
} as const;
/** The frame of the spinner at a time, a place further for each place of what it stands for. */
export function spin(now = Date.now(), place = 0): string {
  const frames = motion.spinner;
  return frames[(Math.floor(now / motion.frame) + place) % frames.length] ?? "⠋";
}
/** The roles of a palette and what the view derives from them, each as a color. */
type Colors = Record<keyof Palette | "selected" | "rule" | "added" | "removed" | "backdrop", RGBA>;
/** The colors of a palette: its roles, and the tints that the view derives from them. */
function colors(name: ThemeName): Colors {
  const roles = Object.fromEntries(
    Object.entries(palettes[name]).map(([role, hex]) => [role, RGBA.fromHex(hex)]),
  ) as Record<keyof Palette, RGBA>;
  const light = themeLabels[name][1] === "Light";
  const veil = light ? roles.bright : RGBA.fromValues(0, 0, 0, 1);
  return {
    ...roles,
    selected: mix(roles.surface2, roles.operator, 0.2),
    rule: mix(roles.surface1, roles.faint, 0.35),
    added: mix(roles.ground, roles.done, 0.16),
    removed: mix(roles.ground, roles.warm, 0.16),
    backdrop: RGBA.fromValues(veil.r, veil.g, veil.b, light ? 0.19 : 0.63),
  };
}
// Each role has its own color object, even when two roles have the same RGB value.
export const theme = colors(defaultTheme);
export function setTheme(name: ThemeName): void {
  Object.assign(theme, colors(name));
}
/** The colors of a palette as the hex of each, for a picture of the screen or a page. */
export function hexes(name: ThemeName): Record<keyof Colors, string> {
  const hex = (color: RGBA) =>
    `#${color
      .toInts()
      .slice(0, color.a < 1 ? 4 : 3)
      .map((part) => part.toString(16).padStart(2, "0"))
      .join("")}`;
  return Object.fromEntries(
    Object.entries(colors(name)).map(([role, color]) => [role, hex(color)]),
  ) as Record<keyof Colors, string>;
}
/** The colors of code and of markdown. Code takes the hues of the roles: a keyword the model, a call the operator, a
 * string what is done, a number what needs a look, and a comment the chrome. No style is bold: bold goes to who speaks,
 * the selected thread, and what Enter does. */
export function syntax(): SyntaxStyle {
  return SyntaxStyle.fromStyles({
    default: { fg: theme.bright },
    keyword: { fg: theme.model },
    string: { fg: theme.done },
    comment: { fg: theme.faint, italic: true },
    number: { fg: theme.warm },
    function: { fg: theme.operator },
    type: { fg: theme.model },
    operator: { fg: theme.prose },
    punctuation: { fg: theme.faint },
    diagnostic: { fg: theme.warm, bg: theme.removed, underline: true },
    matching: { fg: theme.operator, bg: theme.selected },
    conceal: { fg: theme.faint },
    "markup.heading": { fg: theme.bright },
    // A heading takes the style of its level, and no level falls back to the style of a heading.
    "markup.heading.1": { fg: theme.bright },
    "markup.heading.2": { fg: theme.bright },
    "markup.heading.3": { fg: theme.bright },
    "markup.heading.4": { fg: theme.prose },
    "markup.heading.5": { fg: theme.prose },
    "markup.heading.6": { fg: theme.prose },
    "markup.strong": { fg: theme.bright },
    "markup.italic": { fg: theme.prose, italic: true },
    "markup.strikethrough": { fg: theme.faint },
    "markup.list": { fg: theme.faint },
    "markup.raw": { fg: theme.model },
    "markup.link": { fg: theme.operator, underline: true },
    "markup.link.label": { fg: theme.operator },
    "markup.link.url": { fg: theme.faint, underline: true },
    "markup.quote": { fg: theme.prose, italic: true },
  });
}
