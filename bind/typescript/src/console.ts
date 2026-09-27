import { answered, type Engine, shapes } from "../index.cjs";
import { type Ear, fault, over, speaking } from "./ears.js";
import { type Fact, isQuestion, type OperatorPrompt } from "./types.js";

/** A value the operator gives, in the shape its prompt wants: a number goes to a float prompt as a float, which
 * crosses marked, as its digits, since a whole number crosses as an int. */
function shaped(shape: string, value: unknown): unknown {
  return shape === "float" && typeof value === "number" ? { is: "float", args: [String(value)] } : value;
}

export interface ConsoleOptions {
  /** Answers each prompt in the place of a person, as a test or another host does. */
  operator?: (
    prompt: { id: string; shape: string; message: string },
    signal: AbortSignal,
  ) => Promise<unknown>;
  /** Told when the prompts that wait change. */
  changed?: () => void;
  /** Told what the life refused when the console closed a prompt. */
  fault?: (error: unknown) => void;
}

/** The console, as an ear: it takes each prompt put to the operator, shows it, and closes it with what the operator
 * answered. A prompt of a shape that the operator answers not is shown not, and closed with what no line comes to,
 * which is the refusal of its shape. */
export class Console {
  /** The prompts that wait for the operator, by the act. */
  readonly prompts = new Map<string, OperatorPrompt>();
  /** The engine whose prompts it takes, which its later work speaks into. */
  engine?: Engine;
  private readonly controller = new AbortController();

  constructor(private readonly options: ConsoleOptions = {}) {}

  *ear(): Ear {
    for (;;) {
      const fact = (yield null) as Fact | undefined;
      if (!fact) continue;
      const [kind, id, , ...words] = fact;
      if (kind === "prompt" && isQuestion(kind, id)) {
        yield ["started", id];
        queueMicrotask(() =>
          this.asked(id, String(words[1]), String(words[2])).catch((error) => this.options.fault?.(error)),
        );
      } else if (kind === "done") {
        const prompt = this.prompts.get(id);
        if (!prompt) continue;
        this.prompts.delete(id);
        prompt.reject(new Error("The prompt ended."));
      }
    }
  }

  /** A prompt put to the operator, closed later as the console with what the operator answered. */
  private async asked(id: string, shape: string, message: string): Promise<void> {
    const engine = this.engine;
    if (!engine || over(engine, id)) return;
    let value: unknown;
    try {
      if (this.options.operator)
        value = shaped(shape, await this.options.operator({ id, shape, message }, this.controller.signal));
      else if (!shapes().includes(shape)) value = answered(shape, "");
      else
        value = await new Promise((resolve, reject) => {
          this.prompts.set(id, { id, shape, message, resolve, reject });
          this.options.changed?.();
        });
    } catch (error) {
      if (over(engine, id)) return;
      value = fault(error);
    }
    if (!over(engine, id)) speaking(engine, "console", () => engine.close(value, { id }));
  }

  /** What the operator typed, as the answer of the prompt it names, read in the shape the prompt wants by the rules
   * of the crate. */
  answer(id: string, input: string): void {
    const prompt = this.prompts.get(id);
    if (!prompt) throw new Error("This prompt is no longer open.");
    const value = answered(prompt.shape, input);
    this.prompts.delete(id);
    prompt.resolve(value);
    this.options.changed?.();
  }

  /** Every prompt that waits is refused, and no answer comes any more. */
  dispose(): void {
    this.controller.abort();
    for (const prompt of this.prompts.values()) prompt.reject(new Error("The console was disposed."));
    this.prompts.clear();
  }
}
