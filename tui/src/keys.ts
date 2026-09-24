/** A key of the TUI and what it does. A chord that a terminal sends only with the kitty keyboard protocol names, as
 * its legacy chord, the chord that every terminal sends for the same action. */
export interface Key {
  chord: string;
  legacy?: string;
  action: string;
}

/** The keys of the TUI, which the help and the README list. */
export const keys: Key[] = [
  { chord: "Enter", action: "Send a message, or run Python input" },
  { chord: "Shift+Enter", legacy: "Ctrl+J", action: "Insert a new line" },
  {
    chord: "Ctrl+1 through Ctrl+6",
    legacy: "Alt+1 through Alt+6",
    action: "Conversation / program / activity / facts / transcript / changes view",
  },
  { chord: "Ctrl+P", action: "Search all actions" },
  { chord: "Ctrl+B / Ctrl+N", action: "Switch / create a chain" },
  { chord: "Ctrl+M", legacy: "Alt+M", action: "Choose a model" },
  { chord: "Shift+Tab / Ctrl+T", action: "Choose an effort / a theme" },
  { chord: "Ctrl+O", action: "Saved sessions" },
  { chord: "Ctrl+W / Ctrl+\\", action: "Workspaces and sessions / toggle the left sidebar" },
  { chord: "Alt+D / Alt+E", action: "Fold or expand details / external editor" },
  { chord: "Alt+Enter", action: "Queue this message after current work" },
  { chord: "Ctrl+V", action: "Paste a clipboard image" },
  { chord: "@ / !", action: "Find a project file / run a shell command" },
  {
    chord: "Up, Down, Tab, Enter after / or @",
    action: "Choose a slash command or a project file as it is typed",
  },
  { chord: "Ctrl+F / PageUp / PageDown", action: "Filter the current view / scroll" },
  { chord: "Ctrl+R / Ctrl+Space", action: "Python input / complete a name" },
  { chord: "Ctrl+L / Ctrl+A", action: "Prompt programs / answer an operator question" },
  { chord: "Ctrl+G / Ctrl+click a name", action: "Inspect a value and follow its definition" },
  { chord: "Ctrl+Y", action: "Copy the selected text" },
  { chord: "Click / right-click an act", action: "Fold or expand it / inspect its value or prompt program" },
  { chord: "Ctrl+Alt+Left", action: "Return from a definition jump" },
  { chord: "Alt+[ / Alt+]", legacy: "Alt+P / Alt+N", action: "Previous / next prompt REPL" },
  { chord: "Alt+Up / Alt+Down", action: "Previous / next submitted input" },
  { chord: "Ctrl+PageUp / Ctrl+PageDown", action: "Previous / next page of file changes" },
  { chord: "Ctrl+C", action: "Clear the input, close a dialog, or cancel current work" },
  { chord: "Escape", action: "Close a dialog or pause current model work" },
  { chord: "F1 / Ctrl+Q / Ctrl+D twice", action: "Help / save and quit / save and quit from an empty input" },
];

/** The chords of a key that a terminal sends: with the kitty keyboard protocol, the chord and its legacy chord;
 * without it, the legacy chord alone. */
export function chords(key: Key, kitty: boolean): string {
  return !key.legacy ? key.chord : kitty ? `${key.chord} or ${key.legacy}` : key.legacy;
}
