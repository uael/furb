import type { Ask as Asked, Desk as Owed } from "../protocol.js";
import { readable } from "./loom.js";

/** A prompt to the operator that waits for him, as the crate folds it, with the label of its chain. A question waits
 * for an answer of its shape. A notice is a prompt of None, which the console closed as it took it. */
export type Ask = Asked & { label: string };

/** What waits for the operator in a life: each ask, the oldest first, and how many of them are questions and
 * notices. */
export type Desk = Omit<Owed, "asks"> & { asks: Ask[] };

/** The values of a Literal as python writes them, each as its text. */
function tokens(args: string): string[] {
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
  return out.map((value) => value.trim()).filter(Boolean);
}

const quoted = (token: string) => /^(['"]).*\1$/s.test(token);

/** A shape that the operator answers, as the crate reads its name: the same names, so that what the TUI offers is what
 * the console takes. */
type Form =
  | { is: "str" | "int" | "float" | "bool" | "none" }
  | { is: "list" | "dict"; of?: Form }
  | { is: "tuple"; of: Form[] }
  | { is: "literal"; values: string[] }
  | { is: "either"; of: Form[] };

/** The parts of a text between the marks that stand in no brackets and in no quote. */
function parts(text: string, mark: string): string[] {
  const found: string[] = [];
  let depth = 0;
  let quote = "";
  let start = 0;
  for (let at = 0; at < text.length; at++) {
    const char = text[at] ?? "";
    if (quote) {
      if (char === "\\") at++;
      else if (char === quote) quote = "";
    } else if (char === "'" || char === '"') quote = char;
    else if ("([{".includes(char)) depth++;
    else if (")]}".includes(char)) depth--;
    else if (char === mark && depth === 0) {
      found.push(text.slice(start, at));
      start = at + 1;
    }
  }
  found.push(text.slice(start));
  return found;
}

const plain: Record<string, Form> = {
  str: { is: "str" },
  int: { is: "int" },
  float: { is: "float" },
  bool: { is: "bool" },
  None: { is: "none" },
};

/** A shape as its name reads, or none when the operator answers it not. */
function formOf(name: string): Form | undefined {
  const text = name.trim().replace(/^typing\./, "");
  const arms = parts(text, "|");
  if (arms.length > 1) {
    const forms = arms.map(formOf);
    if (forms.some((form) => !form)) return undefined;
    return {
      is: "either",
      of: (forms as Form[]).flatMap((form) => (form.is === "either" ? form.of : [form])),
    };
  }
  const open = text.indexOf("[");
  const head = (open < 0 ? text : text.slice(0, open)).trim();
  if (open < 0) return head === "list" || head === "dict" ? { is: head } : plain[head];
  if (!text.endsWith("]")) return undefined;
  const inside = text.slice(open + 1, -1);
  if (head === "list") {
    const of = formOf(inside);
    return of && { is: "list", of };
  }
  if (head === "dict") {
    const [key = "", value = "", more] = parts(inside, ",");
    const of = formOf(value);
    return formOf(key)?.is === "str" && of && more === undefined ? { is: "dict", of } : undefined;
  }
  if (head === "tuple") {
    const of = parts(inside, ",").map(formOf);
    return of.every(Boolean) ? { is: "tuple", of: of as Form[] } : undefined;
  }
  if (head !== "Literal") return undefined;
  const values = tokens(inside);
  const valid = values.every(
    (one) => quoted(one) || /^-?\d+$/.test(one) || ["True", "False", "None"].includes(one),
  );
  return values.length && valid ? { is: "literal", values } : undefined;
}

/** Whether a line of no word is an answer of a shape: a union that holds None, which an empty line answers with None. */
export function takesNothing(shape: string): boolean {
  const form = formOf(shape);
  return form?.is === "either" && form.of.some((one) => one.is === "none");
}

/** What an ask wants of the operator, in words: nothing for a notice, the choices for a choice, any of the values of a
 * list of them, and else its shape. */
export function wants(ask: Pick<Ask, "notice" | "shape" | "choices" | "ticks">): string {
  if (ask.notice) return "nothing";
  const { choices } = ask;
  if (choices.length > 1) return `${choices.slice(0, -1).join(", ")} or ${choices.at(-1)}`;
  const ticks = ask.ticks.map((one) => one.shown);
  if (ticks.length)
    return `any of ${ticks.length > 1 ? `${ticks.slice(0, -1).join(", ")} or ${ticks.at(-1)}` : ticks[0]}`;
  return choices[0] ?? ask.shape;
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
