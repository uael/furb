import type { Fact, Life, LiveAct, World } from "@furb/engine";
import type { Snapshot } from "./bridge.ts";
import { queueDispatches } from "./queue.ts";
import type { ActRow } from "./session.ts";

/** What a snapshot knows of one chain. A value it must read again is undefined. */
interface ChainView {
  roster?: Snapshot["roster"];
  program?: Snapshot["program"];
  text?: { turns: Snapshot["turns"]; rendered: string[] };
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

/** Queries belong to changes of the life, not to frames or refresh requests. */
export class Snapshots {
  private readonly chains = new Map<string, ChainView>();
  private heard = 0;
  private generation = 0;
  private entries = -1;
  private dispatched: string[] = [];
  constructor(
    private readonly life: Life,
    private readonly world: World,
  ) {}

  private changed(fact: Fact): void {
    const [kind, id, by] = fact;
    const act = this.world.activity.acts.get(id);
    // The kind of the question that a done answers.
    const answered = kind === "done" ? id.split("://")[0] : undefined;
    const chain = ["cd", "write"].includes(kind)
      ? String(fact[3])
      : kind === "stood"
        ? id
        : (act?.on ??
          (answered === "cd" || answered === "write" ? this.world.activity.acts.get(by)?.on : undefined));
    if (["run", "ran", "ready", "wants", "sent"].includes(kind))
      for (const cached of this.chains.values()) cached.actor = undefined;
    const view = chain ? this.chains.get(chain) : undefined;
    if (!view) return;
    if (
      ["tell", "ready", "run", "ran", "answer", "ask", "pause", "wake", "close", "cancel", "cd"].includes(
        kind,
      ) ||
      (kind === "done" && act)
    )
      view.text = undefined;
    if (["tell", "ready", "run", "ran"].includes(kind)) view.program = undefined;
    // A stood says the standing that its chain takes, the roster, the directory and the actor, and tells it there.
    if (kind === "stood")
      Object.assign(view, { roster: undefined, directory: undefined, actor: undefined, text: undefined });
    if (kind === "cd" || answered === "cd") {
      view.directory = undefined;
      view.text = undefined;
    }
    // A write of the door of a prompt edits the program of its ladder and replays it.
    const written = kind === "write" ? fact[4] : answered === "write" ? fact[3] : undefined;
    if (
      written &&
      typeof written === "object" &&
      String((written as { path?: unknown }).path).startsWith("prompt://")
    ) {
      view.text = undefined;
      view.program = undefined;
      view.actor = undefined;
    }
  }
  /** The view of a chain, with the acts that changed after a count of changes of the act table. */
  take(requested: string, since = 0): Snapshot {
    // An act table derived again knows acts whose facts the views were read without, so every view reads again.
    if (this.generation !== this.world.activity.generation) {
      this.generation = this.world.activity.generation;
      for (const view of this.chains.values())
        Object.assign(view, { program: undefined, text: undefined, directory: undefined, actor: undefined });
    }
    for (const fact of this.world.facts.slice(this.heard)) this.changed(fact);
    this.heard = this.world.facts.length;
    const selected = this.world.activity.acts.get(requested)?.kind === "chain" ? requested : this.life.root;
    const view = this.chains.get(selected) ?? {};
    this.chains.set(selected, view);
    view.roster ??= this.life.call<[unknown, [Snapshot["roster"], string, string]]>(
      "ask",
      ["stand", selected],
      {},
    )[1][0];
    view.program ??=
      this.life.call<[unknown, Record<string, string>]>("ask", ["program", selected], {})[1] ?? {};
    view.text ??= this.life.rendering(selected);
    view.directory ??= this.life.cwd(selected);
    view.actor ??= this.life.held("modules", [selected, "actor"], "at") as string;
    if (this.entries !== this.world.records.entries.length) {
      this.entries = this.world.records.entries.length;
      this.dispatched = [...queueDispatches(this.world.records.entries).keys()];
    }
    const { acts, whole, count } = this.world.activity.since(since);
    return {
      selected,
      acts: acts.map(row),
      whole,
      count,
      paused: this.world.isPaused(selected) || this.world.held.size > 0,
      dispatched: this.dispatched,
      roster: view.roster,
      program: view.program,
      turns: view.text.turns,
      rendered: view.text.rendered,
      directory: view.directory,
      actor: view.actor,
    };
  }
}
