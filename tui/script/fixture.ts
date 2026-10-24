import { readFileSync, writeFileSync } from "node:fs";
import { isQuestion } from "furb";

/** The kinds of question that are steps of an act, which a view of a life shows not, as the activity says. The run, the
 * reply and the stand stay, since a run carries the word of its rung, a reply the usage of an answer, and a stand the
 * roster and the actor of the life. */
const steps = new Set(["read", "write", "clock", "chance", "gate", "cd", "merged", "wants"]);
/** The kinds of act that an ear makes under an act of its own, which no view of a life shows, as the questions of the
 * memory that its watcher asks. */
const unseen = new Set(["memory"]);
/** The lines of a word that the fixture keeps: all of them. */
const WHOLE = 100_000;
/** The lines that a command prints, which the fixture keeps for each command that still runs at its moment. */
const PRINTED = 5;

/** A text with no secret and no path of a home in it: each key, token and password that it names is replaced, an
 * address of mail is replaced, and a home directory reads as ~. */
export function scrub(text: string): string {
  return text
    .replace(/(?:\/Users|\/home)\/[^/\s'"`]+/g, "~")
    .replace(/[\w.+-]+@[\w-]+(?:\.[\w-]+)+/g, "someone@example.com")
    .replace(
      /\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_\w{20,}|sk-[\w-]{20,}|AKIA[0-9A-Z]{16}|xox[abprs]-[\w-]{10,}|eyJ[\w-]{10,}\.[\w-]{10,}\.[\w-]+)\b/g,
      "[key]",
    )
    .replace(/\b([A-Z][A-Z0-9_]*(?:KEY|TOKEN|SECRET|PASSWORD)[A-Z0-9_]*)=\S+/g, "$1=[key]")
    .replace(/(authorization:\s*(?:bearer|token)\s+)\S+/gi, "$1[key]");
}

/** A text cut to a count of lines and of characters on each, with a mark where it is cut. */
export function clip(text: string, lines = 40, width = 160): string {
  const all = scrub(text).split("\n");
  const kept = all
    .slice(0, lines)
    .map((line) => (line.length > width ? `${line.slice(0, width - 1)}…` : line));
  if (all.length > lines) kept.push(`… ${all.length - lines} more lines`);
  return kept.join("\n");
}

/** A word with each string that makes its line longer than most cut to its start and a mark inside its quotes, so the
 * word stays python and the fixture stays small. A line of a quote is cut as it is, since a quote is no python. */
export function shortened(word: string, most = 1000, keep = 300): string {
  let open = "";
  let quote = "";
  return word
    .split("\n")
    .map((line) => {
      if (quote || /^<s:\w+>/.test(line)) {
        quote ||= line.match(/^<s:(\w+)>/)?.[1] ?? "";
        if (line.endsWith(`</s:${quote}>`)) quote = "";
        return line.length > most ? `${line.slice(0, keep)}…` : line;
      }
      if (line.length <= most) {
        open = runsOn(line, open);
        return line;
      }
      let out = "";
      let kept = 0;
      let cut = false;
      for (let at = 0; at < line.length; at++) {
        const char = line[at] ?? "";
        if (open) {
          if (line.startsWith(open, at)) {
            out += open;
            at += open.length - 1;
            open = "";
            continue;
          }
          const step = char === "\\" ? line.slice(at, at + 2) : char;
          if (kept < keep) out += step;
          else if (!cut) out += "…";
          cut ||= kept >= keep;
          kept += step.length;
          at += step.length - 1;
          continue;
        }
        if (char === "#") return out + line.slice(at);
        out += char;
        if (char !== '"' && char !== "'") continue;
        open = line.startsWith(char.repeat(3), at) ? char.repeat(3) : char;
        if (open.length === 3) {
          out += char.repeat(2);
          at += 2;
        }
        kept = 0;
        cut = false;
      }
      if (open.length === 1) open = "";
      return out;
    })
    .join("\n");
}

/** The mark of the string of three marks that runs on past a line, after the line, from the one that ran on to it. */
function runsOn(line: string, from: string): string {
  let open = from;
  for (let at = 0; at < line.length; at++) {
    const char = line[at] ?? "";
    if (open) {
      if (char === "\\") at++;
      else if (line.startsWith(open, at)) {
        at += open.length - 1;
        open = "";
      }
      continue;
    }
    if (char === "#") break;
    if (char === '"' || char === "'") {
      open = line.startsWith(char.repeat(3), at) ? char.repeat(3) : char;
      if (open.length === 3) at += 2;
    }
  }
  return open.length === 3 ? open : "";
}

type Fact = unknown[];
/** The last lines of a print, each cut to a width. */
const printed = (text: string) =>
  scrub(text)
    .split("\n")
    .slice(-PRINTED)
    .map((line) => line.slice(0, 100))
    .join("\n");
const text = (value: unknown, lines?: number, width?: number) =>
  typeof value === "string" ? clip(value, lines, width) : value;

/** A value of a done as the fixture keeps it: a command keeps its code and the last lines of each stream, a text its
 * first lines, the answer of a model only its usage, and any other value its first lines as text. */
function value(kind: string, got: unknown): unknown {
  if (got === null || typeof got !== "object") return text(got, 12);
  const object = got as { is?: string; args?: unknown[]; content?: string; path?: string; code?: unknown };
  // A stand keeps the roster and the actor, and the directory with no home in it.
  if (Array.isArray(got))
    return kind === "reply"
      ? [got[0], "", got[2], null]
      : kind === "stand"
        ? [got[0], scrub(String(got[1])), got[2]]
        : [];
  if (object.is === "Exit") {
    const stream = (one: unknown) => {
      const content = String((one as { content?: string })?.content ?? "");
      return { content: clip(content.split("\n").slice(-2).join("\n"), 2, 100) };
    };
    return {
      ...object,
      stdout: stream((got as { stdout: unknown }).stdout),
      stderr: stream((got as { stderr: unknown }).stderr),
    };
  }
  if (object.is === "Text")
    return { is: "Text", path: scrub(String(object.path)), content: clip(String(object.content ?? ""), 3) };
  if (object.is === "tuple" && kind === "reply") {
    const [turn] = object.args as [unknown[]];
    return Array.isArray(turn) ? [turn[0], "", turn[2], null] : null;
  }
  if (object.args) return { is: object.is, args: object.args.map((one) => text(String(one), 3)) };
  return JSON.parse(clip(JSON.stringify(got), 1, 2000).replace(/…$/, "") || "null");
}

/** The facts of a log of a life that the model of a life reads, with no secret in them, up to the moment of the life
 * that holds the most work at once and the most questions to the operator. */
export function fixture(lines: string[]): { facts: Fact[]; header: Record<string, unknown> } {
  const facts: Fact[] = [];
  const kinds = new Map<string, string>();
  /** The rungs that an ear wrote, whose maker is no act. */
  const ears = new Set<string>();
  for (const line of lines) {
    if (!line.startsWith("[")) continue;
    const fact = JSON.parse(line) as Fact;
    const [kind, id] = fact as [string, string];
    if (isQuestion(kind, id)) {
      kinds.set(id, kind);
      if (kind === "rung" && !isQuestion(String(fact[2]).replace(/\d+$/, ""), String(fact[2]))) ears.add(id);
      if (steps.has(kind) || unseen.has(kind)) continue;
      const words = fact.slice(4);
      const kept =
        kind === "rung" || kind === "run"
          ? words.map((one, at) =>
              // The word of a rung that an ear wrote, as an extension, shows as one quiet line, and the word of any
              // other rung stays whole, since the feed shows it as the program that ran.
              at === 1 && kind === "run" && !ears.has(String(words[0]))
                ? text(typeof one === "string" ? shortened(one) : one, WHOLE, WHOLE)
                : text(one, 1, 100),
            )
          : kind === "prompt"
            ? [words[0], text(words[1], 5, 230), words[2]]
            : words.map((one) => (typeof one === "string" ? clip(one, 1, 90) : one));
      facts.push([...fact.slice(0, 4), ...kept]);
      continue;
    }
    const of = kinds.get(id);
    if (!of || steps.has(of) || unseen.has(of)) continue;
    if (kind === "done") facts.push([kind, id, fact[2], value(of, fact[3])]);
    // What a command prints streams under its line while it runs, so a print keeps its last lines and no mark of a cut.
    else if (kind === "out" && of === "bash")
      facts.push([kind, id, fact[2], printed(String(fact[3])), fact[4]]);
    // A start tells no view of a life more than the act tells, so the fixture keeps none.
    else if (["pause", "wake", "cancel"].includes(kind)) facts.push(fact.slice(0, 3));
    else if (kind === "close") facts.push([kind, id, fact[2], text(fact[3], 4, 230)]);
    else if (
      kind === "tell" &&
      of === "rung" &&
      String((fact[3] as string[])?.[0] ?? "").startsWith(`#${id} refused`)
    )
      facts.push([kind, id, fact[2], [clip(String((fact[3] as string[])[0]), 12)]]);
  }
  // The moment to show is the one with the most chains at work, with a question to the operator as a chain at work.
  const open = new Map<string, string>();
  const asks = new Set<string>();
  let best = facts.length,
    most = -1;
  for (const [at, fact] of facts.entries()) {
    const [kind, id] = fact as [string, string];
    if (isQuestion(kind, id) && ["rung", "bash", "wait"].includes(kind)) open.set(id, String(fact[3]));
    if (isQuestion(kind, id) && kind === "prompt" && fact[6] === "operator" && fact[4] !== "None")
      asks.add(id);
    if (kind === "done" || kind === "close" || kind === "cancel") {
      open.delete(id);
      asks.delete(id);
    }
    const score = new Set(open.values()).size + asks.size;
    if (score >= most) {
      most = score;
      best = at + 1;
    }
  }
  // A command that ended before the moment keeps nothing of what it printed, since its exit holds the end of it, and a
  // command that still runs keeps the last lines that it printed.
  const kept = facts.slice(0, best);
  const ended = new Set(kept.filter((fact) => fact[0] === "done").map((fact) => fact[1]));
  const shown = kept.filter((fact, at) => {
    if (fact[0] !== "out") return true;
    if (ended.has(fact[1])) return false;
    const after = kept.slice(at + 1).filter((one) => one[0] === "out" && one[1] === fact[1]).length;
    return after < PRINTED;
  });
  return { facts: shown, header: { recorded: true, cut: best, of: facts.length } };
}

if (import.meta.main) {
  const [log, out = "tui/fixtures/conductor.jsonl"] = process.argv.slice(2);
  if (!log) throw new Error("Give the log of a life, as .furb/sessions/<record>.jsonl.log.");
  const { facts, header } = fixture(readFileSync(log, "utf8").split("\n"));
  // A last pass scrubs each fact whole, so that no value that the passes above keep as it is holds a secret.
  writeFileSync(
    out,
    `${[JSON.stringify(header), ...facts.map((fact) => scrub(JSON.stringify(fact)))].join("\n")}\n`,
  );
  console.log(`${out}: ${facts.length} facts, cut at ${header.cut} of ${header.of}`);
}
