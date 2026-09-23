import type { Life } from "@furb/engine";
import { Act, engineSource, modelNamed, World } from "@furb/engine";
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
    changes: owner.changes.length,
  };
  sentFacts = owner.facts.length;
  self.postMessage({ state: snapshot });
};
/** What a request of the session comes to. */
async function answer(data: { target: string; method: string; args: unknown[] }): Promise<unknown> {
  if (data.method === "open") {
    const { demo, claude, ...options } = data.args[0] as EngineOptions;
    host = hostModels(claude);
    const given = { ...options, models: host.models, roster: options.roster ?? host.roster };
    world = demo ? createDemoWorld(given) : new World(given);
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
    return life.root;
  }
  if (data.target === "library" && data.method === "queue") {
    if (!life || !world) throw new Error("The session is not open.");
    const entry = data.args[0] as FollowUp;
    const prior = queueDispatches(world.records.entries).get(entry.id);
    if (prior) return prior;
    life.send("queue", entry.chain, [
      "begin",
      entry.id,
      queueHash(entry.chain, entry.shape, entry.text, entry.actor),
    ]);
    const act = life.prompt(entry.shape, entry.text, { on: entry.chain, to: entry.actor });
    life.send("queue", entry.chain, ["sent", entry.id, act.id]);
    return act.id;
  }
  if (data.target === "library" && data.method === "model") {
    const owner = world;
    if (!owner) throw new Error("The session is not open.");
    // A roster name is found by the rule the World finds every model by.
    const offered = owner.roster.flatMap((name) => {
      const model = owner.offers(name);
      return model ? [{ name, provider: model.provider, id: model.id }] : [];
    });
    return modelNamed(offered, String(data.args[0]))?.name ?? null;
  }
  if (data.target === "library" && data.method === "source") return engineSource();
  if (data.target === "library" && data.method === "changes")
    return world?.changes.read(Number(data.args[0]), Number(data.args[1]));
  if (data.target === "library" && data.method === "snapshot") {
    if (!snapshots) throw new Error("The session is not open.");
    return snapshots.take(String(data.args[0]), Number(data.args[1]));
  }
  const target = data.target === "world" ? world : life;
  if (!target) throw new Error("The session is not open.");
  const method = Reflect.get(target, data.method) as (...args: unknown[]) => unknown;
  if (typeof method !== "function") throw new Error(`Unknown ${data.target} method ${data.method}.`);
  const result = Reflect.apply(method, target, data.args);
  return result instanceof Act ? result.id : await result;
}

self.onmessage = async ({ data }) => {
  let reply: { value?: unknown; error?: string };
  try {
    reply = { value: await answer(data) };
  } catch (error) {
    reply = { error: error instanceof Error ? error.message : String(error) };
  }
  if (data.method === "open" && !reply.error) state();
  if (data.target !== "world" || data.method !== "dispose") {
    self.postMessage({ id: data.id, ...reply });
    return;
  }
  // A World whose save failed has ended its life, its commands and its lease all the same, so the worker ends too.
  if (timer) clearTimeout(timer);
  world = undefined;
  life = undefined;
  snapshots = undefined;
  host?.dispose();
  host = undefined;
  setTimeout(() => {
    self.postMessage({ id: data.id, ...reply, disposed: true });
    self.close();
  }, 0);
};
