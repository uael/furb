import { afterAll, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { removeDemoDirectories } from "@furb/tui/src/demo.ts";
import { projectOf } from "../host/host.ts";
import { serve } from "../host/serve.ts";
import type { Part } from "../src/wire.ts";
import { until, windowOf, withHost, withLife } from "./hosting.ts";

afterAll(removeDemoDirectories);

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
      expect((heard[0] as { part: Part }).part.whole).toBe(true);
      expect(heard.find((one) => (one as { id?: number }).id === 7)).toEqual({
        id: 7,
        error: "No life is open.",
      });
      socket.close();
    } finally {
      served.stop();
    }
  }));
test("a compaction reaches the window with the mark that the part of the extension gives it, as the TUI shows it", () =>
  withLife(async (host, window) => {
    const session = host.workspaces.current?.session;
    if (!session) throw new Error("No session is open.");
    const word = readFileSync(
      resolve(import.meta.dir, "..", "..", "extensions", "compaction", "compaction.py"),
      "utf8",
    ).replace(/^from furb\.engine import \([^)]*\)\n+/m, "");
    await session.command(`/run ${word}`);
    await session.command('/run before = "the first step"');
    await session.command('/run compact(summary="It read the plan.\\nThe tests wait.")');
    await until("the mark", () => Boolean(window.held().life?.marks.length));
    const life = window.held().life;
    expect(life?.compactions.map((one) => one.chain)).toEqual([life?.root ?? ""]);
    expect(life?.marks).toEqual([
      {
        chain: life?.root ?? "",
        at: life?.compactions[0]?.at ?? 0,
        title: "Compaction: the model reads on from here",
        lines: [[{ text: "It read the plan.", tone: "muted" }], [{ text: "The tests wait.", tone: "muted" }]],
      },
    ]);
  }));
