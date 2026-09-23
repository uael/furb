import { EventEmitter } from "node:events";
import { existsSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { basename, dirname, join } from "node:path";
import type { Fact, Turn, Usage } from "@furb/engine";
import { actorParts, shapes } from "@furb/engine";
import type { FileChange } from "@furb/engine/world";
import type { Engine, HostView } from "./bridge.ts";
import { Preferences } from "./preferences.ts";
import { palettes, type ThemeName } from "./theme.ts";

export type View = "conversation" | "program" | "activity" | "facts" | "transcript" | "changes";
export interface ActRow {
  id: string;
  kind: string;
  by: string;
  on: string;
  words: unknown[];
  done: boolean;
  value: unknown;
}

export class Workspace extends EventEmitter {
  selected: string;
  view: View = "conversation";
  actor: string;
  sessionName: string;
  acts: ActRow[] = [];
  turns: Turn[] = [];
  rendered: string[] = [];
  changes: FileChange[] = [];
  changePage = 0;
  program: Record<string, string> = {};
  query = "";
  notice = "";
  readonly errors: Record<string, string> = {};
  loading = false;
  paused = false;
  editing?: string;
  ladder?: string;
  theme: ThemeName;
  readonly preferences: Preferences;
  mode: "prompt" | "python" = "prompt";
  shape = "str";
  drafts: Record<string, string> = {};
  scrolls: Record<string, number> = {};
  panes = { inspector: 28 };
  toggled: string[] = [];
  roster: [string, string[], number][] = [];
  findings: string[] = [];
  rejectedWord = "";
  rejectedAct = "";
  repls: Record<string, string[]> = {};
  private savedCost = 0;
  histories: Record<string, string[]> = {};
  started: Record<string, number> = {};
  directory = "";
  private refreshTask?: Promise<void>;
  private dirty = false;
  private closed = false;
  private factFilter = { query: "", seen: 0, rows: [] as Fact[] };
  constructor(
    readonly life: Engine,
    readonly world: HostView,
    readonly demo = false,
    preferences?: Preferences,
  ) {
    super();
    this.selected = life.root;
    this.actor = `${world.model}/${world.effort}`;
    this.sessionName = basename(world.directory);
    const path = world.records.path;
    this.preferences =
      preferences ?? new Preferences(demo && path ? join(dirname(path), "ui-preferences.json") : undefined);
    this.theme = this.preferences.theme;
    if (path && existsSync(`${path}.ui.json`)) {
      const saved = JSON.parse(readFileSync(`${path}.ui.json`, "utf8")) as Partial<Workspace> & {
        cost?: number;
        collapsed?: string[];
        expanded?: string[];
      };
      this.savedCost = saved.cost ?? 0;
      this.selected = saved.selected ?? this.selected;
      this.actor = saved.actor ?? this.actor;
      this.demo = saved.demo ?? this.demo;
      this.sessionName = saved.sessionName ?? this.sessionName;
      this.view = saved.view ?? this.view;
      this.mode = saved.mode ?? this.mode;
      this.shape = saved.shape ?? this.shape;
      this.editing = saved.editing;
      this.histories = saved.histories ?? {};
      this.started = saved.started ?? {};
      this.repls = saved.repls ?? {};
      this.ladder = saved.ladder;
      this.drafts = saved.drafts ?? {};
      this.scrolls = saved.scrolls ?? {};
      this.panes = { inspector: saved.panes?.inspector ?? this.panes.inspector };
      this.toggled = saved.toggled ?? [...(saved.collapsed ?? []), ...(saved.expanded ?? [])];
    }
    world.on("change", this.changed);
    world.on("facts", this.factsChanged);
    world.on("fault", this.fail);
    this.save();
  }
  private changed = () => {
    this.emit("change");
  };
  private factsChanged = (facts: Fact[]) => {
    if (
      facts.some(
        ([kind, id]) =>
          [
            "chain",
            "prompt",
            "rung",
            "bash",
            "wait",
            "grant",
            "answer",
            "out",
            "exited",
            "close",
            "cancel",
            "pause",
            "wake",
            "tell",
          ].includes(kind) ||
          (kind === "done" && /^(prompt|rung|bash|wait|grant):/.test(id)),
      )
    )
      void this.refresh().catch(this.fail);
  };
  fail = (error: unknown) => {
    this.error = error instanceof Error ? error.message : String(error);
    this.emit("change");
  };
  get error(): string {
    return this.errors[`${this.selected}:${this.view}`] ?? "";
  }
  set error(value: string) {
    this.errors[`${this.selected}:${this.view}`] = value;
  }

  refresh(): Promise<void> {
    this.dirty = true;
    if (this.refreshTask) return this.refreshTask;
    this.loading = true;
    this.emit("change");
    this.refreshTask = (async () => {
      do {
        this.dirty = false;
        const selected = this.selected;
        const snapshot = await this.world.snapshot(selected);
        if (this.selected !== selected) {
          this.dirty = true;
          continue;
        }
        Object.assign(this, snapshot);
        if (this.view === "changes") this.changes = await this.world.readChanges(this.changePage * 20, 20);
        const rows = this.acts;
        for (const act of rows) if (!act.done) this.started[act.id] ??= Date.now();
        const lastWord = rows
          .filter((act) => act.on === this.selected && act.kind === "rung" && act.by === "operator")
          .at(-1);
        const refused = this.turns
          .flatMap((turn) => turn[1])
          .find(
            (tag) =>
              typeof tag !== "string" &&
              tag[0] === "refused" &&
              tag[1].some(([key, value]) => key === "id" && value === lastWord?.id),
          );
        this.findings =
          refused && typeof refused !== "string"
            ? typeof refused[2] === "string"
              ? refused[2].split("\n")
              : Array.isArray(refused[2])
                ? refused[2].map(String)
                : []
            : [];
        this.rejectedWord = this.findings.length ? String(lastWord?.words[0] ?? "") : "";
        this.rejectedAct = this.findings.length ? (lastWord?.id ?? "") : "";
        this.emit("change");
      } while (this.dirty && !this.closed);
    })().finally(() => {
      this.refreshTask = undefined;
      this.loading = false;
      if (!this.closed) this.emit("change");
    });
    return this.refreshTask;
  }

  get chains(): ActRow[] {
    return this.acts.filter((act) => act.kind === "chain");
  }
  get activity(): ActRow[] {
    return this.acts.filter((act) => act.on === this.selected && act.kind !== "chain");
  }
  get usage(): Usage {
    return this.turns.reduce<Usage>(
      (sum, turn) => (turn[2] ? (sum.map((value, index) => value + (turn[2]?.[index] ?? 0)) as Usage) : sum),
      [0, 0, 0, 0, 0],
    );
  }
  get label(): string {
    return this.labelOf(this.selected);
  }
  get actorChoice(): { model: string; effort: string } {
    return actorParts(
      this.actor,
      this.roster.map(([name]) => name),
    );
  }
  get operatorPrompt() {
    return [...this.world.prompts.values()].find(
      (prompt) => this.acts.find((act) => act.id === prompt.id)?.on === this.selected,
    );
  }
  labelOf(id: string): string {
    return id === this.life.root
      ? "Main"
      : String(this.chains.find((chain) => chain.id === id)?.words[0] || "Chain");
  }
  async select(id: string): Promise<void> {
    if (id !== this.selected) {
      this.ladder = undefined;
      this.editing = undefined;
    }
    this.selected = id;
    this.query = "";
    await this.refresh();
  }
  show(view: View): void {
    this.view = view;
    this.query = "";
    this.emit("change");
    void this.refresh().catch(this.fail);
  }

  private track(id: string): void {
    void this.life
      .result(id)
      .then(
        () => {
          this.notice = "Work complete.";
        },
        async (error: unknown) => {
          if (this.closed) return;
          // A finished act carries its failure in the record and in the feed.
          if (!(await this.life.outcome(id)).done) throw error;
          if (error instanceof Error && error.message.startsWith("CancelledError"))
            this.notice = "Work cancelled.";
        },
      )
      .then(() => {
        if (!this.closed) return this.refresh();
      })
      .catch(this.fail);
  }

  async submit(input: string): Promise<void> {
    const text = input.trim();
    if (!text) return;
    this.error = "";
    if (this.editing) {
      await this.life.result(
        await this.life.rung(`write(Text(${JSON.stringify(this.editing)}, ${JSON.stringify(input)}))`, {
          on: this.selected,
        }),
      );
      this.notice = "Program updated and replayed.";
      this.editing = undefined;
    } else if (text.startsWith("/")) await this.command(text);
    else {
      const pending = this.operatorPrompt;
      if (pending) await this.world.answer(pending.id, input);
      else {
        const held = this.world.held.size > 0;
        const id = await this.life.prompt(this.shape, input, { on: this.selected, to: this.actor });
        if (held) this.emit("resume");
        this.track(id);
        this.notice = "The model is working.";
      }
    }
    await this.refresh();
  }

  async command(text: string): Promise<void> {
    const space = text.indexOf(" ");
    const command = text.slice(1, space < 0 ? undefined : space);
    const argument = space < 0 ? "" : text.slice(space + 1).trim();
    switch (command) {
      case "name":
        this.sessionName = argument || basename(this.world.directory);
        this.save();
        break;
      case "pause":
        await this.life.pause(argument || this.selected);
        this.notice = "Paused. In-flight work can finish.";
        break;
      case "wake":
        if (this.world.held.size) {
          this.emit("resume");
          break;
        }
        await this.life.wake(argument || this.selected);
        this.notice = "Work resumed.";
        break;
      case "cancel":
        await this.life.cancel(argument || this.selected);
        this.notice = "Work cancelled.";
        break;
      case "chain":
        await this.select(await this.life.chain(argument || "New chain"));
        break;
      case "fork":
        await this.select(await this.life.chain(argument || `${this.label} fork`, this.selected));
        break;
      case "details":
        this.emit("details");
        break;
      case "rewind":
        this.emit("rewind");
        break;
      case "grant": {
        const amount = Number(argument);
        if (!argument || !Number.isFinite(amount) || amount < 0)
          throw new Error("Use /grant followed by a dollar amount.");
        const id = await this.life.grant({ usd: amount, on: this.selected });
        const got = await this.life.outcome(id);
        if (got.done) throw new Error(JSON.stringify(got.value));
        this.notice = `Budget set to $${amount.toFixed(2)}. Use /wake if paused.`;
        break;
      }
      case "share": {
        const amount = Number(argument);
        if (!argument || !Number.isFinite(amount) || amount < 0 || amount > 1)
          throw new Error("Use /share with a number from 0 to 1.");
        await this.life.grant({ share: amount, on: this.selected });
        break;
      }
      case "model": {
        if (!argument) {
          this.emit("models");
          break;
        }
        const entry = this.roster.find(([name]) => name === argument && name !== "operator");
        if (!entry) throw new Error("Choose a model in this chain's roster.");
        const current = this.actorChoice.effort;
        const effort = entry[1].includes(current) ? current : entry[1][0];
        this.actor = effort ? `${argument}/${effort}` : argument;
        this.track(await this.life.rung(`actor = ${JSON.stringify(this.actor)}`, { on: this.selected }));
        break;
      }
      case "effort": {
        if (!argument) {
          this.emit("efforts");
          break;
        }
        const { model } = this.actorChoice;
        const offered = this.roster.find(([name]) => name === model)?.[1] ?? [];
        if (!offered.includes(argument))
          throw new Error(`This model offers ${offered.join(", ") || "no reasoning efforts"}.`);
        this.actor = `${model}/${argument}`;
        this.track(await this.life.rung(`actor = ${JSON.stringify(this.actor)}`, { on: this.selected }));
        break;
      }
      case "run": {
        this.findings = [];
        const id = await this.life.rung(argument, { on: this.selected });
        if (this.ladder) {
          this.repls[this.ladder] ??= [];
          this.repls[this.ladder]?.push(id);
        }
        this.view = "program";
        if ((await this.life.outcome(id)).done) {
          try {
            await this.life.result(id);
          } catch {
            await this.refresh();
          }
        } else this.track(id);
        break;
      }
      case "shape":
        if (!shapes.some((name) => name === argument)) throw new Error(`Choose ${shapes.join(", ")}.`);
        this.shape = argument;
        break;
      case "inspect":
        this.emit("inspect", argument);
        break;
      case "theme":
        if (!(argument in palettes)) throw new Error(`Choose ${Object.keys(palettes).join(", ")}.`);
        this.theme = argument as ThemeName;
        this.preferences.save(this.theme);
        break;
      case "bash":
        this.track(await this.life.bash(argument, { on: this.selected }));
        this.view = "activity";
        break;
      case "read": {
        this.track(await this.life.rung(`read(${JSON.stringify(argument)})`, { on: this.selected }));
        this.view = "conversation";
        break;
      }
      case "cd":
        this.track(await this.life.rung(`cd(${JSON.stringify(argument)})`, { on: this.selected }));
        break;
      case "edit": {
        const id = argument || [...this.activity].reverse().find((act) => act.kind === "prompt")?.id;
        if (!id) throw new Error("There is no prompt program to edit.");
        const got = (await this.life.read(id, undefined, this.selected)) as { content: string };
        this.editing = id;
        this.emit("compose", this.drafts[`${this.selected}:${id}`] ?? got.content);
        this.notice = "Edit the Python program, then submit to replay it.";
        break;
      }
      case "feed": {
        const [id, ...words] = argument.split(" ");
        if (!id) throw new Error("Use /feed followed by an act id and text.");
        await this.life.write({ path: `${id}/stdin`, content: words.join(" ") }, this.selected);
        break;
      }
      case "close": {
        const split = argument.indexOf(" ");
        if (split < 0) throw new Error("Use /close followed by an act id and a JSON value.");
        await this.life.close(JSON.parse(argument.slice(split + 1)), argument.slice(0, split));
        break;
      }
      case "export": {
        if (!argument) throw new Error("Use /export followed by a file path.");
        await writeFile(
          argument,
          JSON.stringify({ chain: this.selected, turns: this.turns, program: this.program }, null, 2),
          { flag: "wx" },
        );
        this.notice = `Exported to ${argument}.`;
        break;
      }
      default:
        throw new Error(`Unknown command /${command}. Press F1 for the command list.`);
    }
  }

  filteredFacts(): Fact[] {
    const query = this.query.toLowerCase();
    if (this.factFilter.query !== query) this.factFilter = { query, seen: 0, rows: [] };
    this.factFilter.rows.push(
      ...this.world.facts
        .slice(this.factFilter.seen)
        .filter((fact) => !query || JSON.stringify(fact).toLowerCase().includes(query)),
    );
    this.factFilter.seen = this.world.facts.length;
    return this.factFilter.rows;
  }
  save(): void {
    this.preferences.save(this.theme);
    const record = this.world.records.path;
    if (!record) return;
    const path = `${record}.ui.json`;
    this.savedCost = Math.max(
      this.savedCost,
      this.world.facts
        .filter((fact) => fact[0] === "answer")
        .reduce((sum, fact) => sum + ((fact[3] as Turn)[2]?.[4] ?? 0), 0),
    );
    writeFileSync(
      `${path}.tmp`,
      JSON.stringify({
        sessionName: this.sessionName,
        demo: this.demo,
        cost: this.savedCost,
        selected: this.selected,
        actor: this.actor,
        view: this.view,
        mode: this.mode,
        shape: this.shape,
        editing: this.editing,
        histories: this.histories,
        started: this.started,
        ladder: this.ladder,
        repls: this.repls,
        drafts: this.drafts,
        scrolls: this.scrolls,
        panes: this.panes,
        toggled: this.toggled,
      }),
      { mode: 0o600 },
    );
    renameSync(`${path}.tmp`, path);
  }
  async dispose(): Promise<void> {
    this.save();
    this.closed = true;
    this.world.off("change", this.changed);
    this.world.off("fault", this.fail);
    this.world.off("facts", this.factsChanged);
    await this.refreshTask?.catch(() => {});
    await this.world.dispose();
  }
}
