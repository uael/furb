/** How a child of the test script ended: the code that it exited with, and whether SIGKILL ended it, as the watcher of
 * memory (script/memwatch.sh) ends the largest test process while the memory is tight, and as the kernel ends a process
 * when the memory runs out. The run of such a child still fails. */
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

/** The line that names a child that SIGKILL ended, after the output of its failure. */
export const endedByMemory = (name: string): string =>
  `${name} was ended by SIGKILL, as the watcher of memory or the kernel ends a process when the memory runs low: run it alone again.`;
