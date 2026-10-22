import { existsSync, readFileSync } from "node:fs";
import { Activity, type Call, type Entry, type Fact, isQuestion, type Turn, WORK } from "@furb/engine";

/** Why a record opens to read: the engine that made it, as the record names it, the engine of this build, and what
 * the life that opened on it raised. */
export interface Reading {
  made: string | null;
  engine: string;
  why: string;
  /** Whether the facts are those of the log, which holds every fact that the life said, or those that the record
   * kept, with the acts that they name. */
  logged: boolean;
}

/** Whether a record is a recorded life: a log of a life whose first line is a header that says so, which a host reads
 * and runs not. tui/script/fixture.ts makes one from the log of a real life. */
export function isRecorded(record: string): boolean {
  if (!existsSync(record)) return false;
  const first = readFileSync(record, "utf8").slice(0, 4096).split("\n")[0] ?? "";
  try {
    return (JSON.parse(first) as { recorded?: unknown }).recorded === true;
  } catch {
    return false;
  }
}

/** What a session that only reads does not do. */
const READS = "This record is open to read, so nothing runs on it. Start a new life from it to go on.";

/** A record open to read, with no life: the facts that the crate retold of it, as an engine and a session give a
 * snapshot of a life, so the TUI draws it as it draws a life. No word runs again, no model is asked, no command
 * runs, and nothing is kept. */
export class Reader {
  readonly root = "chain1";
  readonly activity = new Activity();
  readonly facts: Fact[] = [];
  readonly entries: Entry[] = [];
  readonly pending = new Map<string, string>();
  readonly streams = new Map<string, never>();
  readonly console = { prompts: new Map<string, never>(), notices: new Map<string, never>() };
  readonly changes = { length: 0, read: () => [] };
  /** The facts of each chain, as the transcript of the chain holds them. */
  private readonly held = new Map<string, Fact[]>();
  constructor(
    facts: unknown[],
    readonly directory: string,
    readonly record: string,
    readonly reading?: Reading,
  ) {
    for (const fact of facts as Fact[]) this.hear(fact);
    // The life stopped where the record ends, and nothing runs on a record open to read, so the work that is not done
    // waits, as paused work does. A chain is no work, and stays as it is. A recorded life shows a moment of a life,
    // so its work stands as it stood at that moment.
    if (reading)
      for (const act of this.activity.acts.values())
        if (!act.done && WORK.includes(act.kind)) act.paused = true;
  }
  /** A recorded life, read from its file: a log of a life whose first line is a header that says so, which the
   * reader shows at the moment where the log ends. */
  static recorded(path: string, directory: string): Reader {
    const facts = readFileSync(path, "utf8")
      .split("\n")
      .filter((line) => line.startsWith("["))
      .map((line) => JSON.parse(line) as Fact);
    return new Reader(facts, directory, path);
  }
  /** Whether the reader shows a recorded life, which keeps nothing beside its file. */
  get recorded(): boolean {
    return !this.reading;
  }
  get imageDirectory(): string {
    return `${this.record}.images`;
  }
  get actor(): string {
    return String(this.standing()[2] ?? "operator");
  }

  /** One fact: the activity hears it, and the transcript of its chain holds it. */
  private hear(fact: Fact): void {
    this.facts.push(fact);
    const ear = this.activity.hear(fact);
    for (let step = ear.next(); !step.done; ) {
      const asked = step.value as Call;
      step = ear.next(
        asked.call === "covers" ? this.covers(asked.args[0] as Fact, String(asked.args[1])) : null,
      );
    }
    const [kind, about] = fact;
    // A question stands in the transcript of the chain that it is on, and any other fact in that of what it is about.
    const scope =
      this.activity.scope(isQuestion(kind, about) ? String(fact[3]) : about) || String(fact[3] ?? "");
    const facts = this.held.get(scope) ?? [];
    facts.push(fact);
    this.held.set(scope, facts);
  }

  /** Whether a pause or a wake covers an act: it names the act, the chain of the act, or an act that made it. */
  private covers([, name]: Fact, id: string): boolean {
    if (this.activity.scope(id) === name && name.startsWith("chain")) return true;
    const seen = new Set<string>();
    for (let at: string | undefined = id; at && !seen.has(at); at = this.activity.acts.get(at)?.by) {
      if (at === name) return true;
      seen.add(at);
    }
    return false;
  }

  /** The facts that the transcript of a chain holds. */
  transcript({ on = this.root }: { on?: string } = {}): Fact[] {
    return [...(this.held.get(this.activity.scope(on) || on) ?? [])];
  }

  /** What the root stands on: its roster, its directory and its actor, as the last stand said. */
  standing(): [unknown[], string, string] {
    const stood = this.facts.findLast(([kind, about]) => kind === "done" && isQuestion("stand", about));
    const [roster = [], directory = this.directory, actor = "operator"] = (stood?.[3] ?? []) as unknown[];
    // A recorded life names no path of this machine, so it stands where the host opens it.
    return [roster as unknown[], this.recorded ? this.directory : String(directory), String(actor)];
  }

  /** The word of each rung of a chain, by the rung, as each run of the chain ran it since its last module. */
  program({ on = this.root }: { on?: string } = {}): Record<string, string> {
    let words: Record<string, string> = {};
    for (const fact of this.transcript({ on })) {
      if (fact[0] === "module") words = {};
      if (fact[0] === "run" && isQuestion("run", fact[1]))
        words[String(fact[6] || fact[4])] = String(fact[5]);
    }
    return words;
  }

  /** The turns of a chain: the paragraphs told on it between two answers, and each answer of a model. */
  turns({ on = this.root }: { on?: string } = {}): Turn[] {
    const turns: Turn[] = [];
    let told: string[] = [];
    let cut = 0;
    for (const fact of this.transcript({ on })) {
      const [kind, about] = fact;
      if (kind === "reply" && isQuestion("reply", about)) cut = told.length;
      else if (
        kind === "done" &&
        isQuestion("reply", about) &&
        Array.isArray(fact[3]) &&
        fact[3].length === 4
      ) {
        turns.push(["user", told.slice(0, cut).join("\n\n"), null, null], fact[3] as Turn);
        told = told.slice(cut);
        cut = 0;
      } else if (kind === "tell" && Array.isArray(fact[3])) told.push(fact[3].join("\n"));
    }
    return [...turns, ["user", told.join("\n\n"), null, null]];
  }

  /** Where a chain stands: where it went last, or the directory of the life. */
  cwd({ on = this.root }: { on?: string } = {}): string {
    const went = this.transcript({ on }).findLast(
      ([kind, about]) => kind === "done" && isQuestion("cd", about),
    );
    return went ? String(went[3]) : this.standing()[1];
  }

  inspect(name: string): { value: unknown } {
    if (name !== "actor") throw new Error(READS);
    return { value: this.actor };
  }

  verb(): undefined {
    return undefined;
  }

  outcome(id: string): { done: boolean; value: unknown } {
    const act = this.activity.acts.get(id);
    return { done: act?.done ?? false, value: act?.value ?? null };
  }

  isPaused(id: string): boolean {
    return this.activity.acts.get(id)?.paused ?? false;
  }

  dispose(): void {}

  /** Each other verb, which would make an act or change the life. */
  refuse(): never {
    throw new Error(READS);
  }
}
