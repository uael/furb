// The life as the core of the app hands it to the window: what the app shows, and for an open life its acts, as the
// model of a life reads them, with the words of the rungs, the prompts that the operator owes, the measures of the
// chains, and what the models write now. A browser shows the fixture of the demo; the window of the app hears the life.

import fixture from "@fixture";
import type { LiveAct, Measure } from "@furb/engine/life";

/** What the operator sets to pause a chain: at each step of a chain, at each act of a kind, or past a cost. */
export type Break =
  | { kind: "chain"; value: string }
  | { kind: "kind"; value: string }
  | { kind: "cost"; value: number };

/** A command of the operator, as the core takes it. */
export type Command =
  | { kind: "pause" | "wake" | "step" | "cancel"; value: string }
  | { kind: "answer"; value: { id: string; shape: string; line: string } }
  | { kind: "door"; value: { prompt: string; on: string; program: string } }
  | { kind: "prompt"; value: { on: string; shape: string; message: string } }
  | { kind: "word"; value: { on: string; word: string } }
  | { kind: "breaks"; value: Break[] };

/** What the operator picks to open. */
export type Pick =
  | { kind: "project"; value: { directory: string; record: string | null } }
  | { kind: "record"; value: string };

/** One binding that a chain told, as the watch reads it. */
export interface Told {
  name: string;
  value: string;
  at: number;
  act?: string | null;
}

/** The life as the core sends it: the picker, a life that opens, or an open life and what changed in it. */
export type Wire =
  | { state: "picker"; generation: number; said: string[] }
  | { state: "opening"; generation: number; said: string[]; what: string }
  | {
      state: "open";
      generation: number;
      said: string[];
      /** The count of changes that the acts held before this part, and after it. */
      from: number;
      count: number;
      /** How many facts the life said, which is its last moment. */
      len: number;
      root: string;
      place: string;
      directory: string;
      works: boolean;
      acts: LiveAct[];
      words: Record<string, string>;
      owed: string[];
      measures: Record<string, Measure>;
      streams: Record<string, { text: string; thinking: string }>;
      breaks: Break[];
    };

/** One record of a project, as the picker lists it. */
export interface Record_ {
  path: string;
  name: string;
  title: string;
  changed: number;
  size: number;
}

/** The projects that the picker lists, each with its records. */
export interface Places {
  projects: { directory: string; name: string; records: Record_[] }[];
}

/** The fixture of a browser: the demo as the core sends it whole, the watch of each chain, and the places to pick. */
export const demo = fixture as unknown as Extract<Wire, { state: "open" }> & {
  watch: Record<string, Told[]>;
  places?: Places;
};

/** Whether the UI runs in the window of the app, which hears a life, or in a browser, which shows the fixture. */
export const tauri = "__TAURI_INTERNALS__" in window;

/** A call of the core, in the window of the app; nothing in a browser. */
export async function call<T>(name: string, args?: Record<string, unknown>): Promise<T | undefined> {
  if (!tauri) return undefined;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(name, args);
}

/** A command of the operator, sent to the life. */
export function send(command: Command): void {
  void call("send", { command }).catch(() => undefined);
}

/** The words a state reads as, for a tip. */
export type State = "asks" | "failed" | "paused" | "runs" | "waits" | "done" | "rest";
export const stateWords: Record<State, string> = {
  asks: "Waits for you",
  failed: "Failed",
  paused: "Paused",
  runs: "Runs",
  waits: "Waits on another program",
  done: "Done",
  rest: "At rest",
};

/** The model of an actor without its provider: claude-cli:sonnet/high reads sonnet, high. */
export function actorName(actor: string): string {
  const [model = actor, effort] = (actor.split(":").at(-1) ?? actor).split("/");
  return effort ? `${model}, ${effort}` : model;
}

/** Dollars as a person reads them. */
export function dollars(value: number): string {
  return `$${value.toFixed(2)}`;
}
