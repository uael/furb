import type { commands } from "./commands.ts";

/** One thing that a key does: the presses that do it, the action that they run, and where the palette and the footer
 * show its chord. A press names its key after its modifiers, as ctrl+b, and holds exactly those modifiers. The palette
 * shows the chord beside the slash command that does the same, or as a choice of its own before a command. */
export interface Binding {
  on?: readonly string[];
  /** The name of the action that the App runs. */
  run?: string;
  /** The chord as the palette and the footer show it, when it is not the chord of the key. */
  chord?: string;
  command?: keyof typeof commands;
  choice?: { label: string; detail: string; before: keyof typeof commands };
  /** Whether the presses act while a dialog is open. */
  always?: boolean;
  /** What the footer says the key does, while the key acts. */
  hint?: string;
}

/** A key of the TUI, what it does, and each binding that does it. A chord that a terminal sends only with the kitty
 * keyboard protocol names, as its legacy chord, the chord that every terminal sends for the same action. */
export interface Key {
  chord: string;
  legacy?: string;
  action: string;
  bindings?: readonly Binding[];
}

/** The keys of the TUI, which the help and the README list, the palette shows, and the App answers. A key names its
 * modifiers by their marks: ⌃ for Control, ⌥ for Option or Alt, and ⇧ for Shift. No key of the TUI needs ⌘, since a
 * terminal keeps most chords of ⌘ for itself. */
