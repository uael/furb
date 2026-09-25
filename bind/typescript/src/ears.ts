import { Life } from "../index.cjs";
import type { Entry } from "./types.js";

/** One verb of the engine that an ear says while it hears, say and act among them, whose value the ear is given
 * back: an ear of the host hears on the thread of the life, and so speaks through what it yields. */
export interface Call {
  verb: string;
  args?: unknown[];
  kwargs?: Record<string, unknown>;
}
export type Ear = Generator<Call | null | undefined, void, unknown>;
/** What an error is as the engine reads it: an exception of the engine as it came, and anything else as a refusal
 * that says it. */
export const fault = (error: unknown) =>
  error && typeof error === "object" && "is" in error && "args" in error
    ? error
    : { is: "Refused", args: [error instanceof Error ? error.message : String(error)] };
const synchronous = (value: unknown) => {
  if (value && typeof value === "object" && "then" in value && typeof value.then === "function")
    throw new Error("A callable of the host must answer synchronously.");
  return value ?? null;
};

/** JavaScript generators use the same crossing as Python generators, including nested bus calls. */
export class Ears {
  private readonly ears = new Map<string, Ear>();
  private readonly functions = new Map<string, (...args: never[]) => unknown>();
  private serial = 0;
  /** The ears by name, which boot is given in this order, the order in which the engine offers them a question. */
  constructor(ears: Record<string, Ear>) {
    for (const [name, ear] of Object.entries(ears)) this.ears.set(name, ear);
  }
  callable(callback: (...args: never[]) => unknown): { is: "callable"; name: string } {
    const name = `call:${++this.serial}`;
    this.functions.set(name, callback);
    return { is: "callable", name };
  }
  callback = (request: unknown[]): unknown => {
    const [kind, name, value] = request;
    try {
      if (kind === "called") {
        const [, , , kwargs] = request;
        const callback = this.functions.get(String(name));
        if (!callback) throw new Error(`Unknown callable ${name}.`);
        return synchronous(
          callback(...(value as never[]), ...(Object.keys(kwargs as object).length ? [kwargs as never] : [])),
        );
      }
      const ear = this.ears.get(String(name));
      if (!ear) throw new Error(`Unknown ear ${name}.`);
      const answer = value as [string, unknown];
      const next =
        kind === "answered"
          ? answer[0] === "raised"
            ? ear.throw(answer[1])
            : ear.next(answer[1])
          : ear.next(value);
      if (next.done) return ["over"];
      if (!next.value) return null;
      return ["calls", next.value.verb, next.value.args ?? [], next.value.kwargs ?? {}];
    } catch (error) {
      return { error: fault(error) };
    }
  };
  boot(record: Entry[] = []): Life {
    return Life.boot(this.callback, [...this.ears.keys()], record);
  }
}
