import { answered, type Engine, shapes } from "../index.cjs";
import { type Ear, fault, over, speaking } from "./ears.js";
import { type Fact, isQuestion, type OperatorThread } from "./types.js";

/** A value the operator gives, in the shape its thread wants: a number goes to a float thread as a float, which
 * crosses marked, as its digits, since a whole number crosses as an int. */
function shaped(shape: string, value: unknown): unknown {
  return shape === "float" && typeof value === "number" ? { is: "float", args: [String(value)] } : value;
}

export interface ConsoleOptions {
  /** Answers each thread in the place of a person, as a test or another host does. */
  operator?: (
    thread: { id: string; shape: string; markdown: string },
    signal: AbortSignal,
  ) => Promise<unknown>;
  /** Told when the threads that wait change. */
  changed?: () => void;
  /** Told what the life refused when the console closed a thread. */
  fault?: (error: unknown) => void;
}

/** The console, as an ear: it takes each thread put to the operator, shows it, and closes it with what the operator
 * answered. A thread of a shape that the operator answers not is shown not, and closed with what no line comes to,
 * which is the refusal of its shape. */
export class Console {
  /** The threads that wait for the operator, by the act. */
  readonly threads = new Map<string, OperatorThread>();
  /** The engine whose threads it takes, which its later work speaks into. */
  engine?: Engine;
  private readonly controller = new AbortController();

  constructor(private readonly options: ConsoleOptions = {}) {}

  *ear(): Ear {
    for (;;) {
      const fact = (yield null) as Fact | undefined;
      if (!fact) continue;
      const [kind, id, , ...words] = fact;
      if (kind === "thread" && isQuestion(kind, id)) {
        yield ["started", id];
        queueMicrotask(() =>
          this.asked(id, String(words[1]), String(words[2])).catch((error) => this.options.fault?.(error)),
        );
      } else if (kind === "done") {
        const thread = this.threads.get(id);
        if (!thread) continue;
        this.threads.delete(id);
        thread.reject(new Error("The thread ended."));
      }
    }
  }

  /** A thread put to the operator, closed later as the console with what the operator answered. */
  private async asked(id: string, shape: string, markdown: string): Promise<void> {
    const engine = this.engine;
    if (!engine || over(engine, id)) return;
    let value: unknown;
    try {
      if (this.options.operator)
        value = shaped(shape, await this.options.operator({ id, shape, markdown }, this.controller.signal));
      else if (!shapes().includes(shape)) value = answered(shape, "");
      else
        value = await new Promise((resolve, reject) => {
          this.threads.set(id, { id, shape, markdown, resolve, reject });
          this.options.changed?.();
        });
    } catch (error) {
      if (over(engine, id)) return;
      value = fault(error);
    }
    if (!over(engine, id)) speaking(engine, "console", () => engine.close(value, { id }));
  }

  /** What the operator typed, as the answer of the thread it names, read in the shape the thread wants by the rules
   * of the crate. */
  answer(id: string, input: string): void {
    const thread = this.threads.get(id);
    if (!thread) throw new Error("This thread is no longer open.");
    const value = answered(thread.shape, input);
    this.threads.delete(id);
    thread.resolve(value);
    this.options.changed?.();
  }

  /** Every thread that waits is refused, and no answer comes any more. */
  dispose(): void {
    this.controller.abort();
    for (const thread of this.threads.values()) thread.reject(new Error("The console was disposed."));
    this.threads.clear();
  }
}
