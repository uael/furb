import { rm } from "node:fs/promises";

/** A shell command that prints the number the system gives to the shell that runs it: `$$` on Unix, and on Windows
 * the number of its Windows process, which the POSIX shell there keeps in /proc apart from `$$`. */
export const printPid = "{ cat /proc/$$/winpid 2>/dev/null || echo $$; }";

/** Whether a process of this number still runs. */
export function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/** Remove a directory where processes ran that were just ended. Windows removes neither the file of a program that
 * runs nor the working directory of a process, and a process ends a moment after it is killed, so the removal is
 * tried again for a time before its failure stands. */
export async function remove(directory: string): Promise<void> {
  for (const deadline = Date.now() + 5000; ; await Bun.sleep(50))
    try {
      return await rm(directory, { recursive: true, force: true });
    } catch (error) {
      if (Date.now() > deadline) throw error;
    }
}
