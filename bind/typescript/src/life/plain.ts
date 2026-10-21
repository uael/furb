import { failed, type LiveAct } from "./acts.js";

/** The names of exceptions that a message of furb can start with. */
const exception = /^(?:[A-Z]\w*(?:Error|Exception)|Refused|Drift|KeyboardInterrupt)$/;

/** A message as a person reads it: with no name of its exception around it, as Refused('…') or Refused: …, with each
 * path in the directory of the life read from that directory, and with no period at its end. */
export function plain(text: string, directory = ""): string {
  let out = text.trim();
  const called = /^(\w+)\((['"])([\s\S]*)\2\)$/.exec(out);
  const named = /^(\w+): ([\s\S]*)$/.exec(out);
  if (called?.[1] && exception.test(called[1]))
    out = (called[3] ?? "").replace(/\\(['"\\])/g, "$1").replace(/\\n/g, "\n");
  else if (named?.[1] && exception.test(named[1])) out = named[2] ?? "";
  else if (/^\w+\(\)$/.test(out) && exception.test(out.slice(0, -2))) out = out.slice(0, -2);
  if (directory) out = out.split(`${directory.replace(/\/+$/, "")}/`).join("");
  return out.replace(/\.$/, "");
}

/** What a failed act says, as a person reads it: the reason of a rung that failed, or the exception that its value is,
 * and nothing for an act that did not fail. */
export function reason(act: LiveAct, directory = ""): string | undefined {
  if (!failed(act)) return undefined;
  if (act.run) return plain(act.run.reason, directory);
  const value = act.value as { is?: string; args?: unknown[] };
  const first = value.args?.[0];
  return typeof first === "string" && first ? plain(first, directory) : (value.is ?? "");
}
