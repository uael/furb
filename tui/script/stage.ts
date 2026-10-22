import { mkdir, readFile, realpath, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type CliRenderer, CodeRenderable, type Renderable } from "@opentui/core";
import type { createTestRenderer } from "@opentui/core/testing";
import { type App, follow } from "../src/app.ts";
import { demoLibrary } from "../src/demo.ts";
import type { Session } from "../src/session.ts";
import type { Workspaces } from "../src/workspaces.ts";

/** The longest time, in seconds, that a script of the gallery waits for one screen, that a run of it lasts, and that it
 * waits for another run to end. */
const STEP = 180,
  LIMIT = 1800,
  WAIT = 1800;

/** Whether a process runs. A process of another user that runs refuses the signal. */
function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return (error as NodeJS.ErrnoException).code === "EPERM";
  }
}

/** The lock of the home of a script, which one run holds at a time: a folder that holds the process of the run, made
 * by a rename, which is atomic. A lock whose process ended is taken. Another run waits at most WAIT seconds. */
async function hold(lock: string): Promise<void> {
  const draft = `${lock}.${process.pid}`;
  await rm(draft, { recursive: true, force: true });
  await mkdir(draft);
  await writeFile(join(draft, "pid"), String(process.pid));
  const deadline = Date.now() + WAIT * 1000;
  let told = false;
  for (;;) {
    try {
      await rename(draft, lock);
      return;
    } catch (error) {
      if (!["EEXIST", "ENOTEMPTY", "EPERM", "EACCES"].includes((error as NodeJS.ErrnoException).code ?? ""))
        throw error;
    }
    const owner = Number(await readFile(join(lock, "pid"), "utf8").catch(() => "0"));
    if (owner && !alive(owner)) {
      const stale = `${lock}.stale.${process.pid}`;
      await rename(lock, stale).catch(() => {});
      await rm(stale, { recursive: true, force: true });
      continue;
    }
    if (Date.now() > deadline) {
      await rm(draft, { recursive: true, force: true });
      throw new Error(`The run of process ${owner} held ${lock} for ${WAIT} seconds, so this run ends.`);
    }
    if (!told) console.error(`The run of process ${owner} holds ${lock}, so this run waits until it ends.`);
    told = true;
    await new Promise((done) => setTimeout(done, 1000));
  }
}

/** The home of a script of the gallery, so that the paths it shows read as the paths of a user do: ~/fieldnotes. The
 * runtime reads the home directory once, so the script runs again as a child that starts in that home, with more of
 * the environment, and the script ends when the child ends. The home has the same path at each run, so that the
 * transcript, which shows the exact paths, reads the same. On a system with /tmp it stands there, since the temporary
 * directory of a user differs on each machine of macOS. Its path holds no link, as the workspaces hold each folder by
 * such a path, and /tmp of macOS is a link to /private/tmp.
 * One run at a time holds the home: two runs in it would open the same records, and each would remove the files of
 * the other, so a second run waits for the first. The child ends when its parent ends, and the parent stops the child
 * when the parent is stopped or when the child runs past LIMIT seconds. */
export async function home(name: string, environment: Record<string, string> = {}): Promise<string> {
  const given = process.env.FURB_GALLERY_HOME;
  if (given) {
    const parent = Number(process.env.FURB_GALLERY_PARENT);
    if (parent)
      setInterval(() => {
        if (alive(parent)) return;
        console.error(`The run of process ${parent} that started this script ended, so the script ends too.`);
        process.exit(1);
      }, 1000).unref();
    return given;
  }
  const gallery = join(process.platform === "win32" ? tmpdir() : "/tmp", name);
  const lock = `${gallery}.lock`;
  await hold(lock);
  let child: ReturnType<typeof Bun.spawn> | undefined;
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const)
    process.on(signal, () => child?.kill(signal));
  try {
    await rm(gallery, { recursive: true, force: true });
    await mkdir(gallery, { recursive: true });
    const real = await realpath(gallery);
    child = Bun.spawn([process.execPath, Bun.main], {
      env: {
        ...process.env,
        ...environment,
        HOME: real,
        USERPROFILE: real,
        FURB_GALLERY_HOME: real,
        FURB_GALLERY_PARENT: String(process.pid),
      },
      stdio: ["inherit", "inherit", "inherit"],
    });
    const limit = setTimeout(() => {
      console.error(`${Bun.main} ran for ${LIMIT} seconds, so it is stopped.`);
      child?.kill("SIGKILL");
    }, LIMIT * 1000);
    const code = await child.exited;
    clearTimeout(limit);
    process.exitCode = code || (child.signalCode ? 1 : 0);
  } finally {
    await rm(gallery, { recursive: true, force: true });
    await rm(lock, { recursive: true, force: true });
  }
  process.exit();
}

/** The watch of a script of the gallery, told what it waits for at each step: a step that waits for more than STEP
 * seconds ends the script with an error that names what it waited for, as a screen whose state never comes. */
export function watch(): (what: string) => void {
  let waiting = "the first screen";
  const timer = setTimeout(() => {
    console.error(`The script waited ${STEP} seconds for ${waiting}, so it ends.`);
    process.exit(1);
  }, STEP * 1000);
  return (what) => {
    waiting = what;
    timer.refresh();
  };
}

/** The end of a script of the gallery: each part of its end runs, and the process exits, with an error when the script
 * or a part of its end failed. A renderer, a session or a command that stays would keep the process alive, and a
 * part that fails would skip the parts after it, so the script never waits for the process to end on its own. */
export async function end(failed: boolean, ...parts: (() => unknown)[]): Promise<never> {
  let code = failed ? 1 : 0;
  for (const part of parts)
    try {
      await part();
    } catch (error) {
      console.error(error);
      code = 1;
    }
  process.exit(code);
}

/** A session in a library of its own that shows the sidebar, and an App on the session that the library selects. */
export async function mount(
  session: Session,
  renderer: CliRenderer,
): Promise<{ library: Workspaces; app: () => App }> {
  const library = await demoLibrary(session);
  library.preferences.sidebar = true;
  library.preferences.save();
  return { library, app: follow(renderer, { quit() {}, workspaces: library }) };
}

/** The highlights of each block of code under a node, which a parser colors off the main thread. */
export function highlighting(node: Renderable): Promise<void>[] {
  return [
    ...(node instanceof CodeRenderable ? [node.highlightingDone] : []),
    ...node.getChildren().flatMap(highlighting),
  ];
}

/** The column and the row of the first place of the screen that shows a text, at or after a column. */
export function find(
  screen: Awaited<ReturnType<typeof createTestRenderer>>,
  text: string,
  from = 0,
): [number, number] {
  const rows = screen.captureCharFrame().split("\n");
  const row = rows.findIndex((line) => line.indexOf(text, from) >= 0);
  if (row < 0) throw new Error(`The screen shows no ${text}.`);
  return [(rows[row] ?? "").indexOf(text, from), row];
}

/** A click on the first place of the screen that shows a text, at or after a column. */
export async function click(
  screen: Awaited<ReturnType<typeof createTestRenderer>>,
  text: string,
  from = 0,
): Promise<void> {
  const [column, row] = find(screen, text, from);
  await screen.mockMouse.click(column + 1, row);
}
