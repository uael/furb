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
/** What a pause carries: the act that said it, the reply whose error made a chain pause itself, and the cause that it
 * gave, as the fact pause holds it. */
export interface PauseCause {
  by: string;
  act?: string;
  why?: unknown;
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
  /** The cause of the pause that stands over the act, while one does. */
  pause?: PauseCause;
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
/** A compaction of a chain. In place, it is a prefix that a word said on the chain after its birth, which takes the
 * place of what its transcript held, so that its model reads only what the prefix kept. Into a new chain, as an older
 * compaction made it, it is a chain whose source it goes on `from`: the first word that the compaction gave it restores
 * that source, as `restore('chain1')`, then binds the summary and reads the files again. It gives the chain, the source
 * that a new chain goes on from, the act that said the prefix or the word, and its place in time, as the place of an
 * act: the prefix, or the birth of the new chain. */
export interface Compaction {
  chain: string;
  from?: string;
  by: string;
  at: number;
}

/** The chains that compactions into a new chain join into one run with a chain, in order: the chain that the run
 * starts from, then each chain that goes on from the one before it. A chain that no such compaction joins is a run of
 * its own. */
export function compactedRun(chain: string, compactions: readonly Compaction[]): string[] {
  const from = new Map<string, string>();
  const into = new Map<string, string>();
  for (const one of compactions)
    if (one.from) {
      from.set(one.chain, one.from);
      into.set(one.from, one.chain);
    }
  let first = chain;
  for (let before = from.get(first); before && before !== chain; before = from.get(first)) first = before;
  const run = [first];
  for (let next = into.get(first); next && !run.includes(next); next = into.get(next)) run.push(next);
  return run;
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

/** The error that a value is, once the engine gave it as a value of python: its class and its message. */
export function errorOf(value: unknown): { is: string; message: string } | undefined {
  if (!value || typeof value !== "object" || !("is" in value) || !("args" in value)) return undefined;
  const args = (value as { args: unknown }).args;
  return {
    is: String((value as { is: unknown }).is),
    message: Array.isArray(args) ? args.map(String).join(" ") : "",
  };
}

/** The words of the cause of a pause: who paused, which act it was, and the error or the reason that it gave. Whether
 * the cause is an error says which tone it takes. */
export function pauseSaid(cause: PauseCause): { text: string; error: boolean } {
  const who = cause.by === "operator" ? "the operator" : (cause.act ?? cause.by);
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
