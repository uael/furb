import { type Engine, type Fact, isQuestion, type LiveAct, questionKind, type Session } from "@furb/engine";
import type { Snapshot } from "./bridge.ts";
import { queueDispatches } from "./queue.ts";
import type { ActRow } from "./session.ts";

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

/** An act as a row of a snapshot: a command that printed more than a tail carries the tail of each stream. */
function row(act: LiveAct): ActRow {
  const exit =
    act.kind === "bash" ? (act.value as { stdout?: { content: string }; stderr?: { content: string } }) : {};
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

/** Views belong to changes of the life, not to frames or refresh requests. */
export class Snapshots {
  private readonly chains = new Map<string, ChainView>();
  private heard = 0;
  private entries = -1;
  private dispatched: string[] = [];
  constructor(
    private readonly engine: Engine,
    private readonly session: Session,
  ) {}

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
    const { acts, count } = this.session.activity.since(since);
    return {
      selected,
      acts: acts.map(row),
      count,
      paused: this.session.isPaused(selected) || this.session.pending.size > 0,
      dispatched: this.dispatched,
      roster: view.roster,
      program: view.program,
      turns: view.turns,
      directory: view.directory,
      actor: view.actor,
    };
  }
}
