/** A key of the TUI and what it does. A chord that a terminal sends only with the kitty keyboard protocol names, as
 * its legacy chord, the chord that every terminal sends for the same action. */
export interface Key {
  chord: string;
  legacy?: string;
  action: string;
}

/** The keys of the TUI, which the help and the README list. A key names its modifiers by their marks: ⌃ for Control,
 * ⌥ for Option or Alt, and ⇧ for Shift. No key of the TUI needs ⌘, since a terminal keeps most chords of ⌘ for
 * itself. */
export const keys: Key[] = [
  { chord: "Enter", action: "Send a message, or run Python input" },
  { chord: "⇧Enter", legacy: "⌃J", action: "Insert a new line" },
  { chord: "⌥Enter", action: "Queue this message after current work" },
  { chord: "↑ / ↓", action: "Previous / next sent input, from the first or the last line" },
  { chord: "↑ in an empty input", action: "Take the last queued message back to edit it" },
  { chord: "⌃S", action: "Put the input aside, or bring it back" },
  { chord: "Esc twice", action: "Rewind: branch from an earlier point of the chain" },
  { chord: "⌃1-3", legacy: "⌥1-3", action: "Feed / transcript / changes view" },
  { chord: "⌃Tab / ⇧⌃Tab", action: "Next / previous chain" },
  { chord: "⌃P", action: "Search all actions" },
  { chord: "⌃B / ⌃N", action: "Switch / create a chain" },
  { chord: "⌃M", legacy: "⌥M", action: "Choose a model" },
  { chord: "⇧Tab / ⌃T", action: "Choose an effort / a theme" },
  { chord: "⌃O", action: "Saved sessions" },
  { chord: "⌃W / ⌃\\", action: "Workspaces and sessions / show or hide the sidebar" },
  { chord: "⌥D / ⌥E", action: "Fold or expand details / external editor" },
  { chord: "⌃V", action: "Paste a clipboard image" },
  { chord: "@ or !", action: "Find a project file, or run a shell command" },
  {
    chord: "↑, ↓, Tab, Enter after / or @",
    action: "Choose a slash command, its value, or a project file as it is typed",
  },
  { chord: "⌃F / PageUp / PageDown", action: "Filter the current view / scroll" },
  { chord: "⌃R / ⌃Space", action: "Python input / complete a name" },
  { chord: "⌃L / ⌃A", action: "Edit a prompt program / answer an operator question" },
  { chord: "⌃G / ⌃click a name", action: "Inspect a value and follow its definition" },
  { chord: "Drag / ⌃Y", action: "Select text and copy it / copy the selected text again" },
  { chord: "Click / right-click an act", action: "Fold or expand it / inspect its value or prompt program" },
  { chord: "⌃⌥←", action: "Return from a definition jump" },
  { chord: "⌥[ / ⌥]", legacy: "⌥P / ⌥N", action: "Previous / next message of the feed" },
  { chord: "⌃PageUp / ⌃PageDown", action: "Previous / next page of file changes" },
  { chord: "⌃C", action: "Clear the input, close a dialog, or cancel current work" },
  { chord: "Esc", action: "Close a dialog or pause current model work" },
  { chord: "F1 / ⌃Q / ⌃D twice", action: "Help / save and quit / save and quit from an empty input" },
];

/** The chords of a key that a terminal sends: with the kitty keyboard protocol, the chord and its legacy chord;
 * without it, the legacy chord alone. */
export function chords(key: Key, kitty: boolean): string {
  return !key.legacy ? key.chord : kitty ? `${key.chord} or ${key.legacy}` : key.legacy;
}
