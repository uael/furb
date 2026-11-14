import type { Ask as Asked, Desk as Owed } from "../protocol.js";
import { readable } from "./loom.js";

/** A prompt to the operator that waits for him, as the crate folds it, with the label of its chain. A question waits
 * for an answer of its shape. A notice is a prompt of None, which the console closed as it took it. */
export type Ask = Asked & { label: string };

/** What waits for the operator in a life: each ask, the oldest first, and how many of them are questions and
 * notices. */
export type Desk = Omit<Owed, "asks"> & { asks: Ask[] };

/** What an ask wants of the operator, in words: nothing for a notice, the choices for a choice, any of the values of a
 * list of them, and else its shape. */
export function wants(ask: Pick<Ask, "notice" | "shape" | "choices" | "ticks">): string {
  if (ask.notice) return "nothing";
  const { choices } = ask;
  if (choices.length > 1) return `${choices.slice(0, -1).join(", ")} or ${choices.at(-1)}`;
  const ticks = ask.ticks.map((one) => one.shown);
  if (ticks.length)
    return `any of ${ticks.length > 1 ? `${ticks.slice(0, -1).join(", ")} or ${ticks.at(-1)}` : ticks[0]}`;
  return choices[0] ?? ask.shape;
}

/** What a message asks, on one line, as a person reads it: its first line that holds words, with the line after it
 * when the first only leads in to it with a colon, as a relay says who asks before the question. */
export function gist(message: string): string {
  const [first = "", second] = message
    .split("\n")
    .filter((line) => line.trim())
    .map((line) => readable(line));
  return first.endsWith(":") && second ? `${first} ${second}` : first;
}

/** The ask that the operator acts on, on the chain that he reads: the one that he points at, else the oldest question,
 * else the oldest notice. A chain where nothing waits gives none. */
export function aimed(asks: readonly Ask[], chain: string, pointed?: string): Ask | undefined {
  const here = asks.filter((ask) => ask.chain === chain);
  return here.find((ask) => ask.id === pointed) ?? here.find((ask) => !ask.notice) ?? here[0];
}

/** The ask a step after or before one, round from the last to the first, or the first when the one is gone. */
export function beside(asks: readonly Ask[], id: string | undefined, step: number): Ask | undefined {
  const at = asks.findIndex((ask) => ask.id === id);
  if (at < 0) return step < 0 ? asks.at(-1) : asks[0];
  return asks[(((at + step) % asks.length) + asks.length) % asks.length];
}
