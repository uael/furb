import {
  type ActState,
  actState,
  type LiveAct,
  type Measure,
  outline,
  owedAt,
  parentOf,
  rowState,
  type Thread,
  threads,
} from "@furb/engine/life";
import { useEffect, useMemo, useState } from "react";
import { call, demo, type State, shown, tauri, type Wire } from "./data.ts";

/** The record as the window holds it: the acts that the core sent, merged, and what came with the last part. */
export interface Held {
  state: "picker" | "opening" | "open";
  generation: number;
  what: string;
  said: string[];
  count: number;
  len: number;
  root: string;
  place: string;
  directory: string;
  logged: boolean;
  made: string | null;
  read: boolean;
  acts: Map<string, LiveAct>;
  words: Map<string, string>;
  measures: Record<string, Measure>;
}

const empty: Held = {
  state: "picker",
  generation: 0,
  what: "",
  said: [],
  count: 0,
  len: 0,
  root: "",
  place: "",
  directory: "",
  logged: false,
  made: null,
  read: false,
  acts: new Map(),
  words: new Map(),
  measures: {},
};

/** A part that the core sent, taken into what the window holds: a part of another life starts again from nothing. */
function merged(held: Held, part: Wire): Held {
  const base = part.generation === held.generation ? held : empty;
  if (part.state !== "open")
    return {
      ...empty,
      state: part.state,
      generation: part.generation,
      said: part.said,
      what: part.state === "opening" ? part.what : "",
    };
  const acts = part.acts.length ? new Map(base.acts) : base.acts;
  for (const act of part.acts) acts.set(act.id, act);
  const given = Object.entries(part.words);
  const words = given.length ? new Map(base.words) : base.words;
  for (const [id, word] of given) words.set(id, word);
  return {
    state: "open",
    generation: part.generation,
    what: "",
    said: part.said ?? base.said,
    count: Math.max(base.count, part.count),
    len: part.len,
    root: part.root,
    place: part.place,
    directory: part.directory,
    logged: part.logged,
    made: part.made,
    read: part.read,
    acts,
    words,
    measures: part.measures ?? base.measures,
  };
}

/** A fixture that a browser fetches in place of the one built in, as a record too large to build into the UI. */
const fetched = new URLSearchParams(window.location.search).get("fixture");

/** The record that the window shows: the one that the core of the app sends, or the fixture in a browser.
 * A part that comes after a gap makes the window ask the core again for all that changed since what it holds. */
export function useHeld(): Held {
  const [held, setHeld] = useState<Held>(() => (tauri || fetched ? empty : merged(empty, demo)));
  useEffect(() => {
    if (!tauri && fetched)
      void fetch(fetched)
        .then((answer) => answer.json())
        .then((fixture: typeof demo) => {
          shown.fixture = fixture;
          setHeld(merged(empty, fixture));
        });
    if (!tauri) return;
    let gone = false;
    let stop: (() => void) | undefined;
    let mine = empty;
    const take = (part: Wire | undefined) => {
      if (!part || gone) return;
      mine = merged(mine, part);
      setHeld(mine);
    };
    const catchUp = () => void call<Wire>("life", { since: mine.generation ? mine.count : 0 }).then(take);
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      // The window listens first, then asks for the life, so no part that the core sends between the two is lost.
      stop = await listen<Wire>("life", (event) => {
        const part = event.payload;
        const gap = part.state === "open" && (part.generation !== mine.generation || part.from > mine.count);
        if (gap && part.generation !== mine.generation) {
          mine = { ...empty, generation: part.generation };
          void call<Wire>("life", { since: 0 }).then(take);
        } else if (gap) catchUp();
        else take(part);
      });
      await call<Wire>("life", { since: 0 }).then(take);
    })();
    return () => {
      gone = true;
      stop?.();
    };
  }, []);
  return held;
}

/** The record as the views read it: its acts in the order of their place in time, its threads, and what the
 * operator owed when the record ends. */
export interface Moment {
  acts: LiveAct[];
  rows: Map<string, LiveAct>;
  owed: Set<string>;
  tree: Thread[];
  threads: Thread[];
  chains: LiveAct[];
  word: (rung: LiveAct) => string;
}

/** The views of the record that the window holds. */
export function useMoment(held: Held): Moment {
  return useMemo(() => {
    const acts = [...held.acts.values()].sort((one, other) => one.began - other.began);
    const owed = new Set(owedAt(acts).map((one) => one.id));
    const tree = threads(acts, owed);
    const rows = new Map(acts.map((act) => [act.id, act]));
    const word = (rung: LiveAct) => String(rung.words[0] || held.words.get(rung.id) || "");
    return {
      acts,
      rows,
      owed,
      tree,
      threads: outline(tree, new Set()),
      chains: acts.filter((act) => act.kind === "chain"),
      word,
    };
  }, [held.acts, held.words]);
}

/** The state of an act, as a glyph shows it. */
export function glyphOf(state: ActState): State {
  return (
    {
      running: "runs",
      stands: "rest",
      done: "done",
      failed: "failed",
      cancelled: "rest",
      held: "paused",
      asks: "asks",
    } as const
  )[state];
}

/** The state of a thread: the state of its prompt, and for one that runs, waits when its ladder waits on another
 * program, or asks when a thread under it asks the operator. */
export function threadState(moment: Moment, thread: Thread): State {
  const state = glyphOf(thread.state);
  if (state !== "runs") return state;
  const rung = moment.acts.findLast((act) => act.kind === "rung" && act.by === thread.act.id && !act.done);
  const waits = rung && moment.acts.some((act) => act.by === rung.id && !act.done && act.kind === "prompt");
  return waits ? "waits" : "runs";
}

/** The state of a chain, as the loom reads it, as a glyph shows it. */
export function chainState(moment: Moment, chain: string): State {
  const work = moment.acts.filter((act) => act.on === chain && act.kind !== "chain");
  const owes = work.some((act) => moment.owed.has(act.id));
  const state = rowState(work, owes, new Map());
  return (
    { asks: "asks", failed: "failed", working: "runs", paused: "paused", done: "done", idle: "rest" } as const
  )[state];
}

/** The name of a chain as a person reads it: its label, or its id, and You for the operator. */
export function chainName(moment: Moment, id: string): string {
  if (id === "operator") return "You";
  const chain = moment.rows.get(id);
  return String(chain?.words[0] ?? "") || id;
}

/** The chain whose rung made a chain, and nothing for the root. */
export function hirer(moment: Moment, chain: LiveAct): string | undefined {
  return parentOf(chain, moment.rows);
}

export { actState };
