import { SyntaxStyle } from "@opentui/core";

const forest = {
  background: "#101817",
  panel: "#15211e",
  raised: "#1c2b26",
  selected: "#2a4034",
  border: "#30463b",
  text: "#e3e9df",
  muted: "#91a697",
  faint: "#607968",
  accent: "#b7d89b",
  teal: "#88cabe",
  yellow: "#e5c28a",
  red: "#eea79a",
  blue: "#a5bdd4",
};
export const palettes = {
  forest,
  paper: {
    background: "#f3f1e9",
    panel: "#eae8de",
    raised: "#fffdf5",
    selected: "#d9e4d0",
    border: "#bbc4b4",
    text: "#243329",
    muted: "#526757",
    faint: "#697c6c",
    accent: "#3c673c",
    teal: "#246e68",
    yellow: "#896123",
    red: "#ac4238",
    blue: "#325f92",
  },
  midnight: {
    background: "#141722",
    panel: "#1b2030",
    raised: "#252c40",
    selected: "#354162",
    border: "#3c4761",
    text: "#e4e9f4",
    muted: "#9daec7",
    faint: "#7b8ba8",
    accent: "#bbc3ff",
    teal: "#81cfda",
    yellow: "#e6c38b",
    red: "#f09eac",
    blue: "#99bfff",
  },
};
export type ThemeName = keyof typeof palettes;
export const theme = { ...forest };
export function setTheme(name: ThemeName): void {
  Object.assign(theme, palettes[name]);
}
export function syntax(): SyntaxStyle {
  return SyntaxStyle.fromStyles({
    default: { fg: theme.text },
    keyword: { fg: theme.teal },
    string: { fg: theme.accent },
    comment: { fg: theme.faint, italic: true },
    number: { fg: theme.yellow },
    function: { fg: theme.blue },
    type: { fg: theme.blue },
    operator: { fg: theme.muted },
    punctuation: { fg: theme.muted },
    "markup.heading": { fg: theme.accent, bold: true },
    "markup.strong": { fg: theme.text, bold: true },
    "markup.list": { fg: theme.teal },
    "markup.raw": { fg: theme.yellow },
    "markup.link": { fg: theme.blue },
    "markup.quote": { fg: theme.muted, italic: true },
    "markup.italic": { fg: theme.text, italic: true },
  });
}
