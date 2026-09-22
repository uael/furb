import { EventEmitter } from "node:events";
import { existsSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { basename } from "node:path";
import type { Fact, Turn, Usage } from "@furb/engine";
import type { Engine, HostView } from "./bridge.ts";
import type { ThemeName } from "./theme.ts";

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
  program: Record<string, string> = {};
  query = "";
  notice = "Ready when you are.";
  error = "";
  editing?: string;
  theme: ThemeName = "forest";
  mode: "prompt" | "python" = "prompt";
  shape = "str";
  drafts: Record<string, string> = {};
  scrolls: Record<string, number> = {};
  panes = { sidebar: 25, inspector: 31 };
  collapsed: string[] = [];
  findings: string[] = [];
  directory = "";
  private refreshTask?: Promise<void>;
  private dirty = false;
  private closed = false;
  private factFilter = { query: "", seen: 0, rows: [] as Fact[] };
  constructor(
    readonly life: Engine,
    readonly world: HostView,
    readonly demo = false,
  ) {
    super();
    this.selected = life.root;
    this.actor = `${world.model}/${world.effort}`;
    this.sessionName = basename(world.directory);
    const path = world.records.path;
    if (path && existsSync(`${path}.ui.json`)) {
      const saved = JSON.parse(readFileSync(`${path}.ui.json`, "utf8")) as Partial<Workspace>;
      this.selected = saved.selected ?? this.selected;
      this.actor = saved.actor ?? this.actor;
      this.demo = saved.demo ?? this.demo;
      this.sessionName = saved.sessionName ?? this.sessionName;
      this.view = saved.view ?? this.view;
      this.theme = saved.theme ?? this.theme;
      this.mode = saved.mode ?? this.mode;
      this.shape = saved.shape ?? this.shape;
      this.editing = saved.editing;
      this.drafts = saved.drafts ?? {};
      this.scrolls = saved.scrolls ?? {};
      this.panes = saved.panes ?? this.panes;
      this.collapsed = saved.collapsed ?? [];
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

  refresh(): Promise<void> {
    this.dirty = true;
    if (this.refreshTask) return this.refreshTask;
    this.refreshTask = (async () => {
      do {
        this.dirty = false;
        const ids = (await this.life.held("acts", [], "keys")) as string[];
        const rows = await Promise.all(
          ids.map(async (id) => {
            const [fact, outcome] = await Promise.all([this.life.get(id), this.life.outcome(id)]);
            const value = !outcome.done && fact[0] === "bash" ? await this.life.peek(id) : outcome.value;
            return {
              id,
              kind: fact[0],
              by: fact[2],
              on: fact[0] === "chain" ? id : fact[3],
              words: fact.slice(4),
              done: outcome.done,
              value,
            };
          }),
        );
        this.acts = rows;
        if (!rows.some((act) => act.id === this.selected)) this.selected = this.life.root;
        this.turns = await this.life.turns(this.selected);
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
        const [, program] = (await this.life.call("ask", ["program", this.selected], {})) as [
          unknown,
          Record<string, string>,
        ];
        this.program = program ?? {};
        this.actor = (await this.life.held("modules", [this.selected, "actor"], "at")) as string;
        this.directory = await this.life.cwd(this.selected);
        this.emit("change");
      } while (this.dirty && !this.closed);
    })().finally(() => {
      this.refreshTask = undefined;
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
  get paused(): boolean {
    return (
      [...this.world.facts]
        .reverse()
        .find((fact) => fact[1] === this.selected && ["pause", "wake"].includes(fact[0]))?.[0] === "pause"
    );
  }
  get label(): string {
    return this.labelOf(this.selected);
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
    this.selected = id;
    this.query = "";
    await this.refresh();
  }
  show(view: View): void {
    this.view = view;
    this.query = "";
    this.emit("change");
  }

  private track(id: string): void {
    void this.life
      .result(id)
      .then(() => {
        this.notice = "Work complete.";
        return this.refresh();
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
        const active = this.activity.some((act) => act.kind === "prompt" && !act.done);
        const held = this.world.held.size > 0;
        if (active || held) await this.life.pause(this.selected);
        const id = await this.life.prompt(this.shape, input, { on: this.selected, to: this.actor });
        if (held) this.emit("resume");
        else if (active) await this.life.wake(this.selected);
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
        const model = argument.split("/")[0] ?? "";
        if (!this.world.roster.includes(model)) throw new Error("Choose a model in this life's roster.");
        this.actor = argument.includes("/") ? argument : `${argument}/${this.world.effort}`;
        this.track(await this.life.rung(`actor = ${JSON.stringify(this.actor)}`, { on: this.selected }));
        this.notice = `Model set to ${this.actor}.`;
        break;
      }
      case "run": {
        this.findings = [];
        const id = await this.life.rung(argument, { on: this.selected });
        this.view = "program";
        if ((await this.life.outcome(id)).done) {
          try {
            await this.life.result(id);
          } catch (error) {
            await this.refresh();
            throw this.findings.length ? new Error(this.findings.join("\n")) : error;
          }
        } else this.track(id);
        break;
      }
      case "shape":
        if (!["None", "str", "int", "float", "bool", "list", "dict"].includes(argument))
          throw new Error("Choose None, str, int, float, bool, list, or dict.");
        this.shape = argument;
        break;
      case "inspect":
        this.emit("inspect", argument);
        break;
      case "theme":
        if (!["forest", "paper", "midnight"].includes(argument))
          throw new Error("Choose forest, paper, or midnight.");
        this.theme = argument as ThemeName;
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
    const record = this.world.records.path;
    if (!record) return;
    const path = `${record}.ui.json`;
    writeFileSync(
      `${path}.tmp`,
      JSON.stringify({
        sessionName: this.sessionName,
        demo: this.demo,
        cost: this.world.facts
          .filter((fact) => fact[0] === "answer")
          .reduce((sum, fact) => sum + ((fact[3] as Turn)[2]?.[4] ?? 0), 0),
        selected: this.selected,
        actor: this.actor,
        view: this.view,
        theme: this.theme,
        mode: this.mode,
        shape: this.shape,
        editing: this.editing,
        drafts: this.drafts,
        scrolls: this.scrolls,
        panes: this.panes,
        collapsed: this.collapsed,
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
