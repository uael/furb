import { type Engine, type Fact, isQuestion, type LiveAct, questionKind, type Session } from "@furb/engine";
import { createTwoFilesPatch } from "diff";
import type { Snapshot, Usage } from "./bridge.ts";
import { lineCounts } from "./format.ts";
import { queueDispatches } from "./queue.ts";
import type { ActRow, Exit } from "./session.ts";

/** What a snapshot knows of one chain. A value it must read again is undefined. */
interface ChainView {
  roster?: Snapshot["roster"];
  program?: Snapshot["program"];
  turns?: Snapshot["turns"];
  directory?: string;
  actor?: string;
}

/** How many of the last characters of each stream of a command a row carries. */
const TAIL = 2000;

/** An act as a row of a snapshot: a write carries the lines that its change added and removed, and a command that
 * printed more than a tail carries the tail of each stream. An act carries its time when this life ran it. */
function row(act: LiveAct, lines?: ActRow["lines"], took?: number): ActRow {
  if (took !== undefined) return row({ ...act, took } as ActRow, lines);
  // A read and a write carry the count of the lines of their text, and leave the text out, which the view reads whole
  // when it shows it.
  if (act.kind === "read" || act.kind === "write") {
    const text = (act.kind === "write" ? act.words[0] : act.value) as { content?: unknown } | null;
    const bare = (one: unknown) =>
      one && typeof one === "object" && "content" in one ? { ...one, content: "" } : one;
    const content = typeof text?.content === "string" ? text.content : undefined;
    return {
      ...act,
      words: act.words.map(bare),
      value: bare(act.value),
      ...(content === undefined
        ? {}
        : { count: content ? content.replace(/\n$/, "").split("\n").length : 0 }),
      ...(lines ? { lines } : {}),
    };
  }
  const exit: Exit = act.kind === "bash" ? (act.value as Exit) : {};
  const streams = [exit?.stdout?.content ?? "", exit?.stderr?.content ?? ""];
  if (streams.every((content) => content.length <= TAIL)) return act;
  const tail = (stream?: { content: string }) =>
    stream && { ...stream, content: stream.content.slice(-TAIL).replace(/^[\uDC00-\uDFFF]/, "") };
  return {
    ...act,
    value: { ...exit, stdout: tail(exit.stdout), stderr: tail(exit.stderr) },
    output: streams.join("").length,
  };
}

/** The views of a life that a snapshot reads: those of an engine, or those of a record that is open to read. */
export type Views = Pick<Engine, "root" | "standing" | "program" | "turns" | "cwd" | "verb"> & {
  inspect(name: string, chain?: string): { value?: unknown };
};
/** What a snapshot reads of a session: the facts, the activity and the entries of its life, and, of a life that runs,
 * the changes of its files and the facts that it says as it says them. */
export type Lived = Pick<Session, "facts" | "activity" | "entries" | "isPaused" | "pending"> &
  Partial<Pick<Session, "on">> & { changes?: Pick<Session["changes"], "length" | "read"> };

/** Views belong to changes of the life, not to frames or refresh requests. */
export class Snapshots {
  private readonly chains = new Map<string, ChainView>();
  /** The usage of each answer of each chain, by the chain that the reply stands on. */
  private readonly answers = new Map<string, Usage[]>();
  /** The last reply of each chain that an answer with a usage settled, by the chain that the reply stands on. */
  private readonly replies = new Map<string, string>();
  /** The lines that each write added and removed, by the write, and how many changes of files they are read from. */
  private readonly lines = new Map<string, NonNullable<ActRow["lines"]>>();
  private changes = 0;
  /** When this life saw each act begin, and how long each act that it saw begin took to be done, in the time of the
   * machine. The record holds no time, so an act that the life replayed as it opened took no time that it knows. */
  private readonly began = new Map<string, number>();
  private readonly took = new Map<string, number>();
  /** How many facts the life said as it opened, which it replayed, and how many facts the times have heard. */
  private readonly replayed: number;
  private timed = 0;
  private heard = 0;
  private entries = -1;
  private dispatched: string[] = [];
  constructor(
    private readonly engine: Views,
    private readonly session: Lived,
  ) {
    this.replayed = session.facts.length;
    session.on?.("facts", (facts: Fact[]) => this.time(facts));
  }

  /** The time of the facts that the life said, as the session gives them to its host. */
  private time(facts: readonly Fact[]): void {
    const now = Date.now();
    for (const [kind, id] of facts) {
      if (this.timed++ < this.replayed) continue;
      const began = this.began.get(id);
      if (isQuestion(kind, id)) this.began.set(id, now);
      else if (kind === "done" && began !== undefined) {
        this.began.delete(id);
        this.took.set(id, now - began);
      }
    }
  }

