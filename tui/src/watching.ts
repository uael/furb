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
      ? "An earlier moment. b branches here, ] steps on, Esc comes back."
      : "Press [ to go back in time, then b to start a branch from there.",
  ];
}

/** The common words of what a step did, by the kind of its acts. */
const did: Record<string, [one: string, many: (count: number) => string]> = {
  read: ["read a file", (count) => `read ${count} files`],
  write: ["changed a file", (count) => `changed ${count} files`],
  bash: ["ran a command", (count) => `ran ${count} commands`],
  prompt: ["asked a model", (count) => `asked ${count} models`],
  chain: ["started a branch", (count) => `started ${count} branches`],
};

/** What acts of one kind did, in common words, as "ran a command": the kind is the name of an act less its number. */
export function plainActs(name: string, count: number): string {
  const kind = name.replace(/\d+$/, "");
  const words = did[kind];
  if (!words) return count === 1 ? kind : `${count} × ${kind}`;
  return count === 1 ? words[0] : words[1](count);
}

/** The name of a step of Python: the program of the operator, or the step that a model writes. */
export function stepName(author: string): string {
  return author === "you" ? "Your program" : `${author} writes Python`;
}

/** Who asks a model: the program, or furb, which tells the model of a chain that an act is done. */
export function asker(model: string, chain: boolean): string {
  return chain ? `furb tells ${model}` : `the program asks ${model}`;
}

/** The line that holds the earlier steps of a long chain until a click opens them. */
export function earlierSteps(count: number): string {
  return `${count} earlier ${count === 1 ? "step" : "steps"}`;
}

/** Where a branch starts: from a moment of the chain that it branched from, with the files as they were then. */
export function branchStart(source: string): string {
  return `This branch starts from a moment of ${source}, with the files as they were.`;
}

/** How many messages of the operator a branch takes from its source: each one that came before the word that made
 * the branch and that this word does not name, since the word names each act that the branch leaves out. */
export function branchMessages<T extends { id: string; on: string; began: number }>(
  acts: readonly T[],
  source: string,
  maker: { began: number },
  word: string,
  message: (act: T) => boolean,
): number {
  const left = new Set([...word.matchAll(/"([a-z_]+\d+)"/g)].map((found) => found[1]));
  return acts.filter(
    (act) => act.on === source && act.began < maker.began && !left.has(act.id) && message(act),
  ).length;
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
