import type { ActRow } from "../src/session.ts";

/** An act of a life, made at a moment, and done at another when it is done. */
export function act(
  id: string,
  kind: string,
  on: string,
  by: string,
  began: number,
  ended?: number,
  more: Partial<ActRow> = {},
): ActRow {
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
  } as ActRow;
}
