import type { Engine } from "../index.cjs";
import { decodeRecord } from "../index.cjs";
import { type Ear, fault, speaking } from "./ears.js";
import { type Fact, isQuestion, marked, type OperatorPrompt, shapes } from "./types.js";

/** A value the operator gives, in the shape its prompt wants: a whole number crosses as an int, so a float prompt
 * takes it marked as a float. */
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
 * answered, or with a refusal of a shape the operator cannot answer. */
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
    if (!engine || engine.disposed || engine.outcome(id).done) return;
    let value: unknown;
    try {
      if (this.options.operator)
        value = shaped(shape, await this.options.operator({ id, shape, message }, this.controller.signal));
      else if (!shapes.some((name) => name === shape))
        throw new Error(`The operator cannot answer ${shape}.`);
      else
        value = await new Promise((resolve, reject) => {
          this.prompts.set(id, { id, shape, message, resolve, reject });
          this.options.changed?.();
        });
    } catch (error) {
      if (engine.disposed || engine.outcome(id).done) return;
      value = fault(error);
    }
    if (!engine.disposed && !engine.outcome(id).done)
      speaking(engine, "console", () => engine.close(value, { id }));
  }

  /** What the operator typed, as the answer of the prompt it names, read in the shape the prompt wants. */
  answer(id: string, input: string): void {
    const prompt = this.prompts.get(id);
    if (!prompt) throw new Error("This prompt is no longer open.");
    let value: unknown = input;
    if (prompt.shape === "None") value = null;
    else if (prompt.shape === "bool") {
      if (!/^(yes|no|true|false|y|n|0|1)$/i.test(input)) throw new Error("Enter yes or no.");
      value = /^(yes|true|y|1)$/i.test(input);
    } else if (prompt.shape === "int" || prompt.shape === "float") {
      value = Number(input);
      if (
        !input.trim() ||
        !Number.isFinite(value) ||
        (prompt.shape === "int" && !Number.isSafeInteger(value))
      )
        throw new Error(`Enter a valid ${prompt.shape}.`);
      value = shaped(prompt.shape, value);
    } else if (prompt.shape === "list" || prompt.shape === "dict") {
      let plain: unknown;
      try {
        plain = JSON.parse(input);
      } catch {
        throw new Error(`Enter a JSON ${prompt.shape}.`);
      }
      if (
        prompt.shape === "list"
          ? !Array.isArray(plain)
          : !plain || typeof plain !== "object" || Array.isArray(plain)
      )
        throw new Error(`Enter a JSON ${prompt.shape}.`);
      // The native reader keeps what JSON.parse loses: it refuses an unsafe integer, and 2.0 stays a float. A map
      // that holds the key is crosses as its pairs.
      value = marked(plain, decodeRecord(input));
    }
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