  private changed(fact: Fact): void {
    const [kind, id] = fact;
    // The kind of the question that a done answers.
    const answered = kind === "done" ? questionKind(id) : undefined;
    // A word may bind the actor of its chain again, so the actor is read again at each step of a word.
    if (["run", "ready", "wants"].includes(kind) || answered === "run" || answered === "wants")
      for (const cached of this.chains.values()) cached.actor = undefined;
    // The standing has one home, the root, and every chain that takes it tells it and holds its actor.
    if (answered === "stand")
      for (const cached of this.chains.values())
        Object.assign(cached, {
          roster: undefined,
          directory: undefined,
          actor: undefined,
          turns: undefined,
        });
    // An answer is of the chain its reply stands on. A chain with a source holds the answers of its origin in its
    // prefix, and those are not its own.
    const turn = fact[3];
    if (answered === "reply" && Array.isArray(turn) && Array.isArray(turn[2])) {
      const on = this.session.activity.scope(id);
      const answers = this.answers.get(on) ?? [];
      answers.push(turn[2] as Usage);
      this.answers.set(on, answers);
      this.replies.set(on, id);
    }
    // A fact stands in the transcript of the chain its scope names, which is all that view reads.
    const view = this.chains.get(this.session.activity.scope(id));
    if (!view) return;
    if (["tell", "pause", "wake", "cancel", "close", "reply"].includes(kind) || answered === "reply")
      view.turns = undefined;
    if (kind === "run" || kind === "module") view.program = undefined;
    if (kind === "cd") view.directory = undefined;
    // A write of the door of a prompt edits the program of its ladder and replays it.
    const written = kind === "write" ? fact[4] : answered === "write" ? fact[3] : undefined;
    if (
      written &&
      typeof written === "object" &&
      isQuestion("prompt", String((written as { path?: unknown }).path))
    ) {
      view.turns = undefined;
      view.program = undefined;
      view.actor = undefined;
    }
  }
  /** The view of a chain, with the acts that changed after a count of changes of the act table. */
  take(requested: string, since = 0): Snapshot {
    for (const fact of this.session.facts.slice(this.heard)) this.changed(fact);
    this.heard = this.session.facts.length;
    const selected =
      this.session.activity.acts.get(requested)?.kind === "chain" ? requested : this.engine.root;
    const view = this.chains.get(selected) ?? {};
    this.chains.set(selected, view);
    view.roster ??= (this.engine.standing() as [Snapshot["roster"], string, string])[0];
    view.program ??= this.engine.program({ on: selected }) as Snapshot["program"];
    view.turns ??= this.engine.turns({ on: selected });
    view.directory ??= this.engine.cwd({ on: selected });
    view.actor ??= String(this.engine.inspect("actor", selected).value ?? "");
    if (this.entries !== this.session.entries.length) {
      this.entries = this.session.entries.length;
      this.dispatched = [...queueDispatches(this.session.entries).keys()];
    }
    const files = this.session.changes;
    // A journal that a repair or a new journal made shorter is read again from its start.
    if ((files?.length ?? 0) < this.changes) this.changes = 0;
    try {
      for (const change of files?.read(this.changes, files.length - this.changes) ?? [])
        if (change.act)
          this.lines.set(
            change.act,
            lineCounts(createTwoFilesPatch(change.path, change.path, change.before, change.after)),
          );
      this.changes = files?.length ?? 0;
    } catch (error) {
      // A damaged journal is the failure of the view of changes alone, which offers to repair it: the rows keep the
      // counts that they hold, and the journal is read again once it reads.
      if ((error as { name?: unknown } | undefined)?.name !== "DamagedJournal") throw error;
    }
    const { acts, count } = this.session.activity.since(since);
    // The share of the window has one home, filled of the engine, which the advance binds too.
    const reply = this.replies.get(selected);
    const filled = reply === undefined ? undefined : this.engine.verb("filled", [reply]);
    return {
      selected,
      acts: acts.map((act) => row(act, this.lines.get(act.id), this.took.get(act.id))),
      count,
      paused: this.session.isPaused(selected) || this.session.pending.size > 0,
      dispatched: this.dispatched,
      roster: view.roster,
      program: view.program,
      turns: view.turns,
      answers: [...(this.answers.get(selected) ?? [])],
      filled: typeof filled === "number" ? filled : undefined,
      directory: view.directory,
      actor: view.actor,
    };
  }
}
