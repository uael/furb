import { afterAll, expect, test } from "bun:test";
import { mkdtemp, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { demoDirectory, removeDemoDirectories } from "@furb/tui/src/demo.ts";
import { Preferences } from "@furb/tui/src/preferences.ts";
import { Workspaces } from "@furb/tui/src/workspaces.ts";
import { Host, projectOf } from "../host/host.ts";
import { serve } from "../host/serve.ts";
import { empty, type Held, merged } from "../src/held.ts";
import type { Part, Said } from "../src/wire.ts";

afterAll(removeDemoDirectories);

/** A condition that comes true within a while, or a failure that says what did not come. */
async function until(what: string, condition: () => boolean, most = 20000): Promise<void> {
  const end = Date.now() + most;
  while (!condition()) {
    if (Date.now() > end) throw new Error(`${what} did not come`);
    await Bun.sleep(20);
  }
}

/** A window of the test: each part that the host sent it, and the life as the window holds it after them. */
function windowOf(host: Host): { parts: Part[]; held(): Held; leave(): void } {
  const parts: Part[] = [];
  let held = empty;
  const leave = host.connect((said: Said) => {
    parts.push(said.part);
    held = merged(held, said.part);
  });
  return { parts, held: () => held, leave };
}

/** A host on the demo, which asks no model, in a project of its own with a list of its own, which a test uses and
 * which ends after it. */
async function withHost(use: (host: Host, directory: string) => Promise<void>): Promise<void> {
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

test("a window takes the whole life when it connects, then what a model writes as it writes it, and its answer", () =>
  withHost(async (host, directory) => {
    const window = windowOf(host);
    expect(window.parts[0]?.whole).toBe(true);
    expect(window.parts[0]?.life).toBeNull();
    await host.create(directory);
    await until("the life", () => Boolean(window.held().life && !window.held().life?.loading));
    const life = window.held().life;
    expect(life?.directory).toBe(directory);
    expect(life?.record.startsWith(join(directory, ".furb", "sessions"))).toBe(true);
    expect(window.held().projects.map((project) => project.directory)).toEqual([directory]);
    await host.call("send", [life?.root, "Read this project and show live progress."]);
    const prompt = () =>
      [...window.held().acts.values()].find((act) => act.kind === "prompt" && act.by === "operator");
    await until("the stream of the rung", () => window.held().streams.size > 0);
    const rung = [...window.held().streams.keys()][0] ?? "";
    await until("the answer", () => prompt()?.done === true);
    // The model wrote its word in pieces, each of which the host sent after the place that the window held.
    const pieces = window.parts.flatMap((part) => part.streams).filter((piece) => piece.id === rung);
    expect(pieces.length).toBeGreaterThan(1);
    expect(pieces.slice(1).every((piece, at) => piece.at >= (pieces[at]?.at ?? 0))).toBe(true);
    await until("the end of the stream", () => !window.held().streams.has(rung));
    expect(window.held().acts.get(rung)?.word).toContain("close(");
    expect(window.held().acts.get(rung)?.word).toBe(
      pieces.reduce((text, piece) => text.slice(0, piece.at) + piece.text, ""),
    );
    expect(prompt()?.value).toContain("Fieldnotes keeps ideas close.");
    window.leave();
  }));

test("a window that connects later takes the whole life at once, and a part sends only what changed", () =>
  withHost(async (host, directory) => {
    const first = windowOf(host);
    await host.create(directory);
    await until("the life", () => Boolean(first.held().life && !first.held().life?.loading));
    await host.call("send", [first.held().life?.root, "Say what this project does."]);
    await until("the answer", () =>
      [...first.held().acts.values()].some((act) => act.kind === "prompt" && act.done),
    );
    const later = windowOf(host);
    expect(later.parts).toHaveLength(1);
    expect(later.parts[0]?.whole).toBe(true);
    expect([...later.held().acts.keys()].sort()).toEqual([...first.held().acts.keys()].sort());
    expect(later.held().life?.record).toBe(first.held().life?.record);
    // No act is sent twice when it did not change between two parts.
    const sent = first.parts.filter((part) => !part.whole).flatMap((part) => part.acts);
    for (let at = 1; at < sent.length; at++)
      expect(sent.slice(0, at).findLast((act) => act.id === sent[at]?.id)).not.toEqual(sent[at]);
    first.leave();
    later.leave();
  }));

test("a record goes on as a life in its project", () =>
  withHost(async (host, directory) => {
    const window = windowOf(host);
    await host.create(directory);
    await until("the life", () => Boolean(window.held().life && !window.held().life?.loading));
    const record = window.held().life?.record ?? "";
    await host.call("send", [window.held().life?.root, "Say what this project does."]);
    await until("the answer", () =>
      [...window.held().acts.values()].some((act) => act.kind === "prompt" && act.done),
    );
    expect(projectOf(record)).toBe(directory);
    await host.create(directory);
    await until("a new life", () => Boolean(window.held().life && window.held().life?.record !== record));
    await host.open(record);
    await until(
      "the record again",
      () => window.held().life?.record === record && !window.held().life?.loading,
    );
    expect([...window.held().acts.values()].some((act) => act.kind === "prompt" && act.done)).toBe(true);
    expect(window.held().life?.reading).toBeUndefined();
    window.leave();
  }));

test("the socket lets in only a window that holds its token, and answers each call by its number", () =>
  withHost(async (host) => {
    const served = serve(host);
    try {
      const bare = served.url.replace(/\?token=.*/, "").replace("ws:", "http:");
      expect((await fetch(bare)).status).toBe(403);
      const socket = new WebSocket(served.url);
      const heard: unknown[] = [];
      socket.onmessage = (event) => heard.push(JSON.parse(String(event.data)));
      await new Promise((done) => {
        socket.onopen = done;
      });
      socket.send(JSON.stringify({ id: 7, name: "nothing", args: [] }));
      await until("the answer", () => heard.some((one) => (one as { id?: number }).id === 7));
      expect((heard[0] as Said).part.whole).toBe(true);
      expect(heard.find((one) => (one as { id?: number }).id === 7)).toEqual({
        id: 7,
        error: "No life is open.",
      });
      socket.close();
    } finally {
      served.stop();
    }
  }));
