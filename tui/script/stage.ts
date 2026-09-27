import { mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { type CliRenderer, CodeRenderable, type Renderable } from "@opentui/core";
import type { createTestRenderer } from "@opentui/core/testing";
import { type App, follow } from "../src/app.ts";
import { demoLibrary } from "../src/demo.ts";
import type { Extensions } from "../src/extensions.ts";
import type { Session } from "../src/session.ts";
import type { Workspaces } from "../src/workspaces.ts";

/** The home of a script of the gallery, so that the paths it shows read as the paths of a user do: ~/fieldnotes. The
 * runtime reads the home directory once, so the script runs again as a child that starts in that home, with more of
 * the environment, and the script ends when the child ends. The home has the same path at each run, so that the
 * transcript, which shows the exact paths, reads the same. */
export async function home(name: string, environment: Record<string, string> = {}): Promise<string> {
  const given = process.env.FURB_GALLERY_HOME;
  if (given) return given;
  const gallery = join(tmpdir(), name);
  await rm(gallery, { recursive: true, force: true });
  await mkdir(gallery, { recursive: true });
  try {
    const child = Bun.spawn([process.execPath, Bun.main], {
      env: {
        ...process.env,
        ...environment,
        HOME: gallery,
        USERPROFILE: gallery,
        FURB_GALLERY_HOME: gallery,
      },
      stdio: ["inherit", "inherit", "inherit"],
    });
    process.exitCode = await child.exited;
  } finally {
    await rm(gallery, { recursive: true, force: true });
  }
  process.exit();
}

/** A session in a library of its own that shows the sidebar, and an App on the session that the library selects. */
export async function mount(
  session: Session,
  renderer: CliRenderer,
  extensions?: Extensions,
): Promise<{ library: Workspaces; app: () => App }> {
  const library = await demoLibrary(session);
  library.preferences.sidebar = true;
  library.preferences.save();
  return { library, app: follow(renderer, { quit() {}, workspaces: library, extensions }) };
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
