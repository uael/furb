import { spawn, spawnSync } from "node:child_process";
import type { Spawned } from "./extension.js";

/** The POSIX shell that runs a command of the World: `/bin/sh`, and on Windows, which has none of its own, the `sh`
 * on PATH, such as the one of Git for Windows. */
export const shell = process.platform === "win32" ? "sh" : "/bin/sh";

/** The longest delay that one timer holds, in milliseconds. */
const LONGEST = 2 ** 31 - 1;

/** Calls the action at a time, through timers of LONGEST at most, and never for a time that is not finite; gives the
 * cancel. */
export function at(time: number, action: () => void): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const wait = () => {
    timer = setTimeout(step, Math.min(Math.max(time - Date.now(), 0), LONGEST));
  };
  const step = () => (Date.now() < time ? wait() : action());
  if (Number.isFinite(time)) wait();
  return () => clearTimeout(timer);
}

/** The `spawn` of a `WorldContext`. A command ends with every process it started: on Unix its process group, which it
 * leads, and on Windows, which has no process group that a program can signal, its tree of processes. */
export function spawnShell(
  command: string,
  options: { cwd: string; merged: boolean; fed: boolean; timeout: number | null },
): Spawned {
  const child = spawn(shell, ["-c", options.merged ? `exec 2>&1\n${command}` : command], {
    cwd: options.cwd,
    stdio: "pipe",
    detached: process.platform !== "win32",
    windowsHide: true,
  });
  let late = false;
  const kill = () => {
    if (!child.pid) return;
    try {
      if (process.platform === "win32")
        spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"], {
          stdio: "ignore",
          windowsHide: true,
        });
      else process.kill(-child.pid, "SIGKILL");
    } catch {}
  };
  const cancel = at(
    options.timeout === null ? Number.POSITIVE_INFINITY : Date.now() + options.timeout * 1000,
    () => {
      late = true;
      kill();
    },
  );
  child.on("close", cancel);
  child.on("error", cancel);
  if (!options.fed) child.stdin.end();
  child.stdin.on("error", () => {});
  return {
    child,
    stop: () => {
      cancel();
      kill();
    },
    late: () => late,
  };
}
