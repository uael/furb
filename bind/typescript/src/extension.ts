import type { ChildProcessWithoutNullStreams } from "node:child_process";
import type { LiveAct } from "./activity.js";
import type { FileChange } from "./changes.js";
import type { Fact } from "./types.js";

/** A question that an ear asks the life while it hears: a verb, by its name, with its words. */
export interface Call {
  verb: string;
  args?: unknown[];
  kwargs?: Record<string, unknown>;
}
/** What an ear says to the life: the kind of a fact, the act it is about, and its words. */
export type Saying = [kind: string, about: string, ...words: unknown[]];
/** What a part of the World does as it hears one fact: each saying and each call it yields, which the life answers
 * before the part goes on. A saying is given back as the fact the life made of it, and a call as what it gave. */
export type Hearing<T = void> = Generator<Saying | Call, T, unknown>;
/** An exception as it crosses: the name of its class and what it was made with. */
export interface Fault {
  is: string;
  args: unknown[];
}

/** What a part of the World adds to the table of acts: the value of an act of its kinds at birth, and the act it
 * changed as it hears a fact. The host in TypeScript alone has this, since the table is the TUI's. */
export interface LiveView {
  born?(act: LiveAct): unknown;
  hears?(fact: Fact, acts: ReadonlyMap<string, LiveAct>): LiveAct | undefined;
}

/** The part of an extension for a World, in the same shape in every language a host writes its World in: the kinds
 * of act it does, and what it says of each fact the World hears. The World hands it the start of an act of its
 * kinds, and closes the start of a kind that no part names with `Refused("the World does no <kind>")`. It answers a
 * question with plain data, and reaches the life with the calls it yields. */
export interface WorldPart {
  readonly kinds?: readonly string[];
  hears?(fact: Fact): Hearing | undefined;
  live?: LiveView;
  dispose?(): void | Promise<void>;
}
/** The default export of the file an extension names for a World in TypeScript. */
export type WorldExtension = (context: WorldContext) => WorldPart;

/** A command of the shell that a part started, and how it ends: `stop` ends it with every process it started, and
 * `late` says that its timeout ended it. */
export interface Spawned {
  child: ChildProcessWithoutNullStreams;
  stop(): void;
  late(): boolean;
}

/** What the World gives a part: where it stands, and how it reaches the life and the host. The code of an extension
 * reaches the package through this alone, since the cache it runs from holds no copy of the package. */
export interface WorldContext {
  /** The directory the chains of the life start in. */
  readonly directory: string;
  /** An inspection of a record: the part hears nothing and starts nothing. */
  readonly readOnly: boolean;
  /** Aborts when the World is disposed. */
  readonly signal: AbortSignal;
  /** `yield* context.where(on)`: where the paths of a chain resolve, which the `cwd` verb of the chain gives, and
   * the directory of the World when the chain binds no `cwd`. */
  where(on: string): Hearing<string>;
  /** A path of the disk: the directory of the World, where the chain stands, then the path. It throws for a path of
   * a scheme, which names no file. */
  at(here: string, path?: string): string;
  /** One fact said later, as the World, after every saying before it. */
  speak(kind: string, about: string, ...words: unknown[]): void;
  /** An act closed later with a value or a fault, as the World, after every saying before it. */
  close(value: unknown, id: string): void;
  /** One file the life changed, which the host shows. */
  change(change: FileChange): void;
  /** A command of the shell, started in a directory, with its stderr in its stdout when it is merged, its stdin open
   * when it is fed, and ended at its timeout in seconds. */
  spawn(
    command: string,
    options: { cwd: string; merged: boolean; fed: boolean; timeout: number | null },
  ): Spawned;
  /** The refusal of a question, as it crosses. */
  refused(message: string): Fault;
}

/** A value of a class that a word defined, as it crosses out of the life: its class, and its fields. */
export interface Instance {
  is: "instance";
  class: { is: "class"; id: number; name: string; base: unknown };
  value: unknown;
}

/** Whether a value is an instance of a class a word defined, and of the class of that name when a name is given. */
export function isInstance(value: unknown, name?: string): value is Instance {
  if (!value || typeof value !== "object") return false;
  const held = value as Partial<Instance>;
  return (
    held.is === "instance" &&
    Boolean(held.class && typeof held.class === "object") &&
    (name === undefined || held.class?.name === name)
  );
}

/** A value with every instance in it, however deep, made the plain data of its fields. */
export function unwrapped<T = unknown>(value: unknown): T {
  if (isInstance(value)) return unwrapped(value.value);
  if (Array.isArray(value)) return value.map((one) => unwrapped(one)) as T;
  if (!value || typeof value !== "object") return value as T;
  return Object.fromEntries(Object.entries(value).map(([key, one]) => [key, unwrapped(one)])) as T;
}

/** An instance made again of its class with other fields, as it crosses into the life. */
export function remade(
  instance: Instance,
  fields: Record<string, unknown>,
): { is: "instance"; class: number; fields: Record<string, unknown> } {
  return { is: "instance", class: instance.class.id, fields };
}
