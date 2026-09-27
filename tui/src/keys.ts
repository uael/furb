import type { commands } from "./commands.ts";

/** One thing that a key does: the presses that do it, the action that they run, and where the palette shows its
 * chord. A press names its key after its modifiers, as ctrl+b, and holds at least those modifiers. The palette shows
 * the chord beside the slash command that does the same, or as a choice of its own before a command. */
export interface Binding {
  on?: readonly string[];
  /** The name of the action that the App runs. */
  run?: string;
  /** The chord as the palette shows it, when it is not the chord of the key. */
  chord?: string;
  command?: keyof typeof commands;
  choice?: { label: string; detail: string; before: keyof typeof commands };
  /** Whether the presses act while a dialog is open. */
  always?: boolean;
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
    action: "Rewind: branch from an earlier point of the chain",
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
    bindings: [{ on: ["ctrl+tab", "super+tab"], run: "roll" }],
  },
  { chord: "⌃P", action: "Search all actions", bindings: [{ on: ["ctrl+p"], run: "palette" }] },
  {
    chord: "⌃B / ⌃N",
    action: "Switch / create a chain",
    bindings: [
      {
        on: ["ctrl+b"],
        run: "chains",
        chord: "⌃B",
        choice: { label: "Switch chain", detail: "Go to any chain of this session", before: "chain" },
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
    action: "Workspaces and sessions / show or hide the sidebar",
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
    chord: "↑, ↓, Tab, Enter after / or @",
    action: "Choose a slash command, its value, or a project file as it is typed",
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
    action: "Edit a prompt program / answer an operator question",
    bindings: [
      { on: ["ctrl+l"], run: "ladders", chord: "⌃L", command: "edit" },
      { on: ["ctrl+a"], run: "answer" },
    ],
  },
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
    chord: "⌃PageUp / ⌃PageDown",
    action: "Previous / next page of file changes",
    bindings: [{ on: ["ctrl+pageup", "ctrl+pagedown"], run: "page", always: true }],
  },
  { chord: "⌃C", action: "Clear the input, close a dialog, or cancel current work" },
  { chord: "Esc", action: "Close a dialog or pause current model work" },
  {
    chord: "F1 / ⌃Q / ⌃D twice",
    action: "Help / save and quit / save and quit from an empty input",
    bindings: [
      {
        on: ["f1"],
        run: "help",
        chord: "F1",
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

/** Each binding with its key, the bindings with more modifiers first, so that ⌃PageUp pages the changes before
 * PageUp scrolls them. The action of each binding is one that Action names, since Action is read from the table. */
export const bindings = (keys as readonly Key[])
  .flatMap((key) =>
    (key.bindings ?? []).map((binding) => ({ key, binding: binding as Binding & { run?: Action } })),
  )
  .sort((one, other) => modifiers(other.binding) - modifiers(one.binding));

function modifiers(binding: Binding): number {
  return Math.max(0, ...(binding.on ?? []).map((press) => press.split("+").length - 1));
}

/** Whether a press of a key is one that a binding names: the same key, with at least the modifiers of the binding. */
export function presses(
  binding: Binding,
  key: { name: string; ctrl: boolean; meta: boolean; shift: boolean; super?: boolean },
): boolean {
  return (binding.on ?? []).some((press) => {
    const [name, ...held] = press.split("+").reverse();
    return (
      name === key.name && held.every((modifier) => key[modifier as "ctrl" | "meta" | "shift" | "super"])
    );
  });
}
