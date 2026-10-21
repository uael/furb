import type { Engine } from "../index.cjs";
import { over } from "./ears.js";
import { Session } from "./session.js";
import { type Fact, isQuestion } from "./types.js";

/** The kinds of the acts that a program makes, which a later run of it finds again in the record. */
const MADE = ["chain", "prompt", "grant", "rung"];

export interface LifeOptions {
  /** The model a prompt goes to when it names none, as the catalog of the crate names it. */
  model?: string;
  /** The models the life offers beside that one. */
  roster?: string[];
  /** The directory the life stands on; the directory of the process when unsaid. */
  directory?: string;
  /** The claude command line to run, in place of the one that `FURB_CLAUDE_BIN` names or this machine holds. */
  claude?: string;
  /** Whether the life enables the extensions that the configs turn on; true when unsaid. */
  extensions?: boolean;
}

/** One act of a chain, with named fields: its kind, its name, who made it, its chain, its words, and what it came to
 * once it is done. */
export interface LifeAct {
  kind: string;
  id: string;
  by: string;
  on: string;
  words: unknown[];
  done: boolean;
  value: unknown;
}

/** One chain of a life: a conversation with a model of its own, which a program prompts and puts a ceiling on. */
export class Chain {
  constructor(
    readonly life: Life,
    readonly id: string,
  ) {}

  protected get engine(): Engine {
    return this.life.session.engine as Engine;
  }

  /** The label that the chain was opened with. */
  get label(): string {
    return String(this.engine.get(this.id)?.[4]);
  }

  /** The chain that this chain was opened on, and none for the root. */
  get parent(): Chain | undefined {
    const on = String(this.engine.get(this.id)?.[3] ?? "");
    return on ? new Chain(this.life, on) : undefined;
  }

