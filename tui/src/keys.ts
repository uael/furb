import type { Focus } from "./app.ts";

/** The keys of each focus, as the footer offers them. */
export const hints: Record<Focus, [string, string][]> = {
  loom: [
    ["↑↓", "thread"],
    ["⏎", "open"],
    ["←→", "fold"],
    ["[ ]", "time"],
    ["tab", "answer"],
    ["i", "ask"],
    [":", "python"],
    ["space", "pause"],
    ["x", "cancel"],
    ["g", "grant"],
    ["?", "keys"],
  ],
  thread: [
    ["↑↓", "line"],
    ["⏎", "lens"],
    ["esc", "loom"],
    ["tab", "answer"],
    ["space", "pause"],
    ["x", "cancel"],
    ["c", "close"],
    ["e", "door"],
    ["b", "branch"],
    ["?", "keys"],
  ],
  lens: [
    ["↑↓", "scroll"],
    ["esc", "thread"],
    ["space", "pause"],
    ["x", "cancel"],
    ["c", "close"],
  ],
  word: [
    ["⏎", "send"],
    ["⇧⏎", "new line"],
    ["esc", "leave"],
  ],
  lives: [
    ["↑↓", "life"],
    ["⏎", "open"],
    ["n", "new"],
    ["esc", "leave"],
  ],
};

/** Every key of the TUI, which the help and the README list. */
export const keyTable: [string, string][] = [
  ["↑ ↓ or k j", "Select a thread of the loom, a line of the thread, or scroll the lens"],
  ["← → or h l", "Fold the workers of a thread into it, or show them again"],
  ["Enter", "Open the selection: a thread, a lens of an act, or the answer of a prompt that you owe"],
  ["Esc", "Go back one level"],
  ["Tab", "Answer the oldest prompt that you owe, on any chain"],
  ["i", "Ask the model of the selected chain"],
  [":", "Write a word of python, which runs as a rung on the selected chain"],
  ["/", "Type a command, as /model, /theme or /fork"],
  ["space", "Pause the selection, or wake it"],
  ["x", "Cancel the selection, or the work of the selected chain"],
  ["c", "Close the selected act with a value"],
  ["g", "Put a ceiling of dollars on the selected chain"],
  ["e", "Edit the door of the prompt of the selection, which replays its ladder"],
  ["b", "Branch the chain after the selected act, or at the moment of the time cursor"],
  [
    "[ and ]",
    "Move the time cursor back and forward by a moment, or by ten with { and }. Esc returns to now",
  ],
  ["n", "Step: the selected chain runs one more rung, then a pause holds it"],
  ["v", "Change the variant: the loom of the chains in time, or the graph of the programs that run"],
  ["w", "Wake the work that an earlier life left"],
  ["⌃O", "Open the list of lives, by workspace"],
  ["?", "Show or hide these keys"],
  ["⌃Q, or ⌃D twice", "Save and quit"],
];
