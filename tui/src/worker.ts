import type { Life } from "@furb/engine";
import { Act, engineSource, World, type WorldOptions } from "@furb/engine";
import type { Snapshot, WorldState } from "./bridge.ts";
import { createDemoWorld } from "./demo.ts";
import { queueDispatches, queueHash } from "./queue.ts";
import type { FollowUp } from "./session.ts";

declare const self: Worker & { close(): void };
let world: World | undefined;
let life: Life | undefined;
let timer: ReturnType<typeof setTimeout> | undefined;
let sentFacts = 0;
let sentChanges = 0;
const state = () => {
  const owner = world;
  if (!owner) return;
  const snapshot: WorldState = {
    directory: owner.directory,
    imageDirectory: owner.imageDirectory,
    model: owner.model,
    effort: owner.effort,
    roster: owner.roster,
    path: owner.records.path,
    facts: owner.facts.slice(sentFacts),
    prompts: [...owner.prompts.values()].map(({ id, shape, message }) => ({ id, shape, message })),
    streams: [...owner.streams],
    held: [...owner.held],
    changes: owner.changes.paths.slice(sentChanges),
    models: owner.roster.map((name) => owner.route(name)),
  };
  sentFacts = owner.facts.length;
  sentChanges = owner.changes.length;
  self.postMessage({ state: snapshot });
};
self.onmessage = async ({ data }) => {
  try {
    let value: unknown;
    if (data.method === "open") {
      const options = data.args[0] as WorldOptions & { demo?: boolean };
      world = options.demo ? await createDemoWorld(options.record, options.cwd) : new World(options);
      life = world.open();
      const tail = world.records.entries.at(-1)?.[1];
      if (tail?.[0] === "queue_begin" && tail[2] === "operator")
        life.send("queue_aborted", tail[1], [tail[3]]);
      world.on("fault", (error) =>
        self.postMessage({ fault: error instanceof Error ? error.message : String(error) }),
      );
      world.on("change", () => {
        timer ??= setTimeout(() => {
          timer = undefined;
          state();
        }, 20);
      });
      value = life.root;
    } else if (data.target === "library" && data.method === "queue") {
      if (!life || !world) throw new Error("The session is not open.");
      const entry = data.args[0] as FollowUp;
      const prior = queueDispatches(world.records.entries).get(entry.id);
      if (prior) value = prior;
      else {
        life.send("queue_begin", entry.chain, [
          entry.id,
          queueHash(entry.chain, entry.shape, entry.text, entry.actor),
        ]);
        const act = life.prompt(entry.shape, entry.text, { on: entry.chain, to: entry.actor });
        life.send("queue_sent", entry.chain, [entry.id, act.id]);
        value = act.id;
      }
    } else if (data.target === "library" && data.method === "source") {
      value = engineSource();
    } else if (data.target === "library" && data.method === "changes") {
      value = world?.changes.read(data.args[0], data.args[1]);
    } else if (data.target === "library" && data.method === "snapshot") {
      if (!life) throw new Error("The session is not open.");
      const owner = life;
      const acts = (owner.held("acts", [], "keys") as string[]).map((id) => {
        const fact = owner.get(id);
        const outcome = owner.outcome(id);
        return {
          id,
          kind: fact[0],
          by: fact[2],
          on: fact[0] === "chain" ? id : fact[3],
          words: fact.slice(4),
          done: outcome.done,
          paused: world?.isPaused(id) ?? false,
          run: fact[0] === "rung" ? world?.rungState(id) : undefined,
          value: !outcome.done && fact[0] === "bash" ? owner.peek(id) : outcome.value,
        };
      });
      const selected = acts.some((act) => act.id === data.args[0]) ? (data.args[0] as string) : owner.root;
      const [, program] = owner.call<[unknown, Record<string, string>]>("ask", ["program", selected], {});
      const [, [roster]] = owner.call<[unknown, [Snapshot["roster"], string, string]]>(
        "ask",
        ["stand", selected],
        {},
      );
      value = {
        dispatched: world ? [...queueDispatches(world.records.entries).keys()] : [],
        roster,
        acts,
        selected,
        paused: world?.isPaused(selected) ?? false,
        turns: owner.turns(selected),
        rendered: owner.rendered(selected),
        program: program ?? {},
        actor: owner.held("modules", [selected, "actor"], "at") as string,
        directory: owner.cwd(selected),
      } satisfies Snapshot;
    } else {
      const target = data.target === "world" ? world : life;
      if (!target) throw new Error("The session is not open.");
      const method = Reflect.get(target, data.method) as (...args: unknown[]) => unknown;
      if (typeof method !== "function") throw new Error(`Unknown ${data.target} method ${data.method}.`);
      const result = Reflect.apply(method, target, data.args);
      value = result instanceof Act ? result.id : await result;
    }
    const disposed = data.target === "world" && data.method === "dispose";
    if (data.method === "open") state();
    if (disposed) {
      if (timer) clearTimeout(timer);
      world = undefined;
      life = undefined;
      setTimeout(() => {
        self.postMessage({ id: data.id, value, disposed });
        self.close();
      }, 0);
    } else self.postMessage({ id: data.id, value, disposed });
  } catch (error) {
    self.postMessage({ id: data.id, error: error instanceof Error ? error.message : String(error) });
  }
};
