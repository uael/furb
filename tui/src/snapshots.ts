import { type Fact, type Life, type LiveAct, questionKind, unwrapped, type World } from "@furb/engine";
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

/** How many of the last characters of each text in the value of an act a row carries. */
const TAIL = 2000;

/** An act as a row of a snapshot: its value as its plain data, and a text in it longer than a tail cut to its tail,
 * such as what a command printed, with the length of all the texts it cut. */
function row(act: LiveAct): ActRow {
  let output = 0;
  const cut = (value: unknown): unknown => {
    if (typeof value === "string") {
      if (value.length <= TAIL) return value;
      output += value.length;
      return value.slice(-TAIL).replace(/^[\uDC00-\uDFFF]/, "");
    }
    if (Array.isArray(value)) return value.map(cut);
    if (value && typeof value === "object")
      return Object.fromEntries(Object.entries(value).map(([key, one]) => [key, cut(one)]));
    return value;
  };
  const value = cut(unwrapped(act.value));
  return output ? { ...act, value, output } : { ...act, value };
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
    const [kind, id] = fact;
    const act = this.world.activity.acts.get(id);
    // A query of the operator names its chain, and so does every question an extension asks of a chain.
    const chain =
      kind === "stood" ? id : (act?.on ?? (questionKind(id) === kind ? String(fact[3]) : undefined));
    if (["run", "ran", "ready", "wants", "sent"].includes(kind))
      for (const cached of this.chains.values()) cached.actor = undefined;
    const view = chain ? this.chains.get(chain) : undefined;
    if (!view) return;
    // What the transcript says changes the turns, and where the paths of the chain resolve with them, as a cd.
    if (
      ["tell", "ready", "run", "ran", "answer", "ask", "pause", "wake", "close", "cancel"].includes(kind) ||
      (kind === "done" && act) ||
      questionKind(id) === kind
    ) {
      view.turns = undefined;
      view.directory = undefined;
    }
    if (["tell", "ready", "run", "ran"].includes(kind)) view.program = undefined;
    // A stood says the standing that its chain takes, the roster, the directory and the actor, and tells it there.
    if (kind === "stood")
      Object.assign(view, { roster: undefined, directory: undefined, actor: undefined, turns: undefined });
    // A ladder given a word edits the program of the ladder and replays it.
    if (kind === "ladder" && fact.length > 5) {
      view.turns = undefined;
      view.program = undefined;
      view.actor = undefined;
    }
  }
  /** Where the paths of a chain resolve: what its `cwd` gives, and nothing when the chain binds no `cwd`. */
  private directory(chain: string): string {
    try {
      return String(this.life.call("cwd", [], { on: chain }));
    } catch {
      return "";
    }
  }
  /** The view of a chain, with the acts that changed after a count of changes of the act table. */
  take(requested: string, since = 0): Snapshot {
    // An act table derived again knows acts whose facts the views were read without, so every view reads again.
    if (this.generation !== this.world.activity.generation) {
      this.generation = this.world.activity.generation;
      for (const view of this.chains.values())
        Object.assign(view, { program: undefined, turns: undefined, directory: undefined, actor: undefined });
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
    view.turns ??= this.life.turns(selected);
    view.directory ??= this.directory(selected);
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
      paused: this.world.isPaused(selected) || this.world.pending.size > 0,
      dispatched: this.dispatched,
      roster: view.roster,
      program: view.program,
      turns: view.turns,
      directory: view.directory,
      actor: view.actor,
    };
  }
}
