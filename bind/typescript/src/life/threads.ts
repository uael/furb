import type { Act } from "./acts.js";
import type { State } from "./states.js";

/** The acts of a life by their names, as a host indexes them. */
export type Rows = Pick<ReadonlyMap<string, Act>, "get">;

/** What a prompt of a chain is that is no conversation, which the feed of its chain draws as what it is and the list
 * of its threads leaves out: a notice of the round, a prompt of None that a rung or a chain made, which no one answers;
 * what a word of the operator told the chain, a prompt of None that a rung of his made or the acknowledgment of a rung
 * in no thread; a wake, a prompt of None that an ear or a verb of the round made; or a question of another chain to
 * this one, a worker that asks its coordinator, with the chain that asks. The view model of the host gives it, in
 * `src/view/threads.rs`. */
export type Aside =
  | { aside: "notice" }
  | { aside: "rung" }
  | { aside: "wake" }
  | { aside: "question"; from: string };

/** A prompt of a chain that is no thread, with what it is and the prompts on it, in order. */
export interface ChainAside {
  act: Act;
  aside: Aside;
  messages: Act[];
}

/** One thread of a chain, as the list of the threads of that chain shows it: its prompt, the first line of its message,
 * its messages, the questions and the notices that it made for the operator, the questions that wait in it, and its
 * state. A thread is over when its prompt is over. A message of None on an open thread is over at once, so it
 * holds nothing open, and a prompt of str on a thread that is over opens it again until its model closes that prompt. */
export interface ChainThread {
  act: Act;
  title: string;
  messages: Act[];
  /** The prompts to the operator that the thread made, answered or not, in the order they began. */
  asked: Act[];
  /** The ids of the questions that wait for the operator. */
  asks: string[];
  state: State;
  over: boolean;
  /** Who made its prompt: the operator, a rung of a word, or an ear. */
  origin: "operator" | "rung" | "ear";
  /** Whether the operator started it, directly or by a word of their own. */
  yours: boolean;
}

/** The acts whose changes are the work of a thread: its prompt, its messages and its prompts to the operator. */
export function threadWork(thread: Pick<ChainThread, "act" | "messages" | "asked">): Act[] {
  return [thread.act, ...thread.messages, ...thread.asked];
}

/** How a thread concerns the operator: a question waits for them in it, or they started it and it failed or is held. */
export type Concern = "asks" | "failed" | "paused";

/** A run of notes: the asides of a chain that are no question and follow one another in its feed with no card of
 * another kind between them, which a window draws as one block. The view model of the host says the same in
 * `chain_notes` of `src/view/threads.rs`. */
export interface NoteRun {
  prompts: Act[];
  count: number;
}

/** A thread of a list with the threads that its ladder made, its agents, each nested the same way. state is the state
 * that the thread shows: its own, or asks while a question waits in an agent under it. */
export interface ThreadNode {
  thread: ChainThread;
  agents: ThreadNode[];
  state: State;
}
