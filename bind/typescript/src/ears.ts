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
    | "Reply"
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
const fault = (error: unknown) =>
  error && typeof error === "object" && "is" in error && "args" in error
    ? error
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
  /** The ears by name, and the names of the ears of the outside, which boot is given in the order the engine offers
   * them an act; any other ear is one that an ear drives as an ear of the engine. */
  constructor(
    ears: Record<string, Ear>,
    private readonly outside = Object.keys(ears),
  ) {
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
  boot(record: Entry[] = []): Life {
    return Life.boot(this.callback, this.outside, record);
  }
}

/** A ready adapter for the crate's World operations. Async work says its result later on the same life. */
export class WorldAdapter {
  life?: Life;
  stopped = false;
  /** The ears the life boots on, whose callable carries a show or a filter of the host into the life. */
  readonly ears: Ears;
  /** Each command the World runs, whether its stderr goes with its stdout, and its two streams as they came, which
   * the World answers the command with when it ends. */
  private readonly running = new Map<string, [merged: boolean, stdout: string, stderr: string]>();
  /** The actor whose last reply on each chain answered nothing, so a second such reply in a row pauses the chain. */
  private readonly mute = new Map<string, string>();
  constructor(
    readonly handle: WorldHandler,
    readonly onFacts?: (facts: Fact[]) => void,
    readonly onFault?: (error: unknown) => void,
    /** Hears each fact, and may ask the life while it hears through the calls it yields. */
    readonly onFact?: (fact: Fact) => Generator<Call, void, unknown> | undefined,
  ) {
    const owner = this;
    let queued = false;
    const facts: Fact[] = [];
    const observer = (function* (): Ear {
      for (;;) {
        const fact = (yield null) as Fact;
        if (fact) {
          facts.push(fact);
          const hearing = onFact?.(fact);
          if (hearing) yield* hearing;
        }
        if (!queued) {
          queued = true;
          queueMicrotask(() => {
            queued = false;
            if (!owner.stopped && facts.length) onFacts?.(facts.splice(0));
          });
        }
      }
    })();
    this.ears = new Ears({ world: this.world(), typescript: observer }, ["world"]);
  }

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

  /** One fact, said as the World, which is how a command says what it writes. */
  say(kind: string, id: string, words: unknown[]): void {
    this.speak(() => this.life?.say(kind, id, words));
  }

  /** A command ended, with its code, or with none at its timeout: the World says it done with the Exit of the
   * streams it heard, unless a control ended it first. */
  exited(id: string, code: number | null): void {
    const held = this.running.get(id);
    if (!held) return;
    this.running.delete(id);
    const [, stdout, stderr] = held;
    const text = (stream: string, content: string) => ({ is: "Text", path: `${id}/${stream}`, content });
    this.say("done", id, [
      { is: "Exit", code, stdout: text("stdout", stdout), stderr: text("stderr", stderr) },
    ]);
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
        });
    });
  }
  private *world(): Ear {
    // An ear of the outside hears an act only when no ear before it took it, so the observer, which keeps every fact
    // and every act, is an ear of the engine, which the World drives as it is born, before the record is said again.
    yield { verb: "drive", args: [{ is: "ear", name: "typescript", started: false }, "typescript"] };
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
      else if (kind === "reply") {
        yield ["started", id];
        const [chain, actor] = [String(words[0]), String(words[1])];
        const turns = yield { verb: "turns", kwargs: { on: chain } };
        this.later(
          { kind: "Reply", args: [id, fact[2], chain, actor, turns] },
          (value) => {
            if (Array.isArray(value) && Array.isArray(value[2]) && typeof value[2][4] === "number") {
              const turn = [...value];
              turn[2] = value[2].map((part, index) =>
                index === 4 ? { is: "float", args: [String(part)] } : part,
              );
              value = turn;
            }
            this.mute.delete(chain);
            if (!this.life?.outcome(id).done) this.life?.say("done", id, [value]);
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
            this.life?.say("done", id, [{ is: "Refused", args: [`${actor} answered nothing: ${why}`] }]);
          },
        );
      } else if (kind === "bash") {
        yield ["started", id];
        const [on, command, fed, timeout] = words;
        const here = yield { verb: "cwd", kwargs: { on } };
        const merged = Boolean(yield { verb: "ask", args: ["merged", on, id] });
        this.running.set(id, [merged, "", ""]);
        try {
          synchronous(this.handle({ kind: "Run", args: [{ id, here, command, fed, timeout, merged }] }));
        } catch (error) {
          this.running.delete(id);
          yield ["done", id, fault(error)];
        }
      } else if (kind === "wait") {
        yield ["started", id];
        this.later(
          { kind: "Wait", args: [words[1], id] },
          () => {
            if (!this.life?.outcome(id).done) this.life?.say("done", id, [null]);
          },
          (error) => this.life?.say("done", id, [fault(error)]),
        );
      } else if (kind === "prompt") {
        yield ["started", id];
        this.later(
          { kind: "Prompt", args: [id, words[1], words[2]] },
          (value) => this.life?.close(value, id),
          (error) => this.life?.close(fault(error), id),
        );
      } else if (kind === "out") {
        const held = this.running.get(id);
        if (held) held[held[0] || words[1] === "stdout" ? 1 : 2] += String(words[0]);
      } else if (kind === "feed") synchronous(this.handle({ kind: "Feed", args: [id, words[0]] }));
      else if (kind === "done") this.running.delete(id);
      else if (kind === "cancel" || kind === "close") {
        for (const running of [...this.running.keys()]) {
          if (yield { verb: "covers", args: [fact, running] }) {
            synchronous(this.handle({ kind: "Slay", args: [running] }));
            this.running.delete(running);
          }
        }
      }
    }
  }
  boot(record: Entry[] = []): Life {
    this.life = this.ears.boot(record);
    return this.life;
  }
}
