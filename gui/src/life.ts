import {
  type ActState,
  actState,
  type LiveAct,
  outline,
  owedAt,
  parentOf,
  rowState,
  type Thread,
  threads,
} from "@furb/engine/life";
import { useEffect, useMemo, useState } from "react";
import { connect, type State } from "./data.ts";
import { empty, type Held, merged } from "./held.ts";
import type { Said } from "./wire.ts";

export type { Held, Streamed } from "./held.ts";

/** The life that the window shows, as the host sends it. */
export function useHeld(): Held {
  const [held, setHeld] = useState<Held>(empty);
  useEffect(() => {
    let mine = empty;
    return connect(
      (said) => {
        if (!("part" in said)) {
          window.dispatchEvent(new CustomEvent<Said>("furb", { detail: said }));
          return;
        }
        mine = merged(mine, said.part);
        setHeld(mine);
      },
      (connected) => {
        mine = { ...mine, connected };
        setHeld(mine);
      },
    );
  }, []);
  return held;
}

/** A word of the host that is no part, as a cancel to confirm or a message taken back, heard while a view stands. */
export function useSaid(hear: (said: Said) => void): void {
  useEffect(() => {
    const heard = (event: Event) => hear((event as CustomEvent<Said>).detail);
    window.addEventListener("furb", heard);
    return () => window.removeEventListener("furb", heard);
  }, [hear]);
}

/** The life as the views read it: its acts in the order of their place in time, its threads, and what waits for
 * the operator. */
export interface Moment {
  acts: LiveAct[];
  rows: Map<string, LiveAct>;
  owed: Set<string>;
  tree: Thread[];
  threads: Thread[];
  chains: LiveAct[];
  word: (rung: LiveAct) => string;
}

/** How often the views of a life are derived again while the host still reads it. */
const PACE = 1000;

/** The count of changes that the views of a life stand on: each count once the host read the whole life, and while
 * it reads, the count at most once each PACE milliseconds, so a large record is not derived again at each part. */
function useSettled(held: Held): number {
  const loading = held.life?.loading ?? false;
  const [settled, setSettled] = useState(held.count);
  useEffect(() => {
    if (!loading) {
      setSettled(held.count);
      return;
    }
    const timer = setTimeout(() => setSettled(held.count), PACE);
    return () => clearTimeout(timer);
  }, [held.count, loading]);
  return loading ? settled : held.count;
}

/** The views of the life that the window holds. */
export function useMoment(held: Held): Moment {
  const count = useSettled(held);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the acts grow in place, and the count says when.
  return useMemo(() => {
    const acts = [...held.acts.values()].sort((one, other) => one.began - other.began);
    const owed = new Set(owedAt(acts).map((one) => one.id));
    const tree = threads(acts, owed);
    const rows = new Map(acts.map((act) => [act.id, act]));
    const word = (rung: LiveAct) => rung.word ?? String(rung.words[0] ?? "");
    return {
      acts,
      rows,
      owed,
      tree,
      threads: outline(tree, new Set()),
      chains: acts.filter((act) => act.kind === "chain"),
      word,
    };
  }, [held.acts, held.generation, count]);
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
