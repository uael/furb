export type { State } from "../protocol.js";

/** Whether a state is work that is not done: a model that thinks, a word or a command that runs, or a word that awaits
 * another chain. */
export const busy = (state: string): boolean =>
  state === "thinks" || state === "runs" || state === "delegates";
