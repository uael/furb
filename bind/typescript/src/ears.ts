import { Life } from "../index.cjs";
import type { Entry, Fact } from "./types.js";

export interface WorldRequest {
  kind:
    | "Stand"
    | "Read"
    | "Write"
    | "Clock"
    | "Chance"
    | "Keep"
    | "Ask"
    | "Run"
    | "Feed"
    | "Slay"
    | "Wait"
    | "Prompt";
  args: unknown[];
}
export type WorldHandler = (request: WorldRequest) => unknown;
export type Saying = [kind: string, about: string, ...words: unknown[]];
export interface Call {
  verb: string;
  args?: unknown[];
  kwargs?: Record<string, unknown>;
}
export type Ear = Generator<Saying | Call | null | undefined, void, unknown>;
const transcripts = new WeakMap<Fact, string[]>();
const fault = (error: unknown) =>
  error && typeof error === "object" && "is" in error && "args" in error
    ? error
    : { is: "Refused", args: [error instanceof Error ? error.message : String(error)] };
const synchronous = (value: unknown) => {
  if (value && typeof value === "object" && "then" in value && typeof value.then === "function")
    throw new Error("This World query must answer synchronously.");
  return value ?? null;
};

/** JavaScript generators use the same crossing as Python generators, including nested bus calls. */
export class Ears {
  private readonly ears = new Map<string, Ear>();
  private readonly functions = new Map<string, (...args: never[]) => unknown>();
  private serial = 0;
  constructor(ears: Record<string, Ear>) {
    for (const [name, ear] of Object.entries(ears)) this.ears.set(name, ear);
  }
  callable(callback: (...args: never[]) => unknown): { is: "callable"; name: string } {
    const name = `call:${++this.serial}`;
    this.functions.set(name, callback);
    return { is: "callable", name };
  }
  callback = ([kind, name, value, kwargs]: unknown[]): unknown => {
    try {
      if (kind === "called") {
        const callback = this.functions.get(String(name));
        if (!callback) throw new Error(`Unknown callable ${name}.`);
        return synchronous(
          callback(...(value as never[]), ...(Object.keys(kwargs as object).length ? [kwargs as never] : [])),
        );
      }
      const ear = this.ears.get(String(name));
      if (!ear) throw new Error(`Unknown ear ${name}.`);
      if (kind === "hears" && Array.isArray(value) && Array.isArray(kwargs))
        transcripts.set(value as Fact, kwargs as string[]);
      const answer = value as [string, unknown];
      const next =
        kind === "answered"
          ? answer[0] === "raised"
            ? ear.throw(answer[1])
            : ear.next(answer[1])
          : ear.next(value);
      if (next.done) return ["over"];
      if (!next.value) return null;
      if (Array.isArray(next.value)) return ["say", next.value];
      return ["calls", next.value.verb, next.value.args ?? [], next.value.kwargs ?? {}];
    } catch (error) {
      return { error: fault(error) };
    }
  };
  boot(record: Entry[] = []): Life {
    return Life.boot(this.callback, [...this.ears.keys()], record);
  }
}

/** A ready adapter for the crate's World operations. Async work says its result later on the same life. */
export class WorldAdapter {
  life?: Life;
  stopped = false;
  private readonly running = new Set<string>();
  constructor(
    readonly handle: WorldHandler,
    readonly onFacts?: (facts: Fact[]) => void,
    readonly onFault?: (error: unknown) => void,
  ) {}

  private get closed(): boolean {
    return this.stopped || this.life?.disposed === true;
  }

  private speak(action: () => void): void {
    if (!this.life || this.closed) return;
    const previous = this.life.site("world");
    try {
      action();
    } finally {
      if (!this.life.disposed) this.life.site(previous);
    }
  }

