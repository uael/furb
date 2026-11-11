/** The words of a life that the operator watches, which say each thing in common words: the watch is the first
 * minute of furb, before any word of its own means anything. */

/** What the bar of the watch says: what it is, what it costs, what it shows, and the keys that move in time. */
export function watchBar(
  cost: string,
  earlier: boolean,
): [title: string, count: string, shows: string, keys: string] {
  return [
    "Watching a life",
    `0 model calls, ${cost}`,
    "A short Python program asks three models at once.",
    earlier
      ? "An earlier moment. ⌥B branches here, ⌃→ steps on, Esc comes back."
      : "Press ⌃← to go back in time, then ⌥B to start a branch from there.",
  ];
}

/** Where a branch starts: from a moment of the chain that it branched from, with the files as they were then. */
export function branchStart(source: string): string {
  return `This branch starts from a moment of ${source}, with the files as they were.`;
}

/** What a branch takes from the chain that it branched from: the messages before the moment it starts at, with what
 * answered them. */
export function branchTakes(messages: number): string {
  return messages === 0
    ? "It takes no message from before."
    : messages === 1
      ? "It takes the message before it, with its answer."
      : `It takes the ${messages} messages before it, with their answers.`;
}

/** What the watch says once a branch starts at a moment. */
export const branchNotice = "This branch starts from this moment, with the files as they were.";

/** The name of the view of the exact text that the models read, which the watch names in common words. */
export const modelsRead = "What models read";
