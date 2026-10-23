// What the tests of the host share: a host on the demo, a window of a test on it, and a wait for what comes.

import { mkdtemp, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { demoDirectory } from "@furb/tui/src/demo.ts";
import { Preferences } from "@furb/tui/src/preferences.ts";
import { Workspaces } from "@furb/tui/src/workspaces.ts";
import { Host } from "../host/host.ts";
import { empty, type Held, merged } from "../src/held.ts";
import type { Part, Said } from "../src/wire.ts";

/** A condition that comes true within a while, or a failure that says what did not come. */
export async function until(what: string, condition: () => boolean, most = 20000): Promise<void> {
  const end = Date.now() + most;
  while (!condition()) {
    if (Date.now() > end) throw new Error(`${what} did not come`);
    await Bun.sleep(20);
  }
}

/** A window of the test: each part that the host sent it, the life as the window holds it after them, and each other
 * word of the host. */
export function windowOf(host: Host): { parts: Part[]; said: Said[]; held(): Held; leave(): void } {
  const parts: Part[] = [];
  const said: Said[] = [];
  let held = empty;
  const leave = host.connect((one: Said) => {
    if (!("part" in one)) return void said.push(one);
    parts.push(one.part);
    held = merged(held, one.part);
  });
  return { parts, said, held: () => held, leave };
}

/** A host on the demo, which asks no model, in a project of its own with a list of its own, which a test uses and
 * which ends after it. */
export async function withHost(use: (host: Host, directory: string) => Promise<void>): Promise<void> {
  // The project stands where its real path says, as the list of the TUI holds it.
  const directory = await realpath(await demoDirectory());
  const list = join(await mkdtemp(join(tmpdir(), "furb-gui-list-")), "workspaces.json");
  const host = new Host(new Workspaces(new Preferences(join(directory, "ui.json")), { demo: true }, list));
  try {
    await use(host, directory);
  } finally {
    await host.dispose();
  }
}

/** A host on the demo with a life open in it, and a window on it that holds the whole life. */
export async function withLife(
  use: (host: Host, window: ReturnType<typeof windowOf>, directory: string) => Promise<void>,
): Promise<void> {
  await withHost(async (host, directory) => {
    const window = windowOf(host);
    await host.create(directory);
    await until("the life", () => Boolean(window.held().life && !window.held().life?.loading));
    try {
      await use(host, window, directory);
    } finally {
      window.leave();
    }
  });
}
