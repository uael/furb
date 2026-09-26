import { EventEmitter } from "node:events";
import type { Act, Fact, ImageAttachment, LiveAct, Engine as Native, Session as Owner } from "@furb/engine";
import type { FileChange } from "@furb/engine/session";
import type { EngineOptions } from "./models.ts";
import type { ActRow, FollowUp } from "./session.ts";

export interface Snapshot {
  dispatched: string[];
  paused: boolean;
  roster: [string, string[], number][];
  /** The acts that changed after the count the session asked with. */
  acts: ActRow[];
  /** The count of changes of the act table that the acts are read at. */
  count: number;
  selected: string;
  turns: ReturnType<Native["turns"]>;
  program: Record<string, string>;
  actor: string;
  directory: string;
}

type Returned<T> = T extends Act ? string : Awaited<T>;
/** Async calls are a choice of the TUI. The library's methods remain synchronous. */
export type Engine = {
  [K in keyof Native]: Native[K] extends (...args: infer A) => infer R
    ? (...args: A) => Promise<Returned<R>>
    : Native[K];
};
/** What the session in the worker holds, as the host reads it. */
export interface HostState {
  completed: number;
  cost: number;
  directory: string;
  imageDirectory: string;
  actor: string;
  effort: Owner["provider"]["effort"];
  roster: string[];
  record?: string;
  facts: Fact[];
  prompts: { id: string; shape: string; message: string }[];
  streams: [string, { chain: string; text: string; thinking: string }][];
  pending: [string, string][];
  /** How many file changes the session holds. */
  changes: number;
}

/** The session in the worker, as the host sees it: what it holds, and the requests the host makes of it. */
export class HostView extends EventEmitter {
  completed = 0;
  cost = 0;
  directory = "";
  imageDirectory = "";
  actor = "";
  effort: Owner["provider"]["effort"] = "low";
  roster: string[] = [];
  /** The path of the record, when the session keeps one. */
  record?: string;
  facts: Fact[] = [];
  prompts = new Map<string, { id: string; shape: string; message: string }>();
  streams = new Map<string, { chain: string; text: string; thinking: string }>();
  pending = new Map<string, string>();
  /** How many file changes the session holds. */
  changes = 0;
  constructor(private request: (target: string, method: string, args: unknown[]) => Promise<unknown>) {
    super();
  }
  update(state: HostState): void {
    this.completed = state.completed;
    this.cost = state.cost;
    this.directory = state.directory;
    this.imageDirectory = state.imageDirectory;
    this.actor = state.actor;
    this.effort = state.effort;
    this.roster = state.roster;
    this.record = state.record;
    const incoming = state.facts;
    for (const fact of incoming) this.facts.push(fact);
    this.prompts = new Map(state.prompts.map((prompt) => [prompt.id, prompt]));
    this.streams = new Map(state.streams);
    this.pending = new Map(state.pending);
    this.changes = state.changes;
    if (incoming.length) this.emit("facts", incoming);
    this.emit("change");
  }
  route(actor: string): Promise<ReturnType<Owner["provider"]["route"]>> {
    return this.request("provider", "route", [actor]) as Promise<ReturnType<Owner["provider"]["route"]>>;
  }
  /** The name in the roster of the model that a name gives, and nothing when it gives none. */
  model(name: string): Promise<string | null> {
    return this.request("library", "model", [name]) as Promise<string | null>;
  }
  answer(id: string, value: string): Promise<unknown> {
    return this.request("console", "answer", [id, value]);
  }
  attachImage(path: string): Promise<ImageAttachment> {
    return this.request("session", "attachImage", [path]) as Promise<ImageAttachment>;
  }
  sendQueued(entry: FollowUp): Promise<string> {
    return this.request("library", "queue", [entry]) as Promise<string>;
  }
  resume(): Promise<unknown> {
    return this.request("session", "resume", []);
  }
  source(): Promise<string> {
    return this.request("library", "source", []) as Promise<string>;
  }
  snapshot(chain: string, since = 0): Promise<Snapshot> {
    return this.request("library", "snapshot", [chain, since]) as Promise<Snapshot>;
  }
  readChanges(start: number, count: number): Promise<FileChange[]> {
    return this.request("library", "changes", [start, count]) as Promise<FileChange[]>;
  }
  /** An act whole, with all that a command printed, and nothing when the life holds no such act. */
  act(id: string): Promise<LiveAct | undefined> {
    return this.request("library", "act", [id]) as Promise<LiveAct | undefined>;
  }
  /** The text the World reads at a path, from where a chain stands, which makes no act and keeps nothing. */
  look(path: string, chain: string): Promise<{ path: string; content: string }> {
    return this.request("library", "look", [path, chain]) as Promise<{ path: string; content: string }>;
  }
  async dispose(): Promise<void> {
    await this.request("session", "dispose", []);
  }
}

export async function openEngine(options: EngineOptions): Promise<{ engine: Engine; host: HostView }> {
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
  const host = new HostView(request);
  worker.onmessage = ({ data }) => {
    if (data.fault) {
      host.emit("fault", new Error(data.fault));
      return;
    }
    if (data.state) {
      host.update(data.state as HostState);
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
    host.emit("fault", new Error(event.message));
  };
  let root: string;
  try {
    root = (await request("session", "open", [options])) as string;
  } catch (error) {
    // No caller holds a worker whose life did not open, so it ends here.
    closed = true;
    worker.terminate();
    throw error;
  }
  const engine = new Proxy(
    { root },
    {
      get(target, key) {
        if (key === "root") return target.root;
        if (key === "disposed") return closed;
        if (key === "then") return undefined;
        return (...args: unknown[]) => request("engine", String(key), args);
      },
    },
  ) as Engine;
  return { engine, host };
}
