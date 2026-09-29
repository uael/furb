import { EventEmitter } from "node:events";
import type { Act, FileChange, ImageAttachment, LiveAct, Engine as Native, Stream, Turn } from "@furb/engine";
import type { EngineOptions } from "./models.ts";
import type { ActRow, FollowUp } from "./session.ts";

/** What one answer of a model read and wrote: its whole prompt, its output, the reads and the writes of the cache,
 * and its dollars. */
export type Usage = NonNullable<Turn[2]>;

export interface Snapshot {
  dispatched: string[];
  paused: boolean;
  roster: [string, string[], number][];
  /** The acts that changed after the count the session asked with. */
  acts: ActRow[];
  /** The steps asked after the count the session asked with, each with the act that asked it. */
  asked: [string, string][];
  /** The count of changes of the act table that the acts are read at. */
  count: number;
  selected: string;
  turns: ReturnType<Native["turns"]>;
  /** The usage of each answer of the chain, in order. A chain with a source reads the answers of its origin in its
   * turns, and they are not among these. */
  answers: Usage[];
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
/** A question that waits for the operator. */
type Thread = { id: string; shape: string; markdown: string };
/** What the session in the worker holds that the host keeps as it comes. */
interface Plain {
  completed: number;
  cost: number;
  directory: string;
  imageDirectory: string;
  actor: string;
  /** The path of the record, when the session keeps one. */
  record?: string;
  /** How many file changes the session holds. */
  changes: number;
}
/** What the session in the worker holds, as it sends it to the host, and how many facts the life said since the last
 * state, which the host needs to know of and not to hold. */
export interface HostState extends Plain {
  facts: number;
  threads: Thread[];
  streams: [string, Stream][];
  pending: [string, string][];
}

/** The session in the worker, as the host sees it: what it holds, and the requests the host makes of it. */
export class HostView extends EventEmitter implements Plain {
  completed = 0;
  cost = 0;
  directory = "";
  imageDirectory = "";
  actor = "";
  record?: string;
  changes = 0;
  threads = new Map<string, Thread>();
  streams = new Map<string, Stream>();
  pending = new Map<string, string>();
  constructor(private request: (target: string, method: string, args: unknown[]) => Promise<unknown>) {
    super();
  }
  update({ facts, threads, streams, pending, ...plain }: HostState): void {
    Object.assign(this, plain);
    this.threads = new Map(threads.map((thread) => [thread.id, thread]));
    this.streams = new Map(streams);
    this.pending = new Map(pending);
    if (facts) this.emit("facts");
    this.emit("change");
  }
  /** A request of the worker: the method of one of its targets, the session, its console, or the library, which
   * answers what the host reads of the life. */
  private ask<T>(target: "session" | "console" | "library", method: string, ...args: unknown[]): Promise<T> {
    return this.request(target, method, args) as Promise<T>;
  }
  /** Whether the model of an actor takes an image, as the catalog of the crate says. */
  sees = (actor: string) => this.ask<boolean>("library", "sees", actor);
  /** The models the catalog of the crate offers, each as its name, its efforts and its window. */
  catalog = () => this.ask<[string, string[], number][]>("library", "catalog");
  /** The name of the model that a name gives in the roster or in the catalog, and nothing when it gives none. */
  model = (name: string) => this.ask<string | null>("library", "model", name);
  answer = (id: string, value: string) => this.ask<unknown>("console", "answer", id, value);
  attachImage = (path: string) => this.ask<ImageAttachment>("session", "attachImage", path);
  sendQueued = (entry: FollowUp) => this.ask<string>("library", "queue", entry);
  resume = () => this.ask<unknown>("session", "resume");
  /** The work of a chain cancelled, and not the acts that an extension started on it. */
  interrupt = (chain: string) => this.ask<unknown>("session", "interrupt", chain);
  source = () => this.ask<string>("library", "source");
  snapshot = (chain: string, since = 0) => this.ask<Snapshot>("library", "snapshot", chain, since);
  readChanges = (start: number, count: number) => this.ask<FileChange[]>("library", "changes", start, count);
  /** The changes that the writes of some acts made, as the rungs of words, in the order they were made. */
  changesOf = (acts: string[]) => this.ask<FileChange[]>("library", "changesOf", acts);
  /** An act whole, with all that a command printed, and nothing when the life holds no such act. */
  act = (id: string) => this.ask<LiveAct | undefined>("library", "act", id);
  /** The text the World reads at a path, from where a chain stands, which makes no act and keeps nothing. */
  look = (path: string, chain: string) =>
    this.ask<{ path: string; content: string }>("library", "look", path, chain);
  dispose = () => this.ask<void>("session", "dispose");
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
