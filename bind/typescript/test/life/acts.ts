import type { Act, Exit, RunState } from "../../src/life/acts.ts";
import type { State } from "../../src/life/states.ts";

/** What a test says of an act beside its kind, its name, its maker and its place: each field of an act of the
 * daemon, and in a short form whether it is done, what it came to, whether a pause holds it, the thread that a prompt
 * is on, how the word of a rung ran, and whether it had no end when its record ended. */
export type More = Partial<Omit<Act, "run">> & {
  done?: boolean;
  value?: unknown;
  paused?: boolean;
  thread?: string;
  unfinished?: boolean;
  run?: RunState | { status: "running" | "failed" | "done"; reason: string; refused?: boolean } | null;
};

/** Whether a value is a fault that the engine gave, of a class. */
const fault = (value: unknown, cancel: boolean) =>
  Boolean(value && typeof value === "object" && "is" in value && "args" in value) &&
  ((value as { is: unknown }).is === "CancelledError") === cancel;

/** The state of an act as the daemon gives it, from its parts, as State::of of furb-host reads them. */
export function stateOf(act: Act, unfinished = false): State {
  const done = act.phase.phase === "done";
  const value = act.phase.phase === "done" ? act.phase.value : null;
  const failed = act.run ? act.run.run === "failed" || act.run.run === "refused" : fault(value, false);
  const cancelled = act.run ? act.run.run === "cancelled" : fault(value, true);
  if (act.operator && !done) return "asks";
  if ((act.pause || unfinished) && !done) return "paused";
  if (cancelled) return "cancelled";
  if (failed) return "failed";
  if (!done && !["prompt", "rung", "bash", "wait"].includes(act.kind)) return "delegates";
  if (done && (act.kind !== "rung" || act.run?.run !== "running")) return "done";
  if (act.kind === "rung" && !done && act.word == null) return act.asking ? "thinks" : "idle";
  return "runs";
}

/** How the word of a rung ran, from the short form of a test. */
function ran(run: More["run"]): RunState | null {
  if (!run || "run" in run) return run ?? null;
  if (run.status !== "failed") return { run: run.status };
  if (run.refused) return { run: "refused", reason: run.reason };
  return /^CancelledError\b/.test(run.reason)
    ? { run: "cancelled", reason: run.reason }
    : { run: "failed", reason: run.reason };
}

/** An act of a life, made at a moment, and done at another when it is done, in the state that its parts give it. */
export function act(
  id: string,
  kind: string,
  on: string,
  by: string,
  began: number,
  ended?: number,
  more: More = {},
): Act {
  const { done, value = null, paused, thread, unfinished, run, ...rest } = more;
  const end = rest.ended ?? ended ?? (done ? began : undefined);
  const chain = kind === "chain" ? id : on;
  const made: Act = {
    id,
    kind,
    by,
    on: thread ?? chain,
    chain,
    words: [],
    phase: end !== undefined && done !== false ? { phase: "done", value } : { phase: "asked" },
    operator: false,
    pause: paused ? { by: "operator" } : null,
    run:
      run !== undefined ? ran(run) : kind === "rung" ? { run: end === undefined ? "running" : "done" } : null,
    word: null,
    asking: false,
    began,
    ended: end ?? null,
    spend: null,
    state: "idle",
    ...rest,
  };
  // A command that runs holds what it printed so far beside its act, as a window holds it.
  const printed =
    kind === "bash" && made.phase.phase !== "done" && value !== null ? { printed: value as Exit } : {};
  return { ...made, ...printed, state: rest.state ?? stateOf(made, unfinished) };
}

/** An act with some of its parts changed, in the state that its parts give it then. */
export function changed(one: Act, more: More): Act {
  const value = more.value !== undefined ? more.value : one.phase.phase === "done" ? one.phase.value : null;
  const done = more.done ?? (more.ended != null ? true : one.phase.phase === "done");
  return act(one.id, one.kind, one.chain, one.by, one.began, undefined, {
    words: one.words,
    word: one.word,
    asking: one.asking,
    operator: one.operator,
    pause: one.pause,
    run: one.run,
    spend: one.spend,
    ...(one.on !== one.chain ? { thread: one.on } : {}),
    ...(done ? { done: true, ended: one.ended ?? one.began, value } : {}),
    ...more,
    ...(more.paused !== undefined ? { pause: more.paused ? { by: "operator" } : null } : {}),
  });
}

/** The acts that each act made, in the order of the acts. */
export function makers(acts: readonly Act[]): Map<string, Act[]> {
  const made = new Map<string, Act[]>();
  for (const one of acts) {
    const list = made.get(one.by);
    if (list) list.push(one);
    else made.set(one.by, [one]);
  }
  return made;
}

/** The index of the acts of a life that a host holds, as thread reads it: the acts on a chain but its chains, the acts
 * that each act made, each act by its name, and the acts that live. */
export function actIndex(acts: readonly Act[], chain: string) {
  return {
    on: acts.filter((one) => one.chain === chain && one.kind !== "chain"),
    made: makers(acts),
    ids: new Map(acts.map((one) => [one.id, one])),
    live: new Map(
      acts
        .filter((one) => one.phase.phase !== "done" || one.run?.run === "running")
        .map((one) => [one.id, one]),
    ),
  };
}
