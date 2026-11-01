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
  /** The values of a list of a Literal, which the operator ticks: each as he reads it and as the list writes it. A
   * shape that is no such list has none. */
  options: { shown: string; json: string }[];
  /** What the line of the operator is, in words, when its shape is more than text, as a whole number or a list of
   * JSON: none for a text and for a choice. */
  hint: string;
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

/** A value of a Literal as the operator writes it: a string with no quotes, and any other value as Python writes
 * it. */
function shown(token: string): string {
  return quoted(token) ? token.slice(1, -1).replace(/\\(.)/g, "$1") : token;
}

/** A value of a Literal as JSON, which a line that holds a list of values writes. */
function jsonOf(token: string): string {
  if (quoted(token)) return JSON.stringify(shown(token));
  return ({ True: "true", False: "false", None: "null" } as Record<string, string>)[token] ?? token;
}

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

/** The lines that answer a shape that is a choice: yes and no for a bool, and each value of a Literal. A shape that
 * takes any line of its kind gives none. */
export function choicesOf(shape: string): string[] {
  if (shape === "bool") return ["yes", "no"];
  const form = formOf(shape);
  return form?.is === "literal" ? form.values.map(shown) : [];
}

/** Whether a line of no word is an answer of a shape: a union that holds None, which an empty line answers with None. */
export function takesNothing(shape: string): boolean {
  const form = formOf(shape);
  return form?.is === "either" && form.of.some((one) => one.is === "none");
}

/** The values that a list of a Literal takes, each as the operator reads it and as the line of the list writes it. A
 * shape that is no such list gives none. */
export function optionsOf(shape: string): { shown: string; json: string }[] {
  const form = formOf(shape);
  const of = form?.is === "list" ? form.of : undefined;
  return of?.is === "literal" ? of.values.map((one) => ({ shown: shown(one), json: jsonOf(one) })) : [];
}

/** What a form is, in words, as a part of a line of JSON. */
function said(form: Form): string {
  switch (form.is) {
    case "str":
      return "a text";
    case "int":
      return "a whole number";
    case "float":
      return "a number";
    case "bool":
      return "true or false";
    case "none":
      return "null";
    case "literal":
      return `one of ${form.values.map(jsonOf).join(", ")}`;
    case "list":
      return form.of ? `a list, each item ${said(form.of)}` : "a list";
    case "dict":
      return form.of ? `an object, each value ${said(form.of)}` : "an object";
    case "tuple":
      return `a list of ${form.of.length} ${form.of.length === 1 ? "item" : "items"}: ${form.of.map(said).join("; ")}`;
    case "either":
      return form.of.map(said).join(" or ");
  }
}

/** A line of JSON that a form takes, the nth of two examples of it. */
function sample(form: Form, n = 0): string {
  switch (form.is) {
    case "str":
      return n ? '"b"' : '"a"';
    case "int":
      return n ? "2" : "1";
    case "float":
      return n ? "2.5" : "1.5";
    case "bool":
      return n ? "false" : "true";
    case "none":
      return "null";
    case "literal":
      return jsonOf(form.values[n % form.values.length] ?? "");
    case "list":
      return form.of ? `[${sample(form.of, 0)}, ${sample(form.of, 1)}]` : '["a", 1]';
    case "dict":
      return `{"a": ${form.of ? sample(form.of, n) : "1"}}`;
    case "tuple":
      return `[${form.of.map((one) => sample(one, n)).join(", ")}]`;
    case "either":
      return sample(form.of.find((one) => one.is !== "none") ?? { is: "none" }, n);
  }
}

/** The items of a list or of an object, in words, for a form that a word names. */
const nouns: Record<string, string> = {
  str: "text",
  int: "whole numbers",
  float: "numbers",
  bool: "true or false",
};

/** What the operator writes for a shape whose line is more than text, in words: a whole number, a number, a list, an
 * object or a tuple as a line of JSON with an example of it, or a union of them, which takes no word for nothing. A
 * shape that takes any text, or a choice, gives none. */
export function lineHint(shape: string): string {
  const form = formOf(shape);
  if (!form) return "";
  const line = (one: Form): string => {
    switch (one.is) {
      case "int":
        return "a whole number";
      case "float":
        return "a number";
      case "list":
      case "dict": {
        const noun = one.of && nouns[one.of.is];
        const of = noun
          ? ` of ${noun}`
          : one.of
            ? `, each ${one.is === "list" ? "item" : "value"} ${said(one.of)}`
            : "";
        return `a JSON ${one.is === "list" ? "list" : "object"}${of}, as ${sample(one)}`;
      }
      case "tuple":
        return `a JSON ${said(one).slice(2)}, as ${sample(one)}`;
      case "either": {
        const arms = one.of.filter((arm) => arm.is !== "none");
        const words = arms.map(
          (arm) => line(arm) || (arm.is === "bool" ? "yes or no" : arm.is === "str" ? "a text" : said(arm)),
        );
        return `${words.join(" or ")}${arms.length < one.of.length ? ", or no word for nothing" : ""}`;
      }
      default:
        return "";
    }
  };
  return line(form);
}

/** What an ask wants of the operator, in words: nothing for a notice, the choices for a choice, any of the values of a
 * list of them, and else its shape. */
export function wants(
  ask: Pick<Ask, "notice" | "shape" | "choices"> & { options?: readonly { shown: string }[] },
): string {
  if (ask.notice) return "nothing";
  const { choices } = ask;
  if (choices.length > 1) return `${choices.slice(0, -1).join(", ")} or ${choices.at(-1)}`;
  const options = ask.options?.map((one) => one.shown) ?? [];
  if (options.length)
    return `any of ${options.length > 1 ? `${options.slice(0, -1).join(", ")} or ${options.at(-1)}` : options[0]}`;
  return choices[0] ?? ask.shape;
}

/** The desk of a life: each question that waits for the operator and each notice that he did not dismiss, the oldest
 * first. A question waits until its act is done, though the console can hold it a moment longer. */
export function desk({
  acts,
  ids,
  prompts,
  notices,
  label,
}: {
  acts: readonly LiveAct[];
  /** The acts by their name, which a host that holds them gives in place of a pass over every act. */
  ids?: ReadonlyMap<string, LiveAct>;
  prompts: Iterable<Held>;
  notices: Iterable<Held>;
  label(chain: string): string;
}): Desk {
  const byId = ids ?? new Map(acts.map((act) => [act.id, act]));
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
        options: notice ? [] : optionsOf(shape),
        hint: notice ? "" : lineHint(shape),
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
