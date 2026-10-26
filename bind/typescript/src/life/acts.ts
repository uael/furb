/** The model of a life: the views that a host derives from the acts of a life, which make nothing and keep nothing.
 * It holds no native code and no module of node, so that a terminal and a window of the web read the same views. */

/** The kinds of act that are work which ends later, and the table counts each one that is done. */
export const WORK = ["prompt", "rung", "bash", "wait"];

export interface RunState {
  status: "running" | "failed" | "done";
  reason: string;
  /** Whether the gate refused the word, so that the reason holds its findings, one for each line. */
  refused?: boolean;
}
export interface LiveAct {
  id: string;
  kind: string;
  by: string;
  /** The chain that the act stands on, which is the chain of its thread for a prompt on a thread. */
  on: string;
  /** The thread that a prompt on a thread is on: the prompt that its on names. */
  thread?: string;
  words: unknown[];
  /** The word of a rung, which its caller or its model wrote, once its chain took it to run or to refuse. */
  word?: string;
  done: boolean;
  value: unknown;
  /** Whether a pause stands over the act while it lives. */
  paused: boolean;
  /** Whether the act had no end when the record ended, in a record open to read, where nothing runs to end it. */
  unfinished?: boolean;
  run?: RunState;
  /** The place of the act in time: how many facts the life had said when the act was made, and when it was done. It
   * is the order of the record, so a later life gives the same places. */
  began: number;
  ended?: number;
  /** The dollars of the answers of the models that wrote the word of a rung, once one is done. */
  cost?: number;
}
/** What a command came to: its code, and what it printed on each stream. */
export type Exit = { stdout?: { content: string }; stderr?: { content: string }; code?: number };
/** The measure of a chain whose models answered or that a grant holds: the dollars of its answers, the dollars since
 * the grant that stands on it and the dollars of its ceiling, the share of the window that its last answer read, and
 * how many answers it had. */
export interface Measure {
  dollars: number;
  spent: number;
  ceiling?: number;
  share?: number;
  answers: number;
  /** The actor of its last answer. */
  actor?: string;
}

/** Whether an act failed: a rung that the gate refused or whose run raised, or an act done with an exception other
 * than a cancel. */
export function failed(act: LiveAct): boolean {
  if (act.run) return act.run.status === "failed" && !cancelled(act);
  const value = act.value;
  return Boolean(
    value && typeof value === "object" && "is" in value && "args" in value && value.is !== "CancelledError",
  );
}

/** Whether an act was cancelled: a rung whose run ended with a cancel, or an act done with a cancel. A cancel is what
 * the operator asked for, so it is no failure. */
export function cancelled(act: LiveAct): boolean {
  if (act.run) return act.run.status === "failed" && /^CancelledError\b/.test(act.run.reason ?? "");
  const value = act.value;
  return Boolean(value && typeof value === "object" && "is" in value && value.is === "CancelledError");
}

/** The measure of a chain from the answers of its models, in order: the dollars of each answer, how many answers it
 * had, and the actor of the last one. */
export function measureOf(answers: readonly { actor: string; usage: readonly number[] }[]): Measure {
  const dollars = answers.reduce((sum, answer) => sum + (answer.usage[4] ?? 0), 0);
  return { dollars, spent: dollars, answers: answers.length, actor: answers.at(-1)?.actor };
}
