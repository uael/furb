import type { ChildProcessWithoutNullStreams } from "node:child_process";
import type { Act, Life } from "../index.cjs";
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
  /** The config directory of the user, which the config of the extensions stands in. */
  readonly config: string;
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

/** A life seen from another thread: each method gives a promise, and an act gives its name. */
export type Remote<T> = {
  [K in keyof T]: T[K] extends (...args: infer A) => infer R
    ? (...args: A) => Promise<R extends Act ? string : Awaited<R>>
    : T[K];
};

/** What the TUI gives a part: the life, the chain on screen and where its paths resolve, and the ways to reach the
 * session. The code of an extension reaches the TUI through this alone. */
export interface TuiContext {
  life: Remote<Life>;
  /** The chain on screen. */
  chain: string;
  /** The directory that the paths of the chain on screen resolve against. */
  directory: string;
  /** The acts of the session, as the views read them, with the value of each as its plain data. */
  acts: readonly LiveAct[];
  /** A verb said on the chain on screen, unless the keywords name another chain. */
  call(verb: string, args?: unknown[], kwargs?: Record<string, unknown>): Promise<unknown>;
  /** A path that the operator typed, with `~` as the home, against the directory of the chain. */
  path(typed: string): string;
  /** The files of the project, once the TUI has read them. */
  projectFiles(): string[] | undefined;
  notify(message: string): void;
  submit(message: string): Promise<void>;
  /** An act that the session follows until it is done, and shows the failure of. */
  track(id: string): void;
  show(view: "feed" | "transcript" | "changes"): void;
}
/** A value that the argument of a command takes, which the suggestions offer. A value with more after it waits for
 * the rest, and any other value runs the command when it is chosen. */
export interface TuiValue {
  value: string;
  detail: string;
  more?: boolean;
}
/** A slash command of a part: its label, its argument (in angle brackets when it is needed, in square brackets when
 * it may be left out), what it does, the key that says it, whether its value is a path of the project, the values
 * its argument takes, and what it does when it runs. */
export interface TuiCommand {
  label: string;
  argument?: string;
  detail: string;
  keys?: string;
  paths?: boolean;
  values?(context: TuiContext): TuiValue[];
  run(argument: string, context: TuiContext): void | Promise<void>;
}
/** How the TUI shows an act of a kind of a part. By default an act shows its kind as its title and its first word as
 * its subject, and it is at work while it lives, no pause holds it, and the World was started on it. */
export interface ActView {
  subject?(act: LiveAct): string;
  /** What the card of the act shows under its heading while it is folded, and whether that is the tail. */
  preview?(act: LiveAct): { text: string; tail?: boolean } | undefined;
  /** What the card shows when it opens: streams of text, each with its name when it has one, then notes under them,
   * each with its label. */
  details?(act: LiveAct):
    | {
        streams?: { name: string; content: string; failure?: boolean }[];
        notes?: { label?: string; text: string; tone?: "faint" | "success" | "danger" }[];
      }
    | undefined;
  /** A pause does not hold its work, as a command runs on while its chain is paused. */
  runsPaused?: boolean;
  /** It stands over its chain: a dot while it lives, and "ended" once it is done. */
  standing?: boolean;
  /** It is no card, no point to rewind to, and no state of its chain. */
  hidden?: boolean;
  /** The header words that tell how it ended, which its card shows as its state. */
  ends?: string[];
  /** The names of its words, which its details show. */
  fields?: string[];
}
/** What a part adds to the sidebar for the chain on screen: rows under the usage, and a mark on the meter of the
 * context with the words of its tip. */
export interface SidebarPart {
  rows?: { name: string; value: string; tone?: "muted" | "text" }[];
  meter?: { mark: number; tip: string };
}
/** The part of an extension for the TUI: its commands, the prefixes of the input that say a command, how it shows the
 * acts of its kinds, the header words of the notes that no card shows, the header words whose detail is a path, what
 * it adds to the sidebar, and what it does before a message of the operator is sent. */
export interface TuiPart {
  commands?: Record<string, TuiCommand>;
  prefixes?: Record<string, string>;
  acts?: Record<string, ActView>;
  quiet?: string[];
  paths?: string[];
  sidebar?(view: { acts: readonly LiveAct[]; chain: string }): SidebarPart | undefined;
  prompting?(message: string, context: TuiContext): Promise<void>;
  dispose?(): void | Promise<void>;
}
/** The default export of the file an extension names for the TUI. */
export type TuiExtension = () => TuiPart | Promise<TuiPart>;

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
