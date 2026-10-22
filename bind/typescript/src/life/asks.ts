import type { LiveAct } from "./acts.js";
import { readable } from "./loom.js";

/** What the console holds of a prompt to the operator: a question that waits for an answer, or a notice that the
 * operator did not dismiss yet. */
export interface Held {
  id: string;
  message: string;
  shape?: string;
}

/** A prompt to the operator that waits for him. A question waits for an answer of its shape. A notice is a prompt of
 * None, which the console closed as it took it, and which waits only until the operator dismisses it. */
export interface Ask {
  id: string;
  /** The chain that the prompt stands on, and the label of that chain. */
  chain: string;
  label: string;
  /** Who made the prompt: a rung, an ear, or the operator. */
  by: string;
  shape: string;
  message: string;
  notice: boolean;
  /** The lines that answer a choice, one for each value that the shape allows. A shape that takes any line of its
   * kind has none. */
  choices: string[];
  /** The place of the prompt in the order of the record. */
  began: number;
}

/** What waits for the operator in a life: each ask, the oldest first, and how many of them are questions and
 * notices. */
export interface Desk {
  asks: Ask[];
  questions: number;
  notices: number;
}

/** The values of a Literal as the operator writes them: a string with no quotes, and any other value as Python
 * writes it. */
function literals(args: string): string[] {
  const out: string[] = [];
  let part = "";
  let quote = "";
  let depth = 0;
  for (let index = 0; index < args.length; index++) {
    const char = args[index] ?? "";
    if (quote) {
      if (char === "\\") {
        part += char + (args[index + 1] ?? "");
        index++;
        continue;
      }
      if (char === quote) quote = "";
    } else if (char === "'" || char === '"') quote = char;
    else if ("([{".includes(char)) depth++;
    else if (")]}".includes(char)) depth--;
    else if (char === "," && depth === 0) {
      out.push(part);
      part = "";
      continue;
    }
    part += char;
  }
  out.push(part);
  return out
    .map((value) => value.trim())
    .filter(Boolean)
    .map((value) => (/^(['"]).*\1$/s.test(value) ? value.slice(1, -1).replace(/\\(.)/g, "$1") : value));
}

/** The lines that answer a shape that is a choice: yes and no for a bool, and each value of a Literal. A shape that
 * takes any line of its kind gives none. */
export function choicesOf(shape: string): string[] {
  if (shape === "bool") return ["yes", "no"];
  const literal = /^(?:typing\.)?Literal\[(.*)\]$/s.exec(shape.trim());
  return literal ? literals(literal[1] ?? "") : [];
}

/** What an ask wants of the operator, in words: nothing for a notice, the choices for a choice, and else its
 * shape. */
export function wants(ask: Pick<Ask, "notice" | "shape" | "choices">): string {
  if (ask.notice) return "nothing";
  const { choices } = ask;
  if (choices.length > 1) return `${choices.slice(0, -1).join(", ")} or ${choices.at(-1)}`;
  return choices[0] ?? ask.shape;
}

/** The desk of a life: each question that waits for the operator and each notice that he did not dismiss, the oldest
 * first. A question waits until its act is done, though the console can hold it a moment longer. */
export function desk({
  acts,
  prompts,
  notices,
  label,
}: {
  acts: readonly LiveAct[];
  prompts: Iterable<Held>;
  notices: Iterable<Held>;
  label(chain: string): string;
}): Desk {
  const byId = new Map(acts.map((act) => [act.id, act]));
  const made = (held: Held, notice: boolean): Ask[] => {
    const act = byId.get(held.id);
    if (!act || (!notice && act.done)) return [];
    const shape = notice ? "None" : (held.shape ?? String(act.words[0] ?? ""));
    return [
      {
        id: held.id,
        chain: act.on,
        label: label(act.on),
        by: act.by,
        shape,
        message: held.message,
        notice,
        choices: notice ? [] : choicesOf(shape),
        began: act.began,
      },
    ];
  };
  const asks = [
    ...[...prompts].flatMap((held) => made(held, false)),
    ...[...notices].flatMap((held) => made(held, true)),
  ].sort((one, other) => one.began - other.began);
  const questions = asks.filter((ask) => !ask.notice).length;
  return { asks, questions, notices: asks.length - questions };
}

/** What a message asks, on one line, as a person reads it: its first line that holds words, with the line after it
 * when the first only leads in to it with a colon, as a relay says who asks before the question. */
export function gist(message: string): string {
  const [first = "", second] = message
    .split("\n")
    .filter((line) => line.trim())
    .map((line) => readable(line));
  return first.endsWith(":") && second ? `${first} ${second}` : first;
}

/** The ask that the operator acts on, on the chain that he reads: the one that he points at, else the oldest question,
 * else the oldest notice. A chain where nothing waits gives none. */
export function aimed(asks: readonly Ask[], chain: string, pointed?: string): Ask | undefined {
  const here = asks.filter((ask) => ask.chain === chain);
  return here.find((ask) => ask.id === pointed) ?? here.find((ask) => !ask.notice) ?? here[0];
}

/** The ask a step after or before one, round from the last to the first, or the first when the one is gone. */
export function beside(asks: readonly Ask[], id: string | undefined, step: number): Ask | undefined {
  const at = asks.findIndex((ask) => ask.id === id);
  if (at < 0) return step < 0 ? asks.at(-1) : asks[0];
  return asks[(((at + step) % asks.length) + asks.length) % asks.length];
}
