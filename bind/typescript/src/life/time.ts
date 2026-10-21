import { failed, type LiveAct } from "./acts.js";
import { asksOperator } from "./loom.js";

/** A mark on the axis of time of a life: the moment just after it, what it is, and its act.
 *
 * - `step`: a rung starts a step.
 * - `asks`: a prompt asks the operator.
 * - `done`: a thread ends well, or a cancel ends it.
 * - `failed`: a thread fails.
 */
export interface Mark {
  at: number;
  kind: "step" | "asks" | "done" | "failed";
  act: string;
}

/** The marks of a life on its axis of time: each rung that starts a step, each question to the operator, and each end
 * of a thread, which is a prompt of the tree of threads, in the order of time. */
export function marks(acts: readonly LiveAct[], threads: ReadonlySet<string>): Mark[] {
  const out: Mark[] = [];
  for (const act of acts) {
    if (act.kind === "rung") out.push({ at: act.began, kind: "step", act: act.id });
    if (asksOperator(act)) out.push({ at: act.began, kind: "asks", act: act.id });
    if (act.kind === "prompt" && threads.has(act.id) && act.ended !== undefined)
      out.push({ at: act.ended, kind: failed(act) ? "failed" : "done", act: act.id });
  }
  return out.sort((one, other) => one.at - other.at);
}
