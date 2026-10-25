import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { type StructuredPatchHunk as Hunk, structuredPatch } from "diff";
import type { FileChange } from "furb";

export type { Hunk };

/** A branch source that is unavailable, rather than a failed read of the journal or files. */
export class ReviewUnavailable extends Error {
  override readonly name = "ReviewUnavailable";
}

/** The scopes of the review of the changes: the writes since the last message of the operator on the chain shown,
 * the writes of that chain, the writes of the whole life, and the files of the branch against main, which git
 * holds. */
export const scopes = ["turn", "chain", "life", "branch"] as const;
export type Scope = (typeof scopes)[number];
export const scopeLabels: Record<Scope, string> = {
  turn: "This turn",
  chain: "This chain",
  life: "The life",
  branch: "The branch",
};

/** The net change of each file across a list of changes, in the order that each file first changed: the text before
 * its first change, and the text after its last. A file that ends as it began is no change. */
export function net(changes: Iterable<FileChange>): FileChange[] {
  const files = new Map<string, FileChange>();
  for (const change of changes) {
    const held = files.get(change.path);
    files.set(change.path, held ? { ...held, after: change.after, act: change.act } : { ...change });
  }
  return [...files.values()].filter((change) => change.before !== change.after);
}

/** A change of a file as the review shows it: its hunks, and the count of lines that they add and remove. */
export type Reviewed = FileChange & { deleted?: boolean; hunks: Hunk[]; added: number; removed: number };

/** A change of a file with its hunks, each with three lines of context. */
export function reviewed(change: FileChange): Reviewed {
  const { hunks } = structuredPatch(change.path, change.path, change.before, change.after, "", "", {
    context: 3,
  });
  const count = (sign: string) =>
    hunks.reduce((sum, hunk) => sum + hunk.lines.filter((line) => line[0] === sign).length, 0);
  return { ...change, hunks, added: count("+"), removed: count("-") };
}

/** The patch of one hunk alone, as the view of a diff reads it. */
export function hunkPatch(path: string, hunk: Hunk): string {
  const head = `@@ -${hunk.oldStart},${hunk.oldLines} +${hunk.newStart},${hunk.newLines} @@`;
  return `${[`--- ${path}`, `+++ ${path}`, head, ...hunk.lines].join("\n")}\n`;
}

/** The lines of one side of a hunk: its context, with the lines that it removed for the old side, or that it added
 * for the new side. */
export function side(hunk: Hunk, old: boolean): string[] {
  return hunk.lines
    .filter((line) => line[0] === " " || line[0] === (old ? "-" : "+"))
    .map((line) => line.slice(1));
}

/** The word that reverts a hunk through the World. It checks the reviewed text before it changes the file. */
export function revertWord(
  path: string,
  hunk: Hunk,
  change?: Pick<FileChange, "before" | "after" | "made"> & { deleted?: boolean },
): string {
  const lo = hunk.newLines ? hunk.newStart : hunk.newStart + 1;
  const hi = hunk.newStart + hunk.newLines - 1 + (hunk.newLines ? 0 : 1);
  const old = side(hunk, true);
  const text = old.length ? `${old.join("\n")}\n` : "";
  const p = JSON.stringify(path);
  const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;
  if (change?.made) {
    const command = JSON.stringify(`rm -- ${quote(path)}`);
    return [
      "try:",
      `  held = read(${p}, HIDDEN)`,
      "except Refused:",
      "  held = None",
      `if held is None or held.content != ${JSON.stringify(change.after)}:`,
      `  raise Refused(${JSON.stringify(`${path} changed since the review`)})`,
      `write(held.replace(held.content, ""))`,
      `await bash(${command})`,
    ].join("\n");
  }
  if (change?.deleted) {
    return [
      `probe = await bash(${JSON.stringify(`test -e ${quote(path)}`)})`,
      "if probe.code != 1:",
      `  raise Refused(${JSON.stringify(`${path} changed since the review`)})`,
      `write(Text(${p}, ${JSON.stringify(change.before)}))`,
    ].join("\n");
  }
  return [
    `held = read(${p}, HIDDEN)`,
    `if held.lines[${lo - 1}:${hi}] != ${JSON.stringify(side(hunk, false))}:`,
    `  raise Refused(${JSON.stringify(`${path} changed since the review: lines ${lo} to ${hi} are not the lines of the hunk`)})`,
    `write(held.edit(${lo}, ${hi}, ${JSON.stringify(text)}))`,
  ].join("\n");
}

/** One line of a hunk as a comment names it: its number in the file on its side, and its text. A removed line has
 * the number of the old file, and every other line the number of the new file. */
export interface Line {
  number: number;
  sign: "+" | "-" | " ";
  text: string;
}
export function lines(hunk: Hunk): Line[] {
  let old = hunk.oldStart;
  let fresh = hunk.newStart;
  const out: Line[] = [];
  for (const line of hunk.lines) {
    const sign = line[0];
    if (sign === "-") out.push({ number: old++, sign, text: line.slice(1) });
    else if (sign === "+") out.push({ number: fresh++, sign, text: line.slice(1) });
    else if (sign === " ") {
      out.push({ number: fresh++, sign, text: line.slice(1) });
      old++;
    }
  }
  return out;
}

/** The start of a message that comments one line of a change, which the operator ends and sends to the chain. */
export function commentStart(path: string, line: Line): string {
  const where = line.sign === "-" ? `the line ${line.number} that the change removed` : `line ${line.number}`;
  return `On ${path}, ${where}:\n> ${line.text}\n\n`;
}

/** Run git in a directory, and give what it printed, or nothing when it failed. */
function git(directory: string, ...args: string[]): string | undefined {
  const run = spawnSync("git", ["-C", directory, ...args], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  return run.status === 0 ? run.stdout : undefined;
}

/** Files that differ from the selected branch base and the working tree. The default base is origin/HEAD, then main,
 * then master. A caller can name another base. Binary files are left out. */
export function branchChanges(directory: string, requestedBase?: string): FileChange[] {
  const top = git(directory, "rev-parse", "--show-toplevel")?.trim();
  if (!top)
    throw new ReviewUnavailable(`${directory} is not in a git repository, so it has no branch to review.`);
  const base = (requestedBase ? [requestedBase] : ["origin/HEAD", "main", "master"])
    .map((name) => git(top, "merge-base", "HEAD", name)?.trim())
    .find((sha) => sha);
  if (!base)
    throw new ReviewUnavailable(
      `No common base was found against ${requestedBase || "origin/HEAD, main or master"}. Choose another branch base.`,
    );
  const required = (...args: string[]): string => {
    const text = git(top, ...args);
    if (text === undefined) throw new Error(`Cannot read branch changes: git ${args[0]} failed.`);
    return text;
  };
  const listed = required("diff", "--name-status", "--no-renames", "-z", base).split("\0");
  const files: [string, string][] = [];
  for (let at = 0; at + 1 < listed.length; at += 2) files.push([listed[at] ?? "", listed[at + 1] ?? ""]);
  for (const path of required("ls-files", "--others", "--exclude-standard", "-z").split("\0"))
    if (path) files.push(["A", path]);
  const changes: FileChange[] = [];
  for (const [status, path] of files) {
    const before = status === "A" ? "" : required("show", `${base}:${path}`);
    const after = status === "D" ? "" : readFileSync(join(top, path), "utf8");
    if (before.includes("\0") || after.includes("\0")) continue;
    changes.push({
      path: join(top, path),
      before,
      after,
      ...(status === "A" ? { made: true } : {}),
      ...(status === "D" ? { deleted: true } : {}),
    });
  }
  return changes;
}
