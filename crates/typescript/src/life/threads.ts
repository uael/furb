import type { Act } from "./acts.js";
import type { State } from "./states.js";

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

/** A thread of a list with the threads that its ladder made, its agents, each nested the same way. state is the state
 * that the thread shows: its own, or asks while a question waits in an agent under it. */
export interface ThreadNode {
  thread: ChainThread;
  agents: ThreadNode[];
  state: State;
}
