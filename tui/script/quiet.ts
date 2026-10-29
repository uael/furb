/** The quiet machine of the bench: a local bench takes a lock that serializes benches across worktrees, shows the load
 * average with its measures, and refuses when the load is over the number of cores. A measure under another worker's
 * builds swings three times and proves nothing, so CI, which runs no other work, is the place of a trusted measure.
 * CI (CI=true) takes no lock and is never refused. FURB_BENCH_FORCE=1 runs a local bench under load anyway, FURB_BENCH_LOCK
 * names the lock, and FURB_BENCH_WAIT sets how many seconds a bench waits for the lock. */
import { mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism, loadavg } from "node:os";
import { join } from "node:path";

/** The lock, a folder that holds the pid of the bench that took it, the same for every worktree of the machine. */
export const LOCK = process.env.FURB_BENCH_LOCK || "/tmp/furb-bench.lock";
/** How long a folder with no pid may stand before it counts as left behind: the time between mkdir and the pid. */
const UNNAMED = 10_000;

/** Whether a process lives. */
export function alive(pid: number): boolean {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch (fault) {
    return (fault as NodeJS.ErrnoException).code === "EPERM";
  }
}

/** The pid that a lock holds: 0 when it names none yet, or -1 when the lock is gone. */
function holder(lock: string): number {
  try {
    return Number(readFileSync(join(lock, "pid"), "utf8").trim()) || 0;
  } catch {
    try {
      statSync(lock);
      return 0;
    } catch {
      return -1;
    }
  }
}

/** Takes the lock, waiting while a live bench holds it, and gives the function that frees it. A lock whose bench died
 * is taken over. It throws when the wait passes `wait` ms, and names the holder. */
export async function take(
  lock = LOCK,
  wait = 3_600_000,
  poll = 1000,
  log = (_: string) => {},
): Promise<() => void> {
  const began = Date.now();
  let told = false;
  for (;;) {
    try {
      mkdirSync(lock);
      writeFileSync(join(lock, "pid"), String(process.pid));
      let freed = false;
      const free = () => {
        if (freed || holder(lock) !== process.pid) return;
        freed = true;
        rmSync(lock, { recursive: true, force: true });
      };
      process.on("exit", free);
      for (const signal of ["SIGINT", "SIGTERM"] as const)
        process.on(signal, () => {
          free();
          process.exit(130);
        });
      return free;
    } catch (fault) {
      if ((fault as NodeJS.ErrnoException).code !== "EEXIST") throw fault;
    }
    const pid = holder(lock);
    let left = false;
    try {
      left = pid === 0 ? Date.now() - statSync(lock).mtimeMs > UNNAMED : pid > 0 && !alive(pid);
    } catch {
      left = false;
    }
    if (left && holder(lock) === pid) rmSync(lock, { recursive: true, force: true });
    if (left) continue;
    if (Date.now() - began >= wait)
      throw new Error(
        `the bench lock ${lock} stays held by the process ${pid} after ${Math.round((Date.now() - began) / 1000)} s`,
      );
    if (!told) {
      told = true;
      log(`Another bench on this machine holds ${lock} (process ${pid}), so this one waits for it.`);
    }
    await Bun.sleep(poll);
  }
}

/** The message that refuses a bench, or nothing when the machine is quiet enough. */
export function refusal(load: number, cores: number, ci: boolean, force = false): string | undefined {
  if (ci || force || load <= cores) return undefined;
  return `The bench refuses to run: the load average is ${load.toFixed(1)} on ${cores} cores, so other work, as the builds of other workers, would swing its measures. Run it when the machine is quiet, or read the measure of the bench job on CI, the place of a trusted measure. FURB_BENCH_FORCE=1 runs it anyway.`;
}

/** The load average of the last minute. */
export const load = () => loadavg()[0] ?? 0;

/** Starts a bench: takes the lock on a local machine, prints the load average, and exits 1 when the load is over the
 * number of cores. It gives the function that frees the lock. */
export async function settle(): Promise<() => void> {
  const ci = process.env.CI === "true";
  const cores = availableParallelism();
  const free = ci
    ? () => {}
    : await take(LOCK, Number(process.env.FURB_BENCH_WAIT || 3600) * 1000, 1000, console.log);
  const [now, five, fifteen] = loadavg();
  console.log(
    `Load average ${now?.toFixed(1)} ${five?.toFixed(1)} ${fifteen?.toFixed(1)} on ${cores} cores${ci ? ", on CI" : ""}.`,
  );
  const refused = refusal(load(), cores, ci, process.env.FURB_BENCH_FORCE === "1");
  if (refused) {
    console.error(refused);
    free();
    process.exit(1);
  }
  return free;
}