  /** The actor that a prompt of this chain goes to when it names none. */
  get actor(): string {
    for (const [kind, , by, on, word] of this.engine.transcript({ on: this.id }).reverse())
      if (kind === "rung" && by === "operator" && on === this.id && String(word).startsWith("actor = "))
        return String(word)
          .slice("actor = ".length)
          .replace(/^['"]|['"]$/g, "");
    return String((this.engine.standing() as unknown[])[2]);
  }

  /** The dollars of the answers of the models on this chain. */
  get spent(): number {
    let sum = 0;
    for (const [kind, id, , value] of this.engine.transcript({ on: this.id })) {
      const usage = Array.isArray(value) ? value[2] : undefined;
      if (kind === "done" && isQuestion("reply", id) && Array.isArray(usage)) sum += Number(usage[4]);
    }
    return sum;
  }

  /** A thread on this chain: the message to the actor, which answers it with a value of the shape. */
  prompt<T = unknown>(shape: string, message: string, options: { to?: string } = {}): Thread<T> {
    const to = options.to ?? "";
    const found = this.life.again("prompt", this.id, [shape, message, to]);
    return new Thread<T>(this, found ?? this.engine.prompt(shape, { message, to, on: this.id }).id);
  }

  /** A new chain on this one, whose prompts go to the model named, or to the default actor. */
  chain(label: string, options: { model?: string } = {}): Chain {
    const id = this.life.again("chain", this.id, [label, ""]) ?? this.engine.chain({ label, on: this.id }).id;
    if (options.model !== undefined) {
      const word = `actor = ${JSON.stringify(options.model)}`;
      if (!this.life.again("rung", id, [word, "", ""])) this.engine.rung({ word, on: id });
    }
    return new Chain(this.life, id);
  }

  /** A ceiling on this chain, in dollars or in a share of the window, which pauses the chain when it reaches it. */
  grant(options: { usd?: number; share?: number }): void {
    const { usd, share } = options;
    if (!this.life.again("grant", this.id, [usd ?? null, share ?? null]))
      this.engine.grant({ usd, share, on: this.id });
  }

  /** Whether a pause holds this chain. */
  isPaused(): boolean {
    return this.life.session.isPaused(this.id);
  }

  /** Wait until a pause holds this chain, as a grant says one at its ceiling. */
  paused(): Promise<void> {
    return this.life.until(() => this.isPaused());
  }

  /** Wake this chain, so its work goes on. */
  wake(): void {
    this.engine.wake(this.id);
  }

  /** Every chain under this one, each before the chains under it. */
  chains(): Chain[] {
    return this.engine
      .transcript({ on: this.id })
      .filter(([kind, id, , on]) => kind === "chain" && isQuestion(kind, id) && on === this.id)
      .flatMap(([, id]) => {
        const child = new Chain(this.life, id);
        return [child, ...child.chains()];
      });
  }

  /** Every act on this chain, in order, with named fields. */
  acts(): LifeAct[] {
    return this.engine
      .transcript({ on: this.id })
      .filter(([kind, id, , on]) => isQuestion(kind, id) && on === this.id)
      .map(([kind, id, by, on, ...words]) => ({
        kind,
        id,
        by,
        on: String(on),
        words,
        done: over(this.engine, id),
        value: this.engine.peek(id),
      }));
  }
}

/** One prompt of a chain, which a program awaits for its value: the name of the prompt, and the door of its ladder,
 * whose plan is the words of the models that answer it. */
export class Thread<T = unknown> implements PromiseLike<T> {
  constructor(
    readonly chain: Chain,
    readonly id: string,
  ) {}

  private get engine(): Engine {
    return this.chain.life.session.engine as Engine;
  }

  // biome-ignore lint/suspicious/noThenProperty: a thread is awaited for its value.
  then<A = T, B = never>(
    done?: ((value: T) => A | PromiseLike<A>) | null,
    failed?: ((reason: unknown) => B | PromiseLike<B>) | null,
  ): Promise<A | B> {
    return this.engine.result<T>(this.id).then(done, failed);
  }

  toString(): string {
    return this.id;
  }

  /** Whether the thread has its value. */
  get done(): boolean {
    return over(this.engine, this.id);
  }

  /** The words of the rungs of this thread, as python, in order. */
  get plan(): string {
    const words = this.engine.program({ on: this.chain.id }) as Record<string, string>;
    return Object.entries(words)
      .filter(([whose]) => this.engine.verb("under", [whose, this.id]) === true)
      .map(([, word]) => word)
      .join("\n");
  }

  /** Wait until the plan of the thread changes, as it does when a model answers it with a word. */
  step(): Promise<void> {
    const before = this.plan;
    return this.chain.life.until(() => this.done || this.plan !== before);
  }

  /** Write the door of the thread with a new plan: the chain cancels each word that the plan drops, and runs the new
   * words. */
  edit(plan: string): void {
    this.engine.write({ path: this.id, content: plan }, { on: this.chain.id });
  }
}

/** One prompt to the operator, which the program answers. */
export class Question {
  constructor(
    readonly life: Life,
    readonly id: string,
    readonly chain: Chain,
    readonly shape: string,
    readonly message: string,
  ) {}

  /** Answer the question with a line that the console reads in its shape. */
  answer(line: string): void {
    this.life.session.console.answer(this.id, line);
  }
}

/** One life of the engine: the root chain, the questions to the operator, and the session that serves it. */
export class Life extends Chain {
  private made: Fact[];
  private readonly waiting: Array<() => void> = [];

  constructor(readonly session: Session) {
    super(undefined as unknown as Life, (session.engine as Engine).root);
    (this as { life: Life }).life = this;
    this.made = session.entries
      .map(([fact]) => fact)
      .filter(
        ([kind, id, by]) =>
          by === "operator" && MADE.includes(kind) && isQuestion(kind, id) && id !== "chain1",
      );
    session.on("change", () => {
      for (const wake of this.waiting.splice(0)) wake();
    });
  }

  /** The act of the record that a program made at this place, when it agrees with this one. */
  again(kind: string, on: string, words: unknown[]): string | undefined {
    const [one] = this.made;
    if (!one || one[0] !== kind || one[3] !== on || JSON.stringify(one.slice(4)) !== JSON.stringify(words))
      return undefined;
    this.made.shift();
    return one[1];
  }

  /** Wait until a condition of the life holds, which the life checks again after each change. */
  async until(holds: () => boolean): Promise<void> {
    while (!holds()) await new Promise<void>((wake) => this.waiting.push(wake));
  }

  /** The questions to the operator, as the models ask them, until the thread named is done. */
  async *questions(options: { until?: Thread } = {}): AsyncGenerator<Question> {
    const seen = new Set<string>();
    const asked = () => [...this.session.console.prompts.values()].filter(({ id }) => !seen.has(id));
    const { until } = options;
    while (!until?.done) {
      const [next] = asked();
      if (!next) {
        await this.until(() => asked().length > 0 || (until?.done ?? false));
        continue;
      }
      seen.add(next.id);
      const on = String(this.engine.get(next.id)?.[3]);
      yield new Question(this, next.id, new Chain(this, on), next.shape, next.message);
    }
  }

  /** End the life: its commands, its waits and its models end, and the store lets its record go. */
  end(): Promise<void> {
    return this.session.dispose();
  }

  [Symbol.asyncDispose](): Promise<void> {
    return this.end();
  }
}

/** A life on its record, or a new one when the record holds nothing, whose prompts go to the model named, or to the
 * default actor of the crate. Its pending work resumes at once. */
export async function open(record?: string, options: LifeOptions = {}): Promise<Life> {
  const { model, roster, directory, claude, extensions } = options;
  const session = new Session({ record, model, roster, cwd: directory, claude, extensions });
  session.open();
  await session.resume();
  return new Life(session);
}
