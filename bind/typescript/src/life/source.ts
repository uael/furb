import type { Exit, LiveAct } from "./acts.js";
import { type ActState, actState, valueText } from "./ladder.js";
import { unmarked } from "./marks.js";

/** One line of the source of a chain.
 *
 * - `prompt`: a prompt on the chain, which heads the words of its ladder.
 * - `code`: a line of the word of a rung, with the acts that it made, and whether it runs now.
 * - `value`: what a prompt closed with.
 */
export type SourceLine =
  | { type: "prompt"; act: LiveAct; state: ActState }
  | {
      type: "code";
      rung: LiveAct;
      number: number;
      text: string;
      acts: LiveAct[];
      runs: boolean;
      state: ActState;
    }
  | { type: "value"; act: LiveAct; text: string };

/** The kinds of the acts that a person follows in the margin. */
const followed = new Set(["bash", "read", "write", "prompt", "chain", "wait", "rung", "grant", "cd"]);

/** The pattern of the calls of the verbs that make an act and of the verbs of extensions that it is given, made once
 * for each set of those verbs. */
const patterns = new Map<string, RegExp>();
function callsOf(extra: string): RegExp {
  let made = patterns.get(extra);
  if (!made) {
    made = new RegExp(`(?<!\\bdef )\\b(${[...followed, ...(extra ? [extra] : [])].join("|")})\\s*\\(`, "g");
    patterns.set(extra, made);
  }
  made.lastIndex = 0;
  return made;
}

/** The acts that a word made, each on the line of the word that made it: the next line that calls its verb, or for a
 * line that a loop runs again, the last line that calls it, or else the last line. */
export function placed(text: string, made: readonly LiveAct[]): LiveAct[][] {
  const lines = text.split("\n");
  // A verb of an extension, as todo, stands on a line that calls it by its name, and a def of that name calls nothing.
  const extra = [...new Set(made.map((act) => act.kind))].filter(
    (kind) => !followed.has(kind) && /^\w+$/.test(kind),
  );
  const calls = callsOf(extra.join("|"));
  const slots: [line: number, kind: string][] = [];
  for (const [index, line] of lines.entries())
    for (const match of line.matchAll(calls)) slots.push([index, match[1] ?? ""]);
  const out = lines.map((): LiveAct[] => []);
  let next = 0;
  for (const act of made) {
    if (!followed.has(act.kind) && !slots.some(([, kind]) => kind === act.kind)) continue;
    const ahead = slots.findIndex(([, kind], index) => index >= next && kind === act.kind);
    if (ahead >= 0) {
      out[slots[ahead]?.[0] ?? 0]?.push(act);
      next = ahead + 1;
      continue;
    }
    const again = slots.findLast(([, kind]) => kind === act.kind);
    out[again ? again[0] : lines.length - 1]?.push(act);
  }
  return out;
}

/** The source of a chain: each prompt on it, then the words of the rungs of its ladder, line by line, each line with
 * the acts that it made, and what the prompt closed with. A word that the operator wrote stands on its own, and the
 * words of the extensions stand in no source.
 * The line that runs is the line of the newest act of a rung that runs, which is not done. */
export function source(
  chain: string,
  acts: readonly LiveAct[],
  word: (rung: LiveAct) => string,
  owed: ReadonlySet<string>,
): SourceLine[] {
  const made = new Map<string, LiveAct[]>();
  for (const act of acts) {
    const list = made.get(act.by) ?? [];
    list.push(act);
    made.set(act.by, list);
  }
  const out: SourceLine[] = [];
  const rungOf = (rung: LiveAct): void => {
    const children = made.get(rung.id) ?? [];
    const text = word(rung);
    const on = placed(text, children);
    const state = actState(rung, owed);
    const open =
      state === "running" ? children.findLast((act) => !act.done && followed.has(act.kind)) : undefined;
    for (const [index, line] of text.split("\n").entries()) {
      const here = on[index] ?? [];
      out.push({
        type: "code",
        rung,
        number: index + 1,
        text: line,
        acts: here,
        runs: Boolean(open && here.includes(open)),
        state,
      });
    }
  };
  const rows = new Map(acts.map((act) => [act.id, act]));
  for (const act of acts) {
    if (act.on !== chain) continue;
    if (act.kind === "prompt") {
      out.push({ type: "prompt", act, state: actState(act, owed) });
      for (const rung of made.get(act.id) ?? []) if (rung.kind === "rung") rungOf(rung);
      const text = act.done ? valueText(act.value) : "";
      if (text) out.push({ type: "value", act, text });
    } else if (act.kind === "rung" && (act.by === "operator" || rows.get(act.by)?.kind === "chain"))
      rungOf(act);
  }
  return out;
}

/** What an act came to, in a few words, as the margin of the source shows it. */
export function outcome(act: LiveAct, label: (chain: string) => string): string {
  const value = act.value as { is?: string; content?: string; path?: string } | Exit | null | undefined;
  if (act.kind === "bash") {
    const exit = act.value as Exit | undefined;
    if (!act.done) return (exit?.stdout?.content ?? "").trimEnd().split("\n").at(-1) ?? "";
    return exit?.code === undefined ? "" : `exit ${exit.code}`;
  }
  if (act.kind === "read" || act.kind === "write") {
    // A host that cut a long text keeps the count of its lines that are not empty.
    const text = value as { content?: string; path?: string; lines?: number } | undefined;
    const path = String(
      act.words[0] && typeof act.words[0] === "object"
        ? (act.words[0] as { path?: string }).path
        : (act.words[0] ?? text?.path ?? ""),
    );
    const lines =
      typeof text?.lines === "number"
        ? text.lines
        : typeof text?.content === "string"
          ? text.content.split("\n").filter(Boolean).length
          : 0;
    return `${path.split("/").at(-1) ?? path}${lines ? `  ${lines} lines` : ""}`;
  }
  if (act.kind === "prompt") {
    const answer = act.done
      ? (valueText(act.value)
          .split("\n")
          .find((line) => line.trim()) ?? "")
      : "";
    return answer || label(act.on);
  }
  if (act.kind === "chain") return label(act.id);
  if (act.kind === "wait") return act.done ? `first ${valueText(act.value)}` : `${act.words[0] ?? ""}s`;
  if (act.kind === "cd") return valueText(act.value) || String(act.words[0] ?? "");
  if (act.kind === "grant") return `$${unmarked(act.words[0]) ?? ""}`;
  return "";
}
