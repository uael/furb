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
      /** Whether the part holds the whole life that the core holds, or what changed after the last part. */
      whole: boolean;
      /** The last lines that the app said, in a part that is whole or when they changed. */
      said?: string[];
      /** The count of changes that the acts held before this part, and after it. */
      from: number;
      count: number;
      /** How many facts the life said, which is its last moment. */
      len: number;
      root: string;
      place: string;
      directory: string;
      works: boolean;
      /** Whether the facts are those of a whole log: those of a life that works, or of the log beside a record. Else
       * they are what the record kept, so a thread that a word closed has no end that the record shows. */
      logged: boolean;
      /** The engine that made a record to read, as the record names it. */
      made: string | null;
      /** Whether the core read every fact of a record, which a life that works holds from its opening. */
      read: boolean;
      acts: LiveAct[];
      words: Record<string, string>;
      /** The prompts that the operator owes, the measures and the breakpoints, in a part that is whole or when they
       * changed. */
      owed?: string[];
      measures?: Record<string, Measure>;
      breaks?: Break[];
      /** What the models write now: for each rung, where a piece starts in its text and the piece, which is the end of
       * the text in a part that is whole and the new text in another part. */
      streams?: Record<string, [number, string]>;
      /** The rungs whose text ended, in a part that is not whole. */
      ended?: string[];
    };

/** The end of what a model writes for a rung, as the window holds it: where it starts in the whole text, and the
 * text from there. */
export interface Stream {
  from: number;
  text: string;
}

/** How many units of what a model writes for a rung the window holds, as many as the core sends whole. */
export const KEPT = 8000;

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

/** The fixture that a browser shows: the demo, or the one that it fetched at ?fixture=, as for a large record. */
export const shown: { fixture: typeof demo } = { fixture: demo };

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
