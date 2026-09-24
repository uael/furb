import { resolve } from "node:path";
import { configDirectory, Life, type Opening } from "../index.cjs";
import type { FileChange } from "./changes.js";
import type { Call, Fault, Hearing, Saying, WorldContext, WorldPart } from "./extension.js";
import { spawnShell } from "./shell.js";
import type { Entry, Fact } from "./types.js";

/** What the World does for every life, which a host answers: the standing, the clock, chance, a keep of the record,
 * the turn of a model for an ask, and the work of a wait and of a prompt to the operator. */
export interface WorldRequest {
  kind: "Stand" | "Clock" | "Chance" | "Keep" | "Ask" | "Wait" | "Prompt";
  args: unknown[];
}
export type WorldHandler = (request: WorldRequest) => unknown;
export type Ear = Generator<Saying | Call | null | undefined, void, unknown>;
const fault = (error: unknown): Fault =>
  error && typeof error === "object" && "is" in error && "args" in error
    ? (error as Fault)
    : { is: "Refused", args: [error instanceof Error ? error.message : String(error)] };
const synchronous = (value: unknown) => {
  if (value && typeof value === "object" && "then" in value && typeof value.then === "function")
    throw new Error("A request to the World or a callable of the host must answer synchronously.");
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
      if (Array.isArray(next.value)) return ["say", next.value];
      return ["calls", next.value.verb, next.value.args ?? [], next.value.kwargs ?? {}];
    } catch (error) {
      return { error: fault(error) };
    }
  };
  /** The life on these ears, from the record and what it opens with. */
  boot(record: Entry[] = [], opening?: Opening): Life {
    return Life.boot(this.callback, [...this.ears.keys()], record, opening);
  }
}

/** What an adapter is given beside its handler. */
export interface AdapterOptions {
  /** Hears the facts of the life, in batches, after the ears. */
  onFacts?: (facts: Fact[]) => void;
  /** Hears a failure that no act can take. */
  onFault?: (error: unknown) => void;
  /** Hears each fact, and may ask the life while it hears through the calls it yields. */
  onFact?: (fact: Fact) => Generator<Call, void, unknown> | undefined;
  /** An inspection of a record: the parts hear nothing. */
  readOnly?: boolean;
}

/** A ready adapter for the crate's World operations. It does what every World does through its handler, and hands
 * every fact to its parts, in order. Async work says its result later on the same life. */
export class WorldAdapter {
  life?: Life;
  stopped = false;
  /** The parts of the extensions for this World, which hear every fact after the World. */
  parts: WorldPart[] = [];
  /** The ears the life boots on, whose callable carries a show or a filter of the host into the life. */
  readonly ears: Ears;
  /** The actor whose last ask on each chain answered nothing, so a second such ask in a row pauses the chain. */
  private readonly mute = new Map<string, string>();
  private delivery: Promise<unknown> = Promise.resolve();
  constructor(
    readonly handle: WorldHandler,
    readonly options: AdapterOptions = {},
  ) {
    const owner = this;
    let queued = false;
    const facts: Fact[] = [];
    const observer = (function* (): Ear {
      for (;;) {
        const fact = (yield null) as Fact;
        if (fact) {
          facts.push(fact);
          const hearing = options.onFact?.(fact);
          if (hearing) yield* hearing;
        }
        if (!queued) {
          queued = true;
          queueMicrotask(() => {
            queued = false;
            if (!owner.stopped && facts.length) options.onFacts?.(facts.splice(0));
          });
        }
      }
    })();
    this.ears = new Ears({ world: this.world(), typescript: observer });
  }

  private get closed(): boolean {
    return this.stopped || this.life?.disposed === true;
  }

