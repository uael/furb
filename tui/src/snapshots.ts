import type { Fact, Life, World } from "@furb/engine";
import type { Snapshot } from "./bridge.ts";
import { queueDispatches } from "./queue.ts";

interface ChainView {
  roster?: Snapshot["roster"];
  program: Snapshot["program"];
  turns: Snapshot["turns"];
  rendered: string[];
  directory: string;
  actor: string;
  textDirty: boolean;
  programDirty: boolean;
  directoryDirty: boolean;
  actorDirty: boolean;
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
    const query = kind === "done" ? id.split("://")[0] : undefined;
    const chain = ["cd", "write"].includes(kind)
      ? String(fact[3])
      : (act?.on ?? (query === "cd" || query === "write" ? this.world.activity.acts.get(by)?.on : undefined));
    const view = chain ? this.chains.get(chain) : undefined;
    if (["run", "ran", "ready", "wants", "sent"].includes(kind))
      for (const cached of this.chains.values()) cached.actorDirty = true;
    if (!view) return;
    if (
      ["tell", "ready", "run", "ran", "answer", "ask", "pause", "wake", "close", "cancel", "cd"].includes(
        kind,
      ) ||
      (kind === "done" && act)
    )
      view.textDirty = true;
    if (["tell", "ready", "run", "ran"].includes(kind)) view.programDirty = true;
    if (kind === "cd" || query === "cd") {
      view.directoryDirty = true;
      view.textDirty = true;
    }
    if (
      query === "write" &&
      fact[3] &&
      typeof fact[3] === "object" &&
      "path" in fact[3] &&
      String(fact[3].path).startsWith("prompt://")
    ) {
      view.textDirty = true;
      view.programDirty = true;
      view.actorDirty = true;
    }
    if (
      kind === "write" &&
      fact[4] &&
      typeof fact[4] === "object" &&
      "path" in fact[4] &&
      String(fact[4].path).startsWith("prompt://")
    ) {
      view.textDirty = true;
      view.programDirty = true;
      view.actorDirty = true;
    }
  }
  /** The view of a chain, with the acts that changed after a count of changes of the act table. */
  take(requested: string, since = 0): Snapshot {
    // An act table derived again knows acts whose facts the views were read without, so every view reads again.
    if (this.generation !== this.world.activity.generation) {
      this.generation = this.world.activity.generation;
      for (const view of this.chains.values())
        Object.assign(view, { textDirty: true, programDirty: true, directoryDirty: true, actorDirty: true });
    }
    for (const fact of this.world.facts.slice(this.heard)) this.changed(fact);
    this.heard = this.world.facts.length;
    const selected = this.world.activity.acts.get(requested)?.kind === "chain" ? requested : this.life.root;
    let view = this.chains.get(selected);
    if (!view) {
      view = {
        program: {},
        turns: [],
        rendered: [],
        directory: "",
        actor: "",
        textDirty: true,
        programDirty: true,
        directoryDirty: true,
        actorDirty: true,
      };
      this.chains.set(selected, view);
    }
    if (!view.roster) {
      const [, [roster]] = this.life.call<[unknown, [Snapshot["roster"], string, string]]>(
        "ask",
        ["stand", selected],
        {},
      );
      view.roster = roster;
    }
    if (view.programDirty) {
      view.programDirty = false;
      view.program =
        this.life.call<[unknown, Record<string, string>]>("ask", ["program", selected], {})[1] ?? {};
    }
    if (view.textDirty) {
      view.textDirty = false;
      ({ turns: view.turns, rendered: view.rendered } = this.life.rendering(selected));
    }
    if (view.directoryDirty) {
      view.directoryDirty = false;
      view.directory = this.life.cwd(selected);
    }
    if (view.actorDirty) {
      view.actorDirty = false;
      view.actor = this.life.held("modules", [selected, "actor"], "at") as string;
    }
    if (this.entries !== this.world.records.entries.length) {
      this.entries = this.world.records.entries.length;
      this.dispatched = [...queueDispatches(this.world.records.entries).keys()];
    }
    const { acts, whole, count } = this.world.activity.since(since);
    return {
      selected,
      acts,
      whole,
      count,
      paused: this.world.isPaused(selected) || this.world.held.size > 0,
      dispatched: this.dispatched,
      roster: view.roster,
      program: view.program,
      turns: view.turns,
      rendered: view.rendered,
      directory: view.directory,
      actor: view.actor,
    };
  }
}
