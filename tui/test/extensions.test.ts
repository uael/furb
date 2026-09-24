import { expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { resolveExtensions } from "@furb/engine";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { Parts } from "../src/parts.ts";
import { Session } from "../src/session.ts";
import { idle } from "./idle.ts";

/** A project whose config names the extensions it plays, and a demo session on them, which asks no model. */
async function project(extensions: Record<string, unknown>, run: (session: Session) => Promise<void>) {
  const cwd = await mkdtemp(join(tmpdir(), "furb-tui-extension-"));
  try {
    await mkdir(join(cwd, ".furb"));
    await writeFile(join(cwd, ".furb/config.json"), JSON.stringify({ extensions }));
    await writeFile(join(cwd, "README.md"), "# A project\n");
    await mkdir(join(cwd, "probe"));
    await writeFile(
      join(cwd, "probe/package.json"),
      JSON.stringify({ name: "probe", furb: { name: "probe", python: "probe.py", tui: "tui.ts" } }),
    );
    await writeFile(
      join(cwd, "probe/probe.py"),
      'from furb.engine import Act, act, ending, idle\n\n\ndef note(on: str = "") -> Act[None]:\n  return act("note", on, ending(idle))\n',
    );
    await writeFile(
      join(cwd, "probe/tui.ts"),
      [
        "export default function probe() {",
        "  return {",
        "    commands: {",
        "      hello: {",
        '        label: "Say hello",',
        '        detail: "Bind a greeting on the chain",',
        "        async run(argument, context) {",
        '          await context.life.result(String(await context.call("rung", ["greeting = " + JSON.stringify(argument)])));',
        '          context.notify("Hello done.");',
        "        },",
        "      },",
        "    },",
        '    prefixes: { "?": "hello" },',
        "    acts: { note: { hidden: true } },",
        "  };",
        "}",
        "",
      ].join("\n"),
    );
    const { life, world } = await openEngine({ demo: true, cwd, extensions: resolveExtensions(cwd) });
    const session = new Session(life, world, true);
    try {
      await session.refresh();
      await run(session);
    } finally {
      await session.dispose();
    }
  } finally {
    await rm(cwd, { recursive: true, force: true });
  }
}

test("a command of a part stands in the palette and the suggestions, and a prefix of a part says it", async () => {
  await project({ probe: "../probe" }, async (session) => {
    const screen = await createTestRenderer({ width: 140, height: 42 });
    const app = new App(screen.renderer, session, { quit() {} });
    try {
      app.palette();
      await screen.mockInput.typeText("Say hello");
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("Say hello");
      app.closeOverlay();
      await screen.mockInput.typeText("/hel");
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("/hello");
      app.composer.setText("");
      await session.submit("?world");
      expect(await session.life.held("modules", [session.selected, "greeting"], "at")).toBe("world");
      expect(session.notice).toBe("Hello done.");
    } finally {
      app.dispose();
      screen.renderer.destroy();
    }
  });
});

test("an act of a kind that its part hides is no card, and a start that the World refuses shows why", async () => {
  await project({ probe: "../probe" }, async (session) => {
    const screen = await createTestRenderer({ width: 140, height: 42 });
    const app = new App(screen.renderer, session, { quit() {} });
    try {
      await session.submit("/run hidden = note()\njob = act('job', '', started(ending(idle)))");
      await idle(session);
      await until(session, () => session.acts.some((act) => act.kind === "job" && act.done));
      app.render();
      await screen.flush();
      const frame = screen.captureCharFrame();
      expect(session.acts.some((act) => act.kind === "note")).toBe(true);
      expect(frame).toContain("the World does no job");
      expect(frame).not.toMatch(/note\d/);
    } finally {
      app.dispose();
      screen.renderer.destroy();
    }
  });
});

test("with bash off in the config of the project, ! is text of a message, /bash is no command, and the World runs no command", async () => {
  await project({ bash: false }, async (session) => {
    expect([...session.world.parts.commands.keys()]).toEqual(["read", "cd", "grant", "context"]);
    expect(session.world.extensions.map((one) => one.name)).toEqual(["files", "grant"]);
    await expect(session.submit("/bash ls")).rejects.toThrow("Unknown command /bash");
    await session.submit("!ls");
    await idle(session);
    expect(session.acts.filter((act) => session.isUserPrompt(act)).map((act) => act.words[1])).toEqual([
      "!ls",
    ]);
    // The engine that runs keeps bash, so a word may still make a command, which the World refuses.
    for (const act of session.acts.filter((one) => one.kind === "bash"))
      expect(act.value).toEqual({ is: "Refused", args: ["the World does no bash"] });
  });
});

test("two parts that claim one command, prefix or kind are refused with both names", () => {
  const part = { commands: { same: { label: "Same", detail: "", run() {} } } };
  expect(
    () =>
      new Parts([
        { name: "one", part },
        { name: "two", part },
      ]),
  ).toThrow("The extensions one and two both give the command /same.");
  expect(
    () => new Parts([{ name: "one", part: { commands: { run: { label: "", detail: "", run() {} } } } }]),
  ).toThrow("The extension one gives the command /run, which the TUI has.");
});