  private later(request: WorldRequest, done: (value: unknown) => void, fail: (error: unknown) => void): void {
    queueMicrotask(() => {
      if (this.closed) return;
      Promise.resolve()
        .then(() => this.handle(request))
        .then((value) => {
          if (!this.closed) this.speak(() => done(value));
        })
        .catch((error) => {
          if (this.closed) return;
          const id = request.kind === "Wait" ? request.args[1] : request.args[0];
          if (typeof id === "string" && this.life?.outcome(id).done) return;
          try {
            this.speak(() => fail(error));
          } catch (failure) {
            this.onFault?.(failure);
          }
          this.onFault?.(error);
        });
    });
  }
  private *world(): Ear {
    for (;;) {
      const fact = (yield null) as Fact;
      if (!fact) continue;
      const [kind, id, , ...words] = fact;
      if (["stand", "clock", "chance", "read", "write"].includes(kind)) {
        let value: unknown;
        try {
          const here =
            kind === "read" || kind === "write" ? yield { verb: "cwd", kwargs: { on: words[0] } } : null;
          const args =
            kind === "read"
              ? [here, words[1]]
              : kind === "write"
                ? [here, (words[1] as { path: string }).path, (words[1] as { content: string }).content]
                : [];
          value = synchronous(
            this.handle({
              kind: (kind.charAt(0).toUpperCase() + kind.slice(1)) as WorldRequest["kind"],
              args,
            }),
          );
          if (
            (kind === "clock" || kind === "chance") &&
            (typeof value !== "number" ||
              !Number.isFinite(value) ||
              (kind === "chance" && (value < 0 || value >= 1)))
          )
            throw new Error(`Invalid ${kind} from the World.`);
          if (kind === "clock" || kind === "chance") value = { is: "float", args: [String(value)] };
          if (kind === "read" || kind === "write") value = { is: "Text", ...(value as object) };
        } catch (error) {
          value = fault(error);
        }
        yield ["done", id, value];
      } else if (kind === "keep") synchronous(this.handle({ kind: "Keep", args: [words[0]] }));
      else if (kind === "ask") {
        this.later(
          { kind: "Ask", args: [id, ...words, transcripts.get(fact)] },
          (value) => {
            if (Array.isArray(value) && Array.isArray(value[2]) && typeof value[2][4] === "number") {
              const reply = [...value];
              reply[2] = value[2].map((part, index) =>
                index === 4 ? { is: "float", args: [String(part)] } : part,
              );
              value = reply;
            }
            if (!this.life?.outcome(id).done) this.life?.send("answer", id, [value], "world");
          },
          (error) => {
            const actor = String(words[1]);
            const mute = `${actor} answered nothing`;
            const turns = words[2] as import("./types.js").Turn[];
            const repeated = turns
              .at(-1)?.[1]
              .some((tag) => typeof tag !== "string" && String(tag[2]).includes(mute));
            if (repeated) this.life?.pause(String(words[0]));
            this.life?.close(
              { is: "Refused", args: [`${mute}: ${error instanceof Error ? error.message : String(error)}`] },
              id,
            );
          },
        );
      } else if (kind === "start") {
        const act = (yield { verb: "get", args: [id] }) as Fact;
        if (!act) continue;
        if (act[0] === "wait")
          this.later(
            { kind: "Wait", args: [act[4], id] },
            () => {
              if (!this.life?.outcome(id).done) this.life?.send("done", id, [null], "world");
            },
            (error) => this.life?.close(fault(error), id),
          );
        else if (act[0] === "prompt")
          this.later(
            { kind: "Prompt", args: [id, act[4], act[5]] },
            (value) => {
              if (!this.life?.outcome(id).done) this.life?.close(value, id);
            },
            (error) => {
              if (!this.life?.outcome(id).done) this.life?.close(fault(error), id);
            },
          );
        else if (act[0] === "bash") {
          const here = yield { verb: "cwd", kwargs: { on: act[3] } };
          const [, merged] = (yield { verb: "ask", args: ["merged", act[3], id] }) as [unknown, boolean];
          this.running.add(id);
          try {
            synchronous(
              this.handle({
                kind: "Run",
                args: [{ id, here, command: act[4], fed: act[5], timeout: act[6], merged }],
              }),
            );
          } catch (error) {
            yield { verb: "close", args: [fault(error), id] };
          }
        }
      } else if (kind === "feed") synchronous(this.handle({ kind: "Feed", args: [id, words[0]] }));
      else if (kind === "exited") this.running.delete(id);
      else if (kind === "cancel" || kind === "close") {
        for (const running of this.running) {
          const affected = yield { verb: "covers", args: [fact, running] };
          if (affected) {
            synchronous(this.handle({ kind: "Slay", args: [running] }));
            this.running.delete(running);
          }
        }
      }
    }
  }
  boot(record: Entry[] = []): Life {
    const owner = this;
    let queued = false;
    const facts: Fact[] = [];
    const observer = (function* (): Ear {
      for (;;) {
        const fact = (yield null) as Fact;
        if (fact) facts.push(fact);
        if (!queued) {
          queued = true;
          queueMicrotask(() => {
            queued = false;
            if (!owner.stopped && facts.length) owner.onFacts?.(facts.splice(0));
          });
        }
      }
    })();
    this.life = new Ears({ world: this.world(), typescript: observer }).boot(record);
    return this.life;
  }
}
