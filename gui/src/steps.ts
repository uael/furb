import { actState, type LiveAct, outcome, reason, source } from "@furb/engine/life";
import type { State } from "./data.ts";
import { chainName, glyphOf, type Moment } from "./life.ts";

/** One act beside the line that made it: its name, its kind, its state, and what it came to in a few words. */
export interface ActMark {
  id: string;
  kind: string;
  state: State;
  summary: string;
}

/** One line of the word of a step, with its acts, and whether the word runs at it now. */
export interface Line {
  text: string;
  here: boolean;
  acts: ActMark[];
}

/** One step of a thread: a rung, with what it did, its lines, why it failed, whether its model had no word for it
 * when the record ends, and the threads that it asked. */
export interface Step {
  id: string;
  state: State;
  said: string;
  lines: Line[];
  why: string | null;
  thinks: boolean;
  asks: string[];
}

/** A text cut to a number of characters, with an ellipsis when it was cut. */
export function clip(text: string, most: number): string {
  const first = text.split("\n")[0] ?? "";
  return first.length <= most && first.length === text.length ? first : `${first.slice(0, most - 1)}…`;
}

const verbs: Record<string, [string, string]> = {
  read: ["Read", "file"],
  write: ["Wrote", "file"],
  bash: ["Ran", "command"],
  prompt: ["Asked", "thread"],
  chain: ["Hired", "chain"],
  wait: ["Waited", ""],
};

/** What a step did, in a few words: the acts that it made, counted by what they are. */
function said(rung: LiveAct, made: readonly LiveAct[], word: string): string {
  const counts = new Map<string, number>();
  for (const act of made) {
    const verb = act.kind === "prompt" && act.words[2] === "operator" ? "you" : act.kind;
    if (verb === "you" || verbs[verb]) counts.set(verb, (counts.get(verb) ?? 0) + 1);
  }
  const words = [...counts.entries()].map(([verb, n], at) => {
    const [done, noun] = verb === "you" ? ["Asked you", ""] : (verbs[verb] ?? ["", ""]);
    const head = at === 0 ? done : done.toLowerCase();
    return noun ? `${head} ${n} ${noun}${n === 1 ? "" : "s"}` : head;
  });
  if (words.length) return words.join(", ");
  return clip(
    word
      .split("\n")
      .find((line) => line.trim())
      ?.trim() ?? rung.id,
    60,
  );
}

/** The steps of the ladder of a prompt, from the source of its chain: each rung with its lines and their acts. */
export function steps(moment: Moment, prompt: LiveAct, directory: string): Step[] {
  const label = (chain: string) => chainName(moment, chain);
  const mark = (act: LiveAct): ActMark => ({
    id: act.id,
    kind: act.kind,
    state: glyphOf(actState(act, moment.owed)),
    summary: reason(act, directory) ?? (moment.owed.has(act.id) ? "asks you" : outcome(act, label)),
  });
  const out: Step[] = [];
  for (const line of source(prompt.on, moment.acts, moment.word, moment.owed)) {
    if (line.type !== "code" || line.rung.by !== prompt.id) continue;
    let step = out.at(-1);
    if (step?.id !== line.rung.id) {
      const rung = line.rung;
      const made = moment.acts.filter((act) => act.by === rung.id);
      const word = moment.word(rung);
      const why = reason(rung, directory) ?? null;
      const thinks = !rung.done && !word;
      step = {
        id: rung.id,
        state: glyphOf(actState(rung, moment.owed)),
        said: thinks ? "Its model wrote no word for this step" : why ? clip(why, 80) : said(rung, made, word),
        lines: [],
        why,
        thinks,
        asks: made.filter((act) => act.kind === "prompt").map((act) => act.id),
      };
      out.push(step);
    }
    if (step.thinks) continue;
    step.lines.push({ text: line.text, here: line.runs, acts: line.acts.map(mark) });
  }
  return out;
}
