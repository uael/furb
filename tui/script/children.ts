/** How a child of the test script ended: the code that it exited with, and whether the watcher of memory
 * (script/memwatch.sh) ended it, which it does with SIGKILL on the largest test process while the memory is tight. A
 * child ended so is no failure of its test. */
export interface Ended {
  code: number | null;
  memory: boolean;
}

/** The end of a child that exited: a child killed by SIGKILL is ended by memory. */
export async function endOf(child: { exited: Promise<number>; signalCode: string | null }): Promise<Ended> {
  const code = await child.exited;
  const memory = child.signalCode === "SIGKILL" || code === 137;
  return { code: memory ? null : code, memory };
}

/** The line that names a child that the watcher ended, in place of the output of a failure. */
export const endedByMemory = (name: string): string =>
  `${name} was ended by the watcher of memory (SIGKILL), which is no failure of its test: run it again.`;
