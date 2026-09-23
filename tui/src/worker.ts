import type { Life } from "@furb/engine";
import { Act, engineSource, World } from "@furb/engine";
import type { WorldState } from "./bridge.ts";
import { createDemoWorld } from "./demo.ts";
import { type EngineOptions, hostModels } from "./models.ts";
import { queueDispatches, queueEvent, queueHash } from "./queue.ts";
import type { FollowUp } from "./session.ts";
import { Snapshots } from "./snapshots.ts";

declare const self: Worker & { close(): void };
let world: World | undefined;
let life: Life | undefined;
let snapshots: Snapshots | undefined;
let host: ReturnType<typeof hostModels> | undefined;
let timer: ReturnType<typeof setTimeout> | undefined;
let sentFacts = 0;
let sentChanges = 0;
const state = () => {
  const owner = world;
  if (!owner) return;
  const snapshot: WorldState = {
    completed: owner.activity.completed,
    cost: owner.activity.cost,
    directory: owner.directory,
    imageDirectory: owner.imageDirectory,
    actor: owner.actor,
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
      const { demo, claude, ...options } = data.args[0] as EngineOptions;
      host = hostModels(claude);
      const given = { ...options, models: host.models, roster: options.roster ?? host.roster };
      world = demo ? await createDemoWorld(given) : new World(given);
      life = world.open();
      snapshots = new Snapshots(life, world);
      const tail = world.records.entries.at(-1)?.[1];
      const event = tail && queueEvent(tail);
      if (tail && event?.step === "begin") life.send("queue", tail[1], ["aborted", event.key]);
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
        life.send("queue", entry.chain, [
          "begin",
          entry.id,
          queueHash(entry.chain, entry.shape, entry.text, entry.actor),
        ]);
        const act = life.prompt(entry.shape, entry.text, { on: entry.chain, to: entry.actor });
        life.send("queue", entry.chain, ["sent", entry.id, act.id]);
        value = act.id;
      }
    } else if (data.target === "library" && data.method === "source") {
      value = engineSource();
    } else if (data.target === "library" && data.method === "changes") {
      value = world?.changes.read(data.args[0], data.args[1]);
    } else if (data.target === "library" && data.method === "snapshot") {
      if (!snapshots) throw new Error("The session is not open.");
      value = snapshots.take(String(data.args[0]));
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
      snapshots = undefined;
      host?.dispose();
      host = undefined;
      setTimeout(() => {
        self.postMessage({ id: data.id, value, disposed });
        self.close();
      }, 0);
    } else self.postMessage({ id: data.id, value, disposed });
  } catch (error) {
    self.postMessage({ id: data.id, error: error instanceof Error ? error.message : String(error) });
  }
};
