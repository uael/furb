// The record as the core of the app hands it to the window: what the app shows, and for an open record its acts, as
// the model of a life reads them, with the words of the rungs and the measures of the chains. A browser shows the
// fixture of a record; the window of the app hears the record that the core reads.

import fixture from "@fixture";
import type { LiveAct, Measure } from "@furb/engine/life";

/** The record as the core sends it: the picker, a record that opens, or an open record and what changed in it. */
export type Wire =
  | { state: "picker"; generation: number; said: string[] }
  | { state: "opening"; generation: number; said: string[]; what: string }
  | {
      state: "open";
      generation: number;
      /** Whether the part holds the whole record that the core read, or what changed after the last part. */
      whole: boolean;
      /** The last lines that the app said, in a part that is whole or when they changed. */
      said?: string[];
      /** The count of changes that the acts held before this part, and after it. */
      from: number;
      count: number;
      /** How many facts the record told, which is its last moment. */
      len: number;
      root: string;
      place: string;
      directory: string;
      /** Whether the facts are those of the log beside the record, which holds every fact that the life said. Else
       * they are what the record kept, so a thread that a word closed has no end that the record shows. */
      logged: boolean;
      /** The engine that made the record, as the record names it. */
      made: string | null;
      /** Whether the core read every fact of the record. */
      read: boolean;
      acts: LiveAct[];
      words: Record<string, string>;
      /** The measures, in a part that is whole or when they changed. */
      measures?: Record<string, Measure>;
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

/** The fixture of a browser: a record as the core sends it whole, and the places to pick. */
export const demo = fixture as unknown as Extract<Wire, { state: "open" }> & { places?: Places };

/** The fixture that a browser shows: the one built in, or the one that it fetched at ?fixture=, as for a large
 * record. */
export const shown: { fixture: typeof demo } = { fixture: demo };

/** Whether the UI runs in the window of the app, which hears a record, or in a browser, which shows the fixture. */
export const tauri = "__TAURI_INTERNALS__" in window;

/** A call of the core, in the window of the app; nothing in a browser. */
export async function call<T>(name: string, args?: Record<string, unknown>): Promise<T | undefined> {
  if (!tauri) return undefined;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(name, args);
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
