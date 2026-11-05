import type { LiveAct } from "../../src/life/acts.ts";

/** An act of a life, made at a moment, and done at another when it is done. */
export function act(
  id: string,
  kind: string,
  on: string,
  by: string,
  began: number,
  ended?: number,
  more: Partial<LiveAct> = {},
): LiveAct {
  return {
    id,
    kind,
    by,
    on: kind === "chain" ? id : on,
    words: [],
    done: ended !== undefined,
    value: null,
    paused: false,
    began,
    ended,
    ...(kind === "rung" ? { run: { status: ended === undefined ? "running" : "done", reason: "" } } : {}),
    ...more,
  } as LiveAct;
}

/** The acts that each act made, in the order of the acts. */
export function makers(acts: readonly LiveAct[]): Map<string, LiveAct[]> {
  const made = new Map<string, LiveAct[]>();
  for (const one of acts) made.set(one.by, [...(made.get(one.by) ?? []), one]);
  return made;
}

/** The index of the acts of a life that a host holds, as thread reads it: the acts on a chain but its chains, the acts
 * that each act made, each act by its name, and the acts that live. */
export function indexOf(acts: readonly LiveAct[], chain: string) {
  return {
    on: acts.filter((one) => one.on === chain && one.kind !== "chain"),
    made: makers(acts),
    ids: new Map(acts.map((one) => [one.id, one])),
    live: acts.filter((one) => !one.done || one.run?.status === "running"),
  };
}
