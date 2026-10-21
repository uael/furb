// The view of a life, as the core of the app hands it to the UI: the threads, the programs and their messages, and
// the ladder of each thread as code. A browser shows the fixture of the demo; the window of the app hears the life.

import demo from "./data/demo.json";

export type State = "asks" | "failed" | "paused" | "runs" | "waits" | "done" | "rest";

export interface ActMark {
  id: string;
  kind: string;
  state: State;
  summary: string;
}

export interface Line {
  text: string;
  here: boolean;
  acts: ActMark[];
}

export interface Step {
  id: string;
  state: State;
  said: string;
  lines: Line[];
  findings: string | null;
  raised: string | null;
  thinks: boolean;
  asks: string[];
  awaits: string | null;
}

export interface Thread {
  id: string;
  depth: number;
  state: State;
  title: string;
  shape: string;
  chain: string;
  to: string;
  from: string;
  rung: string;
  steps: { done: number; failed: number; all: number };
  answer: string | null;
  failed: boolean;
  messages: string[];
  ladder: Step[];
}

export interface Frame {
  act: string;
  chain: string;
  awaits: string | null;
}

export interface Binding {
  name: string;
  value: string;
  fresh: boolean;
}

export interface Chain {
  id: string;
  label: string;
  parent: string;
  depth: number;
  state: State;
  actor: string;
  cost: number;
  share: number;
  threads: string[];
  paused: boolean;
  stack: Frame[];
  watch: Binding[];
}

export interface Edge {
  id: string;
  from: string;
  to: string;
  rung: string | null;
  state: State;
}

export interface View {
  life: {
    place: string;
    root: string;
    at: number;
    len: number;
    spent: number;
    ceiling: number | null;
    works: boolean;
    owed: string[];
  };
  threads: Thread[];
  chains: Chain[];
  edges: Edge[];
}

export const fixture = demo as View;

/** The words a state reads as, for a tip. */
export const stateWords: Record<State, string> = {
  asks: "Waits for you",
  failed: "Failed",
  paused: "Paused",
  runs: "Runs",
  waits: "Waits on another program",
  done: "Done",
  rest: "At rest",
};

/** The name of a chain as a person reads it: its label, or its id. */
export function chainName(view: View, id: string): string {
  if (id === "operator") return "You";
  const chain = view.chains.find((one) => one.id === id);
  return chain?.label || id;
}

/** The model of an actor without its provider: claude-cli:sonnet/high reads sonnet, high. */
export function actorName(actor: string): string {
  const [model = actor, effort] = (actor.split(":").at(-1) ?? actor).split("/");
  return effort ? `${model}, ${effort}` : model;
}
