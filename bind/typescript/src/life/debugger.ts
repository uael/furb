import type { LiveAct } from "./acts.js";

/** One frame of the call stack of a chain: a rung or a prompt that waits, its chain, and the act that it waits on. */
export interface Frame {
  act: string;
  chain: string;
  awaits?: string;
}

/** The kinds of act that a word awaits. */
const awaited = new Set(["prompt", "bash", "wait"]);

/** The call stack of a chain, from the innermost wait out: its newest rung that is not done, the prompt of its ladder,
 * the rung that made that prompt, and so up to the operator. A rung waits on the newest act that it made and that is
 * not done. */
export function stack(chain: string, acts: readonly LiveAct[]): Frame[] {
  const rows = new Map(acts.map((act) => [act.id, act]));
  const open = acts.findLast((act) => act.kind === "rung" && act.on === chain && !act.done);
  const frames: Frame[] = [];
  for (let at = open; at && frames.length < 64; at = rows.get(at.by)) {
    if (at.kind !== "rung" && at.kind !== "prompt") break;
    const awaits =
      at.kind === "rung"
        ? acts.findLast((act) => act.by === at?.id && !act.done && awaited.has(act.kind))?.id
        : undefined;
    frames.push({ act: at.id, chain: at.on, ...(awaits ? { awaits } : {}) });
  }
  return frames;
}

/** The program of the ladder of a prompt, which its door holds: the words of its rungs, one after the other. */
export function program(prompt: string, acts: readonly LiveAct[], word: (rung: LiveAct) => string): string {
  return acts
    .filter((act) => act.kind === "rung" && act.by === prompt)
    .map(word)
    .join("\n");
}
