import type { State } from "../protocol.js";

export type { State } from "../protocol.js";

/** The states in the order of the roll-up: a thread, a chain or a session takes the first state that any of its parts
 * holds. */
export const STATES: readonly State[] = [
  "asks",
  "unread",
  "failed",
  "paused",
  "thinks",
  "runs",
  "delegates",
  "done",
  "cancelled",
  "idle",
];

/** The state of a whole from the states of its parts: the first of them in the order of [`STATES`], and idle for a
 * whole with no part. */
export function rollUp(states: Iterable<State>): State {
  let best = STATES.length - 1;
  for (const state of states) best = Math.min(best, STATES.indexOf(state));
  return STATES[best] ?? "idle";
}

/** Whether a state is work that is not done: a model that thinks, a word or a command that runs, or a word that awaits
 * another chain. */
export const busy = (state: string): boolean =>
  state === "thinks" || state === "runs" || state === "delegates";
