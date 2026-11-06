import type { Act, State } from "../protocol.js";
import { isDone } from "./acts.js";

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

/** What the work of a chain that is not done does, from its own acts that are not done and the prompts that its words
 * made on other chains and that are not done: a model that writes a word of it thinks, a command or a wait of it runs,
 * a word of it that awaits only another chain delegates, and a chain whose open rungs only wait for their turn rests. */
export function working(
  local: readonly Act[],
  remote: readonly Act[],
): "thinks" | "runs" | "delegates" | "idle" {
  const open = local.filter((act) => !isDone(act));
  if (open.some((act) => act.kind === "rung" && act.word == null && act.asking)) return "thinks";
  if (open.some((act) => act.kind === "bash" || act.kind === "wait")) return "runs";
  if (remote.some((act) => !isDone(act))) return "delegates";
  const turn = (act: Act) => act.kind === "rung" && act.word == null;
  return open.some(turn) && !open.some((act) => act.kind === "rung" && !turn(act)) ? "idle" : "runs";
}
