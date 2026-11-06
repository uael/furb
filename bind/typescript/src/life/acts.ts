import type { Act, Pause, RunState } from "../protocol.js";

/** The model of a life: the views that a host derives from the acts of a life, which make nothing and keep nothing.
 * It holds no native code and no module of node, so that a terminal and a window of the web read the same views. */

/** The kinds of act that are work which ends later, and the table counts each one that is done. */
export const WORK = ["prompt", "rung", "bash", "wait"];

/** The questions the engine asks on the way to what an act does, which are no acts a person follows: each is
 * answered for the act that asked it, which shows what it came to. A read and a write are acts that a person follows,
 * since each says which file it read or changed. An older record holds each writing of a model as such a question,
 * where a newer one holds it as a fact. The act setup, which the World makes on the root at the end of the setup that
 * every chain shares, marks a place and does nothing that a person follows. */
const STEPS = new Set([
  "setup",
  "stand",
  "clock",
  "chance",
  "gate",
  "cd",
  "merged",
  "run",
  "writing",
  "wants",
  "reply",
]);
/** Whether a person follows an act of a kind, as the daemon reads it: each kind but the steps of the engine. */
export const followed = (kind: string): boolean => !STEPS.has(kind);

/** Whether a site is the operator or a member of the operator: the site operator, or a site of the form operator/id,
 * as operated of the engine reads it. */
export const operated = (site: string): boolean => site.split("/")[0] === "operator";

export type { Act, Pause, Phase, RunState, Spend } from "../protocol.js";
/** What a pause carries, as the fact pause holds it: who said it, and the cause that it gave. */
export type PauseCause = Pause;

/** Whether an act is done. */
export const isDone = (act?: Pick<Act, "phase">): boolean => act?.phase.phase === "done";
/** What an act came to once it is done, and null while it is not. */
export const outcomeOf = (act?: Pick<Act, "phase">): unknown =>
  act?.phase.phase === "done" ? act.phase.value : null;
/** Whether a pause holds an act that is not done. */
export const isPaused = (act?: Pick<Act, "phase" | "pause">): boolean => Boolean(act?.pause) && !isDone(act);
/** The thread that a prompt on a thread is on: the prompt that it is on, and nothing for any other act. */
export const onThread = (act?: Pick<Act, "kind" | "on" | "chain">): string | undefined =>
  act?.kind === "prompt" && act.on !== act.chain ? act.on : undefined;
/** A compaction of a chain: a prefix that a word said on the chain after its birth, which takes the place of what its
 * transcript held, so that its model reads only what the prefix kept. It gives the chain, the act that said the
 * prefix, and its place in time, as the place of an act: the prefix. */
export interface Compaction {
  chain: string;
  by: string;
  at: number;
}
/** What a command came to: its code, and what it printed on each stream. */
export type Exit = { stdout?: { content: string }; stderr?: { content: string }; code?: number };
/** What a command came to once it is done, or what it printed so far while it runs, which a window holds beside the
 * act as the lines that the command prints come, and nothing for an act that is no command. */
export const exitOf = (act?: Act & { printed?: Exit }): Exit | undefined =>
  act?.kind !== "bash"
    ? undefined
    : isDone(act)
      ? ((outcomeOf(act) as Exit | null) ?? undefined)
      : act.printed;
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

/** Whether an act had no end when its record ended, in a record open to read, where nothing runs to end it: it stands
 * held with no pause over it, and it is no work. */
export const unfinished = (act: Pick<Act, "phase" | "pause" | "state">): boolean =>
  act.state === "paused" && !act.pause && !isDone(act);
/** What a person reads of how the word of a rung ran: why a fault, a cancel or the gate ended it, and nothing else. */
export const reasonOf = (run: RunState | null | undefined): string =>
  run && "reason" in run ? run.reason : "";

/** Whether an act failed: a rung that the gate refused or whose run raised, or an act done with an exception other
 * than a cancel. */
export function failed(act: Pick<Act, "run" | "phase">): boolean {
  if (act.run) return act.run.run === "failed" || act.run.run === "refused";
  const value = outcomeOf(act);
  return Boolean(
    value && typeof value === "object" && "is" in value && "args" in value && value.is !== "CancelledError",
  );
}

/** The error that a value is, once the engine gave it as a value of python: its class and its message. */
export function errorOf(value: unknown): { is: string; message: string } | undefined {
  if (!value || typeof value !== "object" || !("is" in value) || !("args" in value)) return undefined;
  const args = (value as { args: unknown }).args;
  return {
    is: String((value as { is: unknown }).is),
    message: Array.isArray(args) ? args.map(String).join(" ") : "",
  };
}

/** The words of the cause of a pause: who paused, and the error or the reason that it gave. Whether
 * the cause is an error says which tone it takes. */
export function pauseSaid(cause: PauseCause): { text: string; error: boolean } {
  const who = cause.by === "operator" ? "the operator" : cause.by;
  const error = errorOf(cause.why);
  const why = error
    ? `${error.is}: ${error.message}`
    : typeof cause.why === "string"
      ? cause.why
      : cause.why == null
        ? ""
        : JSON.stringify(cause.why);
  return { text: why ? `${who}: ${why}` : `${who} paused it`, error: error !== undefined };
}

/** Whether an act was cancelled: a rung whose run ended with a cancel, or an act done with a cancel. A cancel is what
 * the operator asked for, so it is no failure. */
export function cancelled(act: Pick<Act, "run" | "phase">): boolean {
  if (act.run) return act.run.run === "cancelled";
  const value = outcomeOf(act);
  return Boolean(value && typeof value === "object" && "is" in value && value.is === "CancelledError");
}

/** The name that a record gives its session, from the acts of its root in the order of the record: the value of the
 * last rename_session that is done, which the operator said and which may be empty to take the default name again,
 * else the value of name_session, which the extension naming gave. */
export function sessionNamed(
  acts: Iterable<Pick<Act, "kind" | "chain" | "phase">>,
  root: string,
): string | undefined {
  let given: string | undefined;
  let said: string | undefined;
  for (const act of acts) {
    const value = outcomeOf(act);
    if (act.chain !== root || typeof value !== "string") continue;
    if (act.kind === "rename_session") said = value;
    else if (act.kind === "name_session" && value) given = value;
  }
  return said ?? given;
}
