import { EventEmitter } from "node:events";
import { type Act, actorParts, type Fact, type ImageAttachment, type Life, type World } from "@furb/engine";
import type { FileChange } from "@furb/engine/world";
import type { EngineOptions } from "./models.ts";
import type { ActRow, FollowUp } from "./session.ts";

export interface Snapshot {
  dispatched: string[];
  paused: boolean;
  roster: [string, string[], number][];
  acts: ActRow[];
  selected: string;
  turns: ReturnType<Life["turns"]>;
  rendered: string[];
  program: Record<string, string>;
  actor: string;
  directory: string;
}

type Returned<T> = T extends Act ? string : Awaited<T>;
/** Async calls are a choice of the TUI. The library's methods remain synchronous. */
export type Engine = {
  [K in keyof Life]: Life[K] extends (...args: infer A) => infer R
    ? (...args: A) => Promise<Returned<R>>
    : Life[K];
};
export interface WorldState {
  completed: number;
  cost: number;
  directory: string;
  imageDirectory: string;
  actor: string;
  effort: World["effort"];
  roster: string[];
  path?: string;
  facts: Fact[];
  prompts: { id: string; shape: string; message: string }[];
  streams: [string, { chain: string; text: string; thinking: string }][];
  held: [string, string][];
  changes: string[];
  models: ReturnType<World["route"]>[];
}

export class HostView extends EventEmitter {
  completed = 0;
  cost = 0;
  directory = "";
  imageDirectory = "";
  actor = "";
  effort: World["effort"] = "low";
  roster: string[] = [];
  records: { path?: string } = {};
  facts: Fact[] = [];
  prompts = new Map<string, { id: string; shape: string; message: string }>();
  streams = new Map<string, { chain: string; text: string; thinking: string }>();
  held = new Map<string, string>();
  changes: string[] = [];
  private models: ReturnType<World["route"]>[] = [];
  constructor(private request: (target: string, method: string, args: unknown[]) => Promise<unknown>) {
    super();
  }
  update(state: WorldState): void {
    this.completed = state.completed;
    this.cost = state.cost;
    this.directory = state.directory;
    this.imageDirectory = state.imageDirectory;
    this.actor = state.actor;
    this.effort = state.effort;
    this.roster = state.roster;
    this.records.path = state.path;
    const incoming = state.facts;
    this.facts.push(...incoming);
    this.prompts = new Map(state.prompts.map((prompt) => [prompt.id, prompt]));
    this.streams = new Map(state.streams);
    this.held = new Map(state.held);
    this.changes.push(...state.changes);
    this.models = state.models;
    if (incoming.length) this.emit("facts", incoming);
    this.emit("change");
  }
  route(actor: string): ReturnType<World["route"]> {
    const model = this.models.find((model) =>
      [actor, actorParts(actor).model].some(
        (name) => `${model.provider}:${model.id}` === name || model.id === name,
      ),
    );
    if (!model) throw new Error(`No model ${actor}.`);
    return model;
  }
  answer(id: string, value: string): Promise<unknown> {
    return this.request("world", "answer", [id, value]);
  }
  attachImage(path: string): Promise<ImageAttachment> {
    return this.request("world", "attachImage", [path]) as Promise<ImageAttachment>;
  }
  sendQueued(entry: FollowUp): Promise<string> {
    return this.request("library", "queue", [entry]) as Promise<string>;
  }
  resume(): Promise<unknown> {
    return this.request("world", "resume", []);
  }
  source(): Promise<string> {
    return this.request("library", "source", []) as Promise<string>;
  }
  snapshot(chain: string): Promise<Snapshot> {
    return this.request("library", "snapshot", [chain]) as Promise<Snapshot>;
  }
  readChanges(start: number, count: number): Promise<FileChange[]> {
    return this.request("library", "changes", [start, count]) as Promise<FileChange[]>;
  }
  async dispose(): Promise<void> {
    await this.request("world", "dispose", []);
  }
}

export async function openEngine(options: EngineOptions): Promise<{ life: Engine; world: HostView }> {
  const worker = new Worker(new URL("worker.ts", import.meta.url).href, { type: "module" });
  const waiting = new Map<number, { resolve(value: unknown): void; reject(error: Error): void }>();
  let sequence = 0;
  let closed = false;
  const request = (target: string, method: string, args: unknown[]) =>
    new Promise<unknown>((resolve, reject) => {
      if (closed) {
        reject(new Error("The session is disposed."));
        return;
      }
      const id = ++sequence;
      waiting.set(id, { resolve, reject });
      worker.postMessage({ id, target, method, args });
    });
  const world = new HostView(request);
  worker.onmessage = ({ data }) => {
    if (data.fault) {
      world.emit("fault", new Error(data.fault));
      return;
    }
    if (data.state) {
      world.update(data.state as WorldState);
      return;
    }
    const promise = waiting.get(data.id);
    waiting.delete(data.id);
    if (data.error) promise?.reject(new Error(data.error));
    else promise?.resolve(data.value);
    if (data.disposed) {
      closed = true;
      for (const pending of waiting.values()) pending.reject(new Error("The session is disposed."));
      waiting.clear();
    }
  };
  worker.onerror = (event) => {
    closed = true;
    for (const promise of waiting.values()) promise.reject(new Error(event.message));
    waiting.clear();
    world.emit("fault", new Error(event.message));
  };
  const root = (await request("world", "open", [options])) as string;
  const life = new Proxy(
    { root },
    {
      get(target, key) {
        if (key === "root") return target.root;
        if (key === "disposed") return closed;
        if (key === "then") return undefined;
        return (...args: unknown[]) => request("life", String(key), args);
      },
    },
  ) as Engine;
  return { life, world };
}