export const keys = [
  { chord: "Enter", action: "Send a message, or run Python input" },
  { chord: "⇧Enter", legacy: "⌃J", action: "Insert a new line" },
  {
    chord: "⌥Enter",
    action: "Queue this message after current work",
    bindings: [{ on: ["meta+return", "meta+enter"], run: "queue", command: "queue" }],
  },
  { chord: "↑ / ↓", action: "Previous / next sent input, from the first or the last line" },
  { chord: "↑ in an empty input", action: "Take the last queued message back to edit it" },
  {
    chord: "→ / ⌃F",
    action: "At the end of the input, take the grey rest of the line that the history suggests",
  },
  { chord: "⌥→ / ⌥F", action: "At the end of the input, take the next word of that grey rest" },
  {
    chord: "⌃S",
    action: "Put the input aside, or bring it back",
    bindings: [
      {
        on: ["ctrl+s"],
        run: "stash",
        choice: {
          label: "Stash the input",
          detail: "Put the input aside, or bring it back",
          before: "files",
        },
      },
    ],
  },
  {
    chord: "Esc twice",
    action: "Rewind: go back to an earlier point of the chain, with its files or not",
    bindings: [{ chord: "Esc Esc", command: "rewind" }],
  },
  {
    chord: "⌃1-3",
    legacy: "⌥1-3",
    action: "Feed / transcript / changes view",
    bindings: [{ on: ["ctrl+1", "ctrl+2", "ctrl+3", "meta+1", "meta+2", "meta+3"], run: "view" }],
  },
  {
    chord: "⌃Tab / ⇧⌃Tab",
    action: "Next / previous chain",
    bindings: [{ on: ["ctrl+tab", "super+tab", "ctrl+shift+tab", "super+shift+tab"], run: "roll" }],
  },
  {
    chord: "⌃P",
    action: "Search all actions",
    bindings: [{ on: ["ctrl+p"], run: "palette", hint: "commands" }],
  },
  {
    chord: "⌃B / ⌃N",
    action: "Chains / create a chain",
    bindings: [
      {
        on: ["ctrl+b"],
        run: "chains",
        chord: "⌃B",
        choice: {
          label: "Chains",
          detail: "The chains of this session as a loom, to open one",
          before: "chain",
        },
      },
      { on: ["ctrl+n"], run: "newChain", chord: "⌃N", command: "chain" },
    ],
  },
  {
    chord: "⌃M",
    legacy: "⌥M",
    action: "Choose a model",
    bindings: [{ on: ["ctrl+m", "meta+m"], run: "models", command: "model" }],
  },
  {
    chord: "⇧Tab / ⌃T",
    action: "Choose an effort / a theme",
    bindings: [
      { on: ["shift+tab"], run: "effort", chord: "⇧Tab", command: "effort" },
      { on: ["ctrl+t"], run: "themes", chord: "⌃T", command: "theme" },
    ],
  },
  {
    chord: "⌃W / ⌃\\",
    action:
      "Workspaces and sessions / show or hide the sidebar, which takes the place of the feed on a narrow terminal",
    bindings: [
      { on: ["ctrl+w", "ctrl+o"], run: "workspaces", chord: "⌃W", command: "workspace" },
      { on: ["ctrl+\\"], run: "sidebar", chord: "⌃\\", command: "sidebar" },
    ],
  },
  {
    chord: "⌥D / ⌥E",
    action: "Fold or expand details / external editor",
    bindings: [
      { on: ["meta+d"], run: "details", chord: "⌥D", command: "details" },
      { on: ["meta+e"], run: "editor", chord: "⌥E", command: "editor" },
    ],
  },
  {
    chord: "⌃V",
    action: "Paste a clipboard image",
    bindings: [{ on: ["ctrl+v"], run: "image", command: "image" }],
  },
  {
    chord: "@ or !",
    action: "Find a project file, or run a shell command",
    bindings: [
      { chord: "@", command: "files" },
      { chord: "!", command: "bash" },
    ],
  },
  {
    chord: "↑, ↓, Tab, Enter after /, @ or :",
    action: "Choose a slash command, its value, a project file, or an emoji as it is typed",
  },
  {
    chord: "⌃F / PageUp / PageDown",
    action: "Filter the current view / scroll",
    bindings: [
      {
        on: ["ctrl+f"],
        run: "search",
        chord: "⌃F",
        choice: { label: "Filter the view", detail: "Show only what holds a text", before: "grant" },
      },
      { on: ["pageup", "pagedown"], run: "scroll" },
    ],
  },
  {
    chord: "⌃R / ⌃Space",
    action: "Python input / complete a name",
    bindings: [
      {
        on: ["ctrl+r"],
        run: "python",
        chord: "⌃R",
        choice: {
          label: "Python input",
          detail: "Write code with the same gate as the model",
          before: "run",
        },
      },
      { on: ["ctrl+space"], run: "complete" },
    ],
  },
  {
    chord: "⌃L / ⌃A",
    action: "Edit a prompt program / answer the questions that wait",
    bindings: [
      { on: ["ctrl+l"], run: "ladders", chord: "⌃L", command: "edit" },
      { on: ["ctrl+a"], run: "answer", chord: "⌃A", hint: "answer" },
    ],
  },
  { chord: "Tab / ⇧Tab in an answer", action: "Next / previous question that waits" },
  {
    chord: "⌃G / ⌃click a name",
    action: "Inspect a value and follow its definition",
    bindings: [{ on: ["ctrl+g"], run: "names", chord: "⌃G", command: "inspect" }],
  },
  {
    chord: "Drag / ⌃Y",
    action: "Select text and copy it / copy the selected text again",
    bindings: [{ on: ["ctrl+y"], run: "copy" }],
  },
  { chord: "Click / right-click an act", action: "Fold or expand it / inspect its value or prompt program" },
  { chord: "Pointer / click on an underlined ...", action: "Show a folded string whole / open it in place" },
  {
    chord: "Pointer on a line of a program",
    action: "Show what its acts came to, the act that it awaits, a message whole, or why the line failed",
  },
  {
    chord: "⌃⌥←",
    action: "Return from a definition jump",
    bindings: [{ on: ["ctrl+meta+left"], run: "back" }],
  },
  {
    chord: "⌥[ / ⌥]",
    legacy: "⌥P / ⌥N",
    action: "Previous / next message of the feed",
    // ⌃⌥P and ⌃⌥N jump too, so they are named whole: ⌃P and ⌃N alone name the palette and a new chain.
    bindings: [{ on: ["meta+[", "meta+]", "meta+p", "meta+n", "ctrl+meta+p", "ctrl+meta+n"], run: "jump" }],
  },
  {
    chord: "⌥V / ⌥G",
    action: "Switch the view of the life: main, the loom, or the graph / move in the loom or the graph",
    bindings: [
      { on: ["meta+v"], run: "lifeView", chord: "⌥V", command: "view" },
      {
        on: ["meta+g"],
        run: "steer",
        chord: "⌥G",
        choice: {
          label: "Move in the view of the life",
          detail: "The keys move on the loom or the graph, and Esc gives them back to the input",
          before: "view",
        },
      },
    ],
  },
  {
    chord: "⌥T",
    action: "Move in the sidebar: the workspaces, their sessions, and the threads of the session shown",
    bindings: [{ on: ["meta+t"], run: "rail", chord: "⌥T" }],
  },
  {
    chord: "↑↓ / Enter / → / ← or Esc after ⌥T",
    action:
      "Move / open a workspace, a session or a thread / peek at a workspace / fold it again, or back to the input",
  },
  {
    chord: "Space / r / o / a after ⌥T",
    action: "Collapse or expand / resolve / reply to the thread / go to the next question",
  },
  {
    chord: "⌥A",
    action: "Go to the next question that waits for you, in any workspace",
    bindings: [{ on: ["meta+a"], run: "nextAsk", chord: "⌥A" }],
  },
  { chord: "↑↓←→ / Enter / Esc after ⌥G", action: "Move, fold or zoom / zoom in / zoom out or go back" },
  { chord: "[ ] / { } after ⌥G", action: "Move the cursor in time by one / ten moments" },
  {
    chord: "Space / x / n / b after ⌥G",
    action: "Pause or wake / cancel / step / branch at the cursor the chain",
  },
  {
    chord: "⌃PageUp / ⌃PageDown",
    action: "Previous / next scope of file changes: this turn, this chain, the life, the branch",
    bindings: [{ on: ["ctrl+pageup", "ctrl+pagedown"], run: "scope", always: true }],
  },
  {
    chord: "⌥R / ⌥J",
    action:
      "Repair the journal of file changes / start a new journal, once the view of changes finds it damaged",
    bindings: [
      { on: ["meta+r"], run: "repair", chord: "⌥R", command: "repair" },
      { on: ["meta+j"], run: "renew", chord: "⌥J", command: "renew" },
    ],
  },
  {
    chord: "⌃C",
    action:
      "Clear the input, close a dialog, or cancel what you sent last (a message goes back to the input)",
    bindings: [{ on: ["ctrl+c"], run: "takeBack", hint: "take back" }],
  },
  {
    chord: "Esc",
    action: "Close a dialog or pause current model work",
    bindings: [{ on: ["escape"], run: "pause", hint: "pause" }],
  },
  {
    chord: "F1 / ⌃Q / ⌃D twice",
    action: "Help / save and quit / save and quit from an empty input",
    bindings: [
      {
        on: ["f1"],
        run: "help",
        chord: "F1",
        hint: "help",
        choice: { label: "Help", detail: "Keys, marks, and slash commands", before: "exit" },
      },
      { on: ["ctrl+q"], run: "quit", chord: "⌃Q", command: "exit", always: true },
    ],
  },
] as const satisfies readonly Key[];

/** The name of each action that a key of the table runs, which the App answers. */
export type Action = Extract<
  Extract<(typeof keys)[number], { bindings: unknown }>["bindings"][number],
  { run: string }
>["run"];

/** The chords of a key that a terminal sends: with the kitty keyboard protocol, the chord and its legacy chord;
 * without it, the legacy chord alone. */
export function chords(key: Key, kitty: boolean): string {
  return !key.legacy ? key.chord : kitty ? `${key.chord} or ${key.legacy}` : key.legacy;
}

/** The chord of a binding as the palette shows it: the chord that this terminal sends. */
export function shown(key: Key, binding: Binding, kitty: boolean): string {
  return binding.chord ?? (kitty ? key.chord : (key.legacy ?? key.chord));
}

/** Each binding with its key, in the order of the table. The action of each binding is one that Action names, since
 * Action is read from the table. */
export const bindings = (keys as readonly Key[]).flatMap((key) =>
  (key.bindings ?? []).map((binding) => ({ key, binding: binding as Binding & { run?: Action } })),
);
