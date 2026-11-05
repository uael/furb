import { cancelled, failed, type LiveAct, WORK } from "./acts.js";

/** The one set of states of the work of a life, which an act, a thread, a chain and a session each take: it asks the
 * operator, it ended while the operator was away, it failed, a pause holds it, its model thinks or writes its word, a
 * word or a command of it runs, its word awaits a prompt on another chain, it ended well, a cancel ended it, or it
 * rests. A window adds states of its own, as a record open to read, which are no state of the work. */
export type State =
  | "asks"
  | "unread"
  | "failed"
  | "paused"
  | "thinks"
  | "runs"
  | "delegates"
  | "done"
  | "cancelled"
  | "idle";

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

/** The state of an act, with the prompts that the operator owes. A rung whose word has not come yet is its model that
 * thinks while a reply of it runs, and it rests while it waits for its turn, and an act that is no work and lives, as a chain or a grant, is work that waits on another chain. */
export function actState(act: LiveAct, owed: ReadonlySet<string>): State {
  if (owed.has(act.id)) return "asks";
  if ((act.paused || act.unfinished) && !act.done) return "paused";
  if (cancelled(act)) return "cancelled";
  if (failed(act)) return "failed";
  if (!act.done && !WORK.includes(act.kind)) return "delegates";
  if (act.done && (act.kind !== "rung" || act.run?.status !== "running")) return "done";
  if (act.kind === "rung" && !act.done && act.word === undefined) return act.asking ? "thinks" : "idle";
  return "runs";
}

/** What the work of a chain that is not done does, from its own acts that are not done and the prompts that its words
 * made on other chains and that are not done: a model that writes a word of it thinks, a command or a wait of it runs,
 * a word of it that awaits only another chain delegates, and a chain whose open rungs only wait for their turn rests. */
export function working(
  local: readonly LiveAct[],
  remote: readonly LiveAct[],
): "thinks" | "runs" | "delegates" | "idle" {
  const open = local.filter((act) => !act.done);
  if (open.some((act) => act.kind === "rung" && act.word === undefined && act.asking)) return "thinks";
  if (open.some((act) => act.kind === "bash" || act.kind === "wait")) return "runs";
  if (remote.some((act) => !act.done)) return "delegates";
  const turn = (act: LiveAct) => act.kind === "rung" && act.word === undefined;
  return open.some(turn) && !open.some((act) => act.kind === "rung" && !turn(act)) ? "idle" : "runs";
}
