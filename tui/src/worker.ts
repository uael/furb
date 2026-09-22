import type { Life } from "@furb/engine";
import { Act, engineSource, World, type WorldOptions } from "@furb/engine";
import type { WorldState } from "./bridge.ts";
import { createDemoWorld } from "./demo.ts";

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
    model: owner.model,
    effort: owner.effort,
    roster: owner.roster,
    path: owner.records.path,
    facts: owner.facts.slice(sentFacts),
    prompts: [...owner.prompts.values()].map(({ id, shape, message }) => ({ id, shape, message })),
    streams: [...owner.streams],
    held: [...owner.held],
    changes: owner.changes.slice(sentChanges),
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
      world = options.demo ? await createDemoWorld(options.record) : new World(options);
      life = world.open();
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
    } else if (data.target === "library" && data.method === "source") {
      value = engineSource();
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
      // Let native wrappers finalize while the worker's JavaScript environment is still valid.
      world = undefined;
      life = undefined;
      setTimeout(() => {
        Bun.gc(true);
        self.postMessage({ id: data.id, value, disposed });
        self.close();
      }, 0);
    } else self.postMessage({ id: data.id, value, disposed });
  } catch (error) {
    self.postMessage({ id: data.id, error: error instanceof Error ? error.message : String(error) });
  }
};
