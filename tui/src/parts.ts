/** The parts of the extensions for the TUI. A part is a module of TypeScript beside the word of an extension, which
 * the manifest names under `tui`. The TUI loads the part of each extension that the configs turn on, at its start,
 * and again when a file of its folder changes or when a write of an extension or of the config of the project lands,
 * with no restart and no build of furb. The part of an official extension is the text that the crate carries, which
 * changes only with a build of furb. The module exports a function as its default, which the TUI calls with a
 * `Furb`: through it, the part adds panels, the rows of the acts of a kind, marks in the feed of a chain, keys, and
 * commands of the palette. A part imports no value of furb: it takes all that it needs from what the TUI gives it, so
 * it loads in any project. A module that fails to load, and a component that throws, show the error in plain words in
 * the panel of the part, and the TUI goes on. */
import { type FSWatcher, mkdirSync, statSync, watch, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { parts as configured, type LiveAct, type Part } from "furb";
import type { Compaction } from "furb/life";
import * as model from "furb/life";
import { commands } from "./commands.ts";

/** The tone of a run of text, which the theme colors. */
export type Tone = "text" | "muted" | "faint" | "accent" | "success" | "warning" | "danger";
/** A line of a component: a text, or runs of text, each with its tone. */
export type Line = string | readonly (readonly [text: string, tone?: Tone])[];
/** The row of an act of a kind in the feed: what its label says after the kind, and the lines that its card shows
 * when it opens. */
export interface Row {
  label?: string;
  details?: readonly Line[];
}
/** A place of the feed of a chain that a part marks: the chain, the place in time of the mark, as the place of an act
 * that `began` gives, its title, and the lines under it. */
export interface Mark {
  chain: string;
  at: number;
  title: string;
  lines?: readonly Line[];
}
/** What a component reads of the life, and what it can do in it. */
export interface Seen {
  /** The chain on screen. */
  chain: string;
  /** Every act of the life, in the order of the record. */
  acts: readonly LiveAct[];
  /** Each compaction of a chain of the life, in the order of the record, as `furb/life` gives it. */
  compactions: readonly Compaction[];
  /** The acts of a kind, in the order of the record. */
  of(kind: string): LiveAct[];
  /** Runs python on the chain on screen, through the gate, as `/run` does. */
  run(word: string): Promise<void>;
  /** Shows a notice in the footer. */
  say(text: string): void;
}
/** What the TUI gives the function that a part exports. */
export interface Furb {
  /** The model of a life, `furb/life`. */
  life: typeof model;
  /** A panel over the input: a name, and the lines that it shows, which the TUI draws again with the life. */
  panel(name: string, draw: (seen: Seen) => readonly Line[]): void;
  /** The row of each act of a kind in the feed. The rows of a rung and of a prompt are the TUI's own. */
  row(kind: string, draw: (act: LiveAct, seen: Seen) => Row): void;
  /** Marks in the feed: `draw(seen)` gives each mark, and the feed of its chain shows it as a rule with its title,
   * then its lines, after the last act that began before its place, or at its start. A moment of the past shows only
   * the marks before that moment. The TUI draws the marks again with the life. */
  mark(draw: (seen: Seen) => readonly Mark[]): void;
  /** A key, as `ctrl+t`, which acts while no dialog is open. A key with no ctrl, meta or alt acts on an empty input
   * alone, and a key of the TUI wins. */
  key(on: string, run: (seen: Seen) => unknown): void;
  /** A command of the palette and of the input, as `/name`, with what it does. A command of the TUI wins. */
  command(name: string, detail: string, run: (argument: string, seen: Seen) => unknown): void;
}
/** A part as the TUI loaded it: its extension, its file, its components, and the error that stopped its load. */
export interface Loaded {
  name: string;
  file: string;
  /** What the disk held of the file when the part loaded, which `Parts` compares with the disk, since a watcher can
   * miss a change. The part of an extension that the crate carries has none. */
  stamp?: string;
  error?: string;
  panels: { name: string; draw: (seen: Seen) => readonly Line[] }[];
  rows: Map<string, (act: LiveAct, seen: Seen) => Row>;
  marks: ((seen: Seen) => readonly Mark[])[];
  keys: { on: string; run: (seen: Seen) => unknown }[];
  commands: { name: string; detail: string; run: (argument: string, seen: Seen) => unknown }[];
}

/** The text of an error in plain words, as its message says it, with no stack. */
export function plainError(error: unknown): string {
  // Bun gives a module that does not build or resolve as a message that is no Error, with the place of the fault.
  const shape = error as { message?: unknown; position?: { line?: number; column?: number } | null } | null;
  const said = typeof shape?.message === "string" ? shape.message : String(error);
  const at = shape?.position?.line
    ? ` at line ${shape.position.line}, column ${shape.position.column ?? 0}`
    : "";
  const text = said.replace(/\?furb=\d+/g, "").trim();
  return text ? `${text}${at}` : "It failed with no message.";
}

/** What a part gives, in a sentence: its components, or the error that stopped its load. */
export function summary(part: Loaded): string {
  if (part.error) return part.error;
  const marks = part.marks.length;
  const found = [
    ...part.panels.map((one) => `panel ${one.name}`),
    ...[...part.rows.keys()].map((kind) => `rows of ${kind}`),
    ...(marks ? [`${marks} ${marks === 1 ? "mark" : "marks"} in the feed`] : []),
    ...part.keys.map((one) => `key ${one.on}`),
    ...part.commands.map((one) => `/${one.name}`),
  ];
  return found.length ? `Its part gives ${found.join(", ")}.` : "Its part gives nothing.";
}

/** A part with no component. */
const bare = (name: string, file: string, error?: string, stamp?: string): Loaded => ({
  name,
  file,
  ...(stamp === undefined ? {} : { stamp }),
  ...(error ? { error } : {}),
  panels: [],
  rows: new Map(),
  marks: [],
  keys: [],
  commands: [],
});

/** What the disk holds of a file now, which changes when the file does, so a change shows with no event. */
function stamped(file: string): string {
  try {
    const seen = statSync(file);
    return `${seen.mtimeMs}:${seen.size}:${seen.ino}`;
  } catch {
    return "";
  }
}

/** How many modules this process imported, which makes the address of each import new. */
let imported = 0;

/** The file that holds the text of a part that the crate carries, since Bun imports a module from a file: a file in a
 * folder of the user alone in the folder of the system for temporary files, named by the hash of the text. */
function carried(file: string, text: string): string {
  const folder = join(tmpdir(), `furb-parts-${process.getuid?.() ?? "user"}`);
  mkdirSync(folder, { recursive: true, mode: 0o700 });
  // The folder for temporary files can be shared, and a folder there that another user made can hold a module of his.
  const made = statSync(folder);
  if (process.getuid && (made.uid !== process.getuid() || made.mode & 0o022))
    throw new Error(`${folder} is not of this user alone.`);
  const at = join(folder, `${Bun.hash(text).toString(36)}-${basename(file)}`);
  writeFileSync(at, text);
  return at;
}

/** The part of an extension, from its file as it stands, or from its text when the crate carries it: the module is
 * imported again, and its function says its components. A module that fails to import, that exports no function as
 * its default, or whose function throws, gives a part with no component and the error. */
export async function loadPart(name: string, file: string, text?: string): Promise<Loaded> {
  const stamp = text === undefined ? stamped(file) : undefined;
  const part = bare(name, file, undefined, stamp);
  const furb: Furb = {
    life: model,
    panel: (title, draw) => void part.panels.push({ name: String(title), draw }),
    row: (kind, draw) => {
      if (kind === "rung" || kind === "prompt") throw new Error(`The row of a ${kind} is the TUI's own.`);
      part.rows.set(String(kind), draw);
    },
    mark: (draw) => void part.marks.push(draw),
    key: (on, run) => void part.keys.push({ on: String(on), run }),
    command: (title, detail, run) => {
      if (Object.hasOwn(commands, title)) throw new Error(`The command /${title} is the TUI's own.`);
      part.commands.push({ name: String(title), detail: String(detail), run });
    },
  };
  try {
    const from = text === undefined ? file : carried(file, text);
    statSync(from);
    // A new address imports the module again, since Bun keeps each module that it imported by its address.
    const module: { default?: unknown } = await import(`${from}?furb=${++imported}`);
    if (typeof module.default !== "function")
      throw new Error("Its module exports no function as its default.");
    await module.default(furb);
  } catch (error) {
    return bare(name, file, `${basename(file)} of ${name}: ${plainError(error)}`, stamp);
  }
  return part;
}

/** The parts of a life in the directory of a project, which follow their files. */
export class Parts {
  loaded: Loaded[] = [];
  /** How many loads ended, which a view reads to know that the components changed. */
  version = 0;
  private asked = 0;
  private closed = false;
  private timer?: ReturnType<typeof setTimeout>;
  private readonly watchers = new Map<string, FSWatcher>();
  private readonly heardWrites = new Set<string>();
  private busy = 0;
  private readonly poll = setInterval(() => this.verify(), 500).unref();

  constructor(
    readonly project: string,
    private readonly changed: () => void,
    private readonly list: (project: string) => Part[] = configured,
  ) {}

  /** Loads each part again, as the configs give them now, each from its file as it stands, and calls back. A later
   * load wins over one that ends after it. */
  async load(): Promise<void> {
    this.busy++;
    try {
      await this.pull();
    } finally {
      this.busy--;
    }
  }

  private async pull(): Promise<void> {
    const asked = ++this.asked;
    let found: Part[] = [];
    let error = "";
    try {
      found = this.list(this.project);
    } catch (failure) {
      error = plainError(failure);
    }
    const loaded = await Promise.all(found.map((one) => loadPart(one.name, one.file, one.text)));
    if (asked !== this.asked || this.closed) return;
    this.loaded = error ? [bare("extensions", join(this.project, ".furb"), error), ...loaded] : loaded;
    this.follow(found);
    this.version++;
    this.changed();
  }

  /** Loads the parts again soon, once, for a burst of changes. */
  soon(): void {
    if (this.closed) return;
    clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.load(), 30);
  }

  /** Loads the parts again when the disk holds another file than the one that a part loaded from, since a watcher can
   * miss a change, and no later event then tells it. */
  private verify(): void {
    if (this.busy || this.closed) return;
    if (this.loaded.some((one) => one.stamp !== undefined && stamped(one.file) !== one.stamp)) this.soon();
  }

  /** The list of writes that the parts heard last, and how many of it. */
  private heardList?: readonly LiveAct[];
  private heardCount = 0;
  /** Hears the writes of the life that are done, in a list that only grows: a write that lands in the folder `.furb`
   * of the project or in the folder of a part loads the parts again. */
  heard(writes: readonly LiveAct[]): void {
    if (writes !== this.heardList) [this.heardList, this.heardCount] = [writes, 0];
    const fresh = writes.slice(this.heardCount);
    this.heardCount = writes.length;
    let loads = false;
    for (const act of fresh) {
      if (act.kind !== "write" || !act.done || this.heardWrites.has(act.id)) continue;
      this.heardWrites.add(act.id);
      const path = (act.words[0] as { path?: unknown } | null)?.path;
      if (typeof path !== "string") continue;
      const at = resolve(this.project, path);
      const inside = (folder: string) => {
        const way = relative(folder, at);
        return !way.startsWith("..") && !isAbsolute(way);
      };
      if (inside(join(this.project, ".furb")) || this.loaded.some((one) => inside(dirname(one.file))))
        loads = true;
    }
    if (loads) this.soon();
  }

  /** The command of a part of that name. */
  command(name: string): Loaded["commands"][number] | undefined {
    for (const part of this.loaded) {
      const found = part.commands.find((one) => one.name === name);
      if (found) return found;
    }
    return undefined;
  }

  /** The marks that the parts give for the feed of a chain, which shows each chain of its run of compactions, in the
   * order of their places, each with the error of a component that throws as its title. */
  marks(seen: Seen): Mark[] {
    const run = model.compactedRun(seen.chain, seen.compactions);
    const out: Mark[] = [];
    for (const part of this.loaded)
      for (const draw of part.marks)
        try {
          const marks = draw(seen);
          if (!Array.isArray(marks)) throw new Error("Its marks give no list.");
          out.push(...marks.filter((one) => run.includes(one.chain)));
        } catch (error) {
          out.push({
            chain: seen.chain,
            at: 0,
            title: `A mark of ${part.name} failed: ${plainError(error)}`,
          });
        }
    return out.sort((one, other) => one.at - other.at);
  }

  /** The row of an act of a kind that a part gives. */
  row(kind: string): ((act: LiveAct, seen: Seen) => Row) | undefined {
    for (const part of this.loaded) {
      const found = part.rows.get(kind);
      if (found) return found;
    }
    return undefined;
  }

  /** Watches the folder of each part, and no folder that no part has now. */
  private follow(found: Part[]): void {
    const folders = new Set(found.flatMap((one) => (one.text ? [] : [dirname(one.file)])));
    for (const [folder, watcher] of this.watchers)
      if (!folders.has(folder)) {
        watcher.close();
        this.watchers.delete(folder);
      }
    for (const folder of folders) {
      if (this.watchers.has(folder)) continue;
      try {
        const watcher = watch(folder, () => this.soon());
        watcher.on("error", () => watcher.close());
        this.watchers.set(folder, watcher);
      } catch {
        // A folder that cannot be watched loads again on a write that the life makes there.
      }
    }
  }

  dispose(): void {
    this.closed = true;
    clearTimeout(this.timer);
    clearInterval(this.poll);
    for (const watcher of this.watchers.values()) watcher.close();
    this.watchers.clear();
  }
}
