// The probes of the walk of the audit of the TUI: checks that read the frames that a walk met, with no model and no
// judge, each for a kind of defect that a person who uses the TUI sees at once. A probe finds; the auditor judges.

/** A defect that a probe found: the capture of the step where it showed, the probe, and what it saw. */
export interface Finding {
  step: string;
  probe: string;
  detail: string;
}

/** A value of the code of the TUI that reached the screen as the text of no value. */
const LEAK = /\bundefined\b|\bNaN\b|\[object Object\]|\bInvalid Date\b/;

/** Each row of a frame that shows a value that leaked, trimmed. */
export function leaks(frame: string): string[] {
  return frame
    .split("\n")
    .filter((row) => LEAK.test(row))
    .map((row) => row.trim());
}

/** Whether a frame says that its life is not open, which a window on a life that runs never shows. */
export function shut(frame: string): boolean {
  return /\bnot open\b/i.test(frame);
}

/** The rows that differ between two frames, each as its number from 1 and its text before and after, at most `most`. */
export function changed(before: string, after: string, most = 6): string[] {
  const [was, now] = [before.split("\n"), after.split("\n")];
  const rows: string[] = [];
  for (let n = 0; n < Math.max(was.length, now.length); n++)
    if (was[n] !== now[n])
      rows.push(
        `row ${n + 1}: ${JSON.stringify((was[n] ?? "").trim())} -> ${JSON.stringify((now[n] ?? "").trim())}`,
      );
  return rows.length > most ? [...rows.slice(0, most), `and ${rows.length - most} more rows`] : rows;
}

/** The key that the mock input of a test terminal sends for the name of a key in a chord of the table of keys. */
const NAMED: Record<string, string> = {
  tab: "TAB",
  escape: "ESCAPE",
  return: "RETURN",
  enter: "RETURN",
  up: "ARROW_UP",
  down: "ARROW_DOWN",
  left: "ARROW_LEFT",
  right: "ARROW_RIGHT",
  f1: "F1",
  pageup: "\u001b[5~",
  pagedown: "\u001b[6~",
  space: " ",
};

/** A chord of the table of keys, as `ctrl+meta+left`, as the key and the modifiers that the mock input presses. */
export function chord(on: string): {
  key: string;
  modifiers: { ctrl: boolean; meta: boolean; shift: boolean };
} {
  const parts = on.split("+");
  // A chord whose key is + itself ends with an empty part.
  const last = on.endsWith("++") ? "+" : (parts.pop() ?? "");
  const named = new Set(parts);
  return {
    key: NAMED[last] ?? last,
    modifiers: { ctrl: named.has("ctrl"), meta: named.has("meta"), shift: named.has("shift") },
  };
}