  /** The kinds of act this World does: a wait, a prompt, and the kinds of its parts. */
  get kinds(): Set<string> {
    return new Set(["wait", "prompt", ...this.parts.flatMap((part) => part.kinds ?? [])]);
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

  /** One fact said later, as the World, after every saying before it. */
  send(kind: string, id: string, words: unknown[]): void {
    this.delivery = this.delivery
      .then(() => {
        if (!this.closed) this.life?.send(kind, id, words, "world");
      })
      .catch((error) => this.options.onFault?.(error));
  }

  /** An act closed later, as the World, after every saying before it, unless it is done. */
  close(value: unknown, id: string): void {
    this.delivery = this.delivery
      .then(() => {
        if (!this.closed && !this.life?.outcome(id).done) this.speak(() => this.life?.close(value, id));
      })
      .catch((error) => this.options.onFault?.(error));
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
            this.options.onFault?.(failure);
          }
        });
    });
  }
  private *world(): Ear {
    for (;;) {
      const fact = (yield null) as Fact;
      if (!fact) continue;
      const [kind, id, , ...words] = fact;
      let act: Fact | undefined;
      if (["stand", "clock", "chance"].includes(kind)) {
        let value: unknown;
        try {
          value = synchronous(
            this.handle({
              kind: (kind.charAt(0).toUpperCase() + kind.slice(1)) as WorldRequest["kind"],
              args: [],
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
        } catch (error) {
          value = fault(error);
        }
        yield ["done", id, value];
      } else if (kind === "keep") synchronous(this.handle({ kind: "Keep", args: [words[0]] }));
      else if (kind === "ask") {
        const [chain, actor] = [String(words[0]), String(words[1])];
        this.later(
          { kind: "Ask", args: [id, ...words] },
          (value) => {
            if (Array.isArray(value) && Array.isArray(value[2]) && typeof value[2][4] === "number") {
              const reply = [...value];
              reply[2] = value[2].map((part, index) =>
                index === 4 ? { is: "float", args: [String(part)] } : part,
              );
              value = reply;
            }
            this.mute.delete(chain);
            if (!this.life?.outcome(id).done) this.life?.send("answer", id, [value], "world");
          },
          (error) => {
            if (this.mute.get(chain) === actor) this.life?.pause(chain);
            this.mute.set(chain, actor);
            const why =
              error instanceof Error
                ? `${error.name}: ${error.message}`
                : error && typeof error === "object" && "is" in error && "args" in error
                  ? `${error.is}: ${Array.isArray(error.args) ? error.args.map(String).join(", ") : String(error.args)}`
                  : `Refused: ${String(error)}`;
            this.life?.close({ is: "Refused", args: [`${actor} answered nothing: ${why}`] }, id);
          },
        );
      } else if (kind === "start") {
        act = (yield { verb: "get", args: [id] }) as Fact;
        if (!act) continue;
        if (act[0] === "wait" || act[0] === "prompt") this.started(act);
        else if (!this.kinds.has(act[0]))
          yield { verb: "close", args: [{ is: "Refused", args: [`the World does no ${act[0]}`] }, id] };
      }
      if (this.options.readOnly) continue;
      for (const part of this.parts) {
        try {
          const hearing = part.hears?.(fact);
          if (hearing) yield* hearing;
        } catch (error) {
          // Nothing else ends an act whose part failed at its start, so it closes with why.
          if (act && part.kinds?.includes(act[0])) yield { verb: "close", args: [fault(error), id] };
          else this.options.onFault?.(error);
        }
      }
    }
  }
  /** The outside work of a wait or of a prompt to the operator, which the start of the act says: at its birth, or at
   * the first wake over it that this life says, for one the record shows begun and not done. */
  private started(act: Fact): void {
    const id = act[1];
    if (act[0] === "wait")
      this.later(
        { kind: "Wait", args: [act[4], id] },
        () => {
          if (!this.life?.outcome(id).done) this.life?.send("done", id, [null], "world");
        },
        (error) => this.life?.close(fault(error), id),
      );
    else
      this.later(
        { kind: "Prompt", args: [id, act[4], act[5]] },
        (value) => {
          if (!this.life?.outcome(id).done) this.life?.close(value, id);
        },
        (error) => {
          if (!this.life?.outcome(id).done) this.life?.close(fault(error), id);
        },
      );
  }
  /** The life on this World, from the record and what it opens with. */
  boot(record: Entry[] = [], opening?: Opening): Life {
    this.life = this.ears.boot(record, opening);
    return this.life;
  }
}

/** What a World gives the parts of its extensions, over the adapter it speaks through. */
export function worldContext(
  adapter: WorldAdapter,
  given: { directory: string; change: (change: FileChange) => void },
): WorldContext {
  return {
    ...given,
    config: configDirectory(),
    *where(on: string): Hearing<string> {
      const [, here] = (yield { verb: "ask", args: ["cwd", on] }) as [unknown, unknown];
      return String(here);
    },
    at(here: string, path = ""): string {
      if (path.includes("://")) throw new Error(`No file at ${path}.`);
      return resolve(given.directory, here, path);
    },
    speak: (kind, about, ...words) => adapter.send(kind, about, words),
    close: (value, id) => adapter.close(value, id),
    spawn: spawnShell,
    refused: (message) => ({ is: "Refused", args: [message] }),
  };
}
