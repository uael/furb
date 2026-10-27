import { afterEach, expect, test } from "bun:test";
import { appendFileSync, writeFileSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { RGBA } from "@opentui/core";
import { createTestRenderer } from "@opentui/core/testing";
import type { Turn } from "furb";
import { until } from "../../bind/typescript/test/until.ts";
import { follow } from "../src/app.ts";
import { demoLibrary, demoSession } from "../src/demo.ts";
import { Reader } from "../src/reading.ts";
import {
  colorless,
  glyph,
  motion,
  own,
  paletteOf,
  palettes,
  setTheme,
  spin,
  terminalPalette,
  theme,
  themeOf,
} from "../src/theme.ts";
import { mix, styled } from "../src/ui.ts";
import { composing, drawn } from "./composing.ts";

afterEach(() => {
  colorless.on = false;
  motion.still = false;
  setTheme("ink");
});

/** The glyphs of a border, of the bar of a panel, and of the spinner, which the mode of a screen reader draws none of. */
const borders = /[┃╻╹│─┌┐└┘╭╮╰╯▄▀⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏]/g;
/** The glyphs of the frame that the mode of a screen reader draws none of. */
const glyphs = (frame: string) => [...frame.matchAll(borders)].map(([found]) => found).join("");

test("the mode of a screen reader draws one column of text, with no box, no spinner, and no sidebar", async () => {
  const session = await demoSession({ seed: true });
  session.preferences.screenReader = true;
  session.preferences.view = "loom";
  await composing(
    async ({ app, screen, frame }) => {
      await session.engine.result(
        await session.engine.rung({ word: "await wait(0.2)", on: session.engine.root }),
      );
      const shown = await drawn(frame, screen.renderer.root);
      expect(shown).toContain("Explore this project");
      expect(glyphs(shown)).toBe("");
      expect(app.lifeView).toBe("main");
      expect(Reflect.get(app, "rail").visible).toBe(false);
      expect(motion.still).toBe(true);
    },
    { width: 120, height: 44 },
    session,
  );
});

test("reduced motion stops the spinner, which shows the mark of work that runs", () => {
  expect(spin(0)).not.toBe(spin(100));
  motion.still = true;
  expect(spin(0)).toBe(glyph.running);
  expect(spin(100)).toBe(glyph.running);
});

test("/access turns reduced motion and the mode of a screen reader on and off, and a new view follows the mode", async () => {
  const session = await demoSession();
  const library = await demoLibrary(session);
  const screen = await createTestRenderer({ width: 120, height: 40, exitSignals: [] });
  const app = follow(screen.renderer, { quit() {}, workspaces: library });
  try {
    const first = app();
    first.composer.setText("/access motion");
    first.submit();
    await Bun.sleep(50);
    expect(session.preferences.reducedMotion).toBe(true);
    expect(motion.still).toBe(true);
    expect(app()).toBe(first);
    first.composer.setText("/access reader");
    first.submit();
    await Bun.sleep(50);
    expect(session.preferences.screenReader).toBe(true);
    expect(app()).not.toBe(first);
    app().render();
    await screen.flush();
    expect(glyphs(screen.captureCharFrame())).toBe("");
  } finally {
    app().dispose();
    screen.renderer.destroy();
    await library.dispose();
  }
});

test("NO_COLOR gives each role the default color of the terminal, and a mix of two of them gives the nearer one", () => {
  colorless.on = true;
  setTheme("paper");
  expect(theme.text.intent).toBe("default");
  expect(theme.accent.intent).toBe("default");
  expect(theme.background.intent).toBe("default");
  expect(theme.text).not.toBe(theme.muted);
  expect(mix(theme.text, theme.background, 0.2)).toBe(theme.text);
  expect(mix(theme.text, theme.background, 0.8)).toBe(theme.background);
  colorless.on = false;
  setTheme("paper");
  expect(theme.text.intent).toBe("rgb");
});

test("the theme Terminal follows the mode of the terminal, dark or light, and shows Ink while it says none", () =>
  composing(async ({ session, app, screen, frame }) => {
    expect(themeOf("terminal", null)).toBe("ink");
    expect(themeOf("terminal", "light")).toBe("day");
    expect(themeOf("paper", "dark")).toBe("paper");
    expect(session.preferences.theme).toBe("terminal");
    expect(session.theme).toBe("ink");
    screen.renderer.emit("theme_mode", "light");
    await frame();
    expect(session.theme).toBe("day");
    expect(theme.background.equals(RGBA.fromHex(palettes.day.background))).toBe(true);
    expect(app.renderer).toBe(screen.renderer);
    session.theme = "midnight";
    screen.renderer.emit("theme_mode", "dark");
    await frame();
    expect(session.theme).toBe("midnight");
  }));

test("an address of the web in a text is a link, which ends before the mark that ends its sentence", () => {
  const color = RGBA.fromHex("#ffffff");
  const { chunks } = styled([["Read https://github.com/uael/furb/pull/1. Then go on.", color], [" plain"]]);
  expect(chunks.map((chunk) => [chunk.text, chunk.link?.url])).toEqual([
    ["Read ", undefined],
    ["https://github.com/uael/furb/pull/1", "https://github.com/uael/furb/pull/1"],
    [". Then go on.", undefined],
    [" plain", undefined],
  ]);
  expect(chunks[1]?.fg).toBe(color);
});

test("a question that comes while the terminal has no focus calls the operator once, by a notification or the bell", async () => {
  let rings = 0;
  const notified: string[] = [];
  const session = await demoSession();
  const library = await demoLibrary(session);
  const screen = await createTestRenderer({ width: 120, height: 40, exitSignals: [] });
  let shows = true;
  screen.renderer.triggerNotification = (message: string) => {
    notified.push(message);
    return shows;
  };
  const app = follow(screen.renderer, {
    quit() {},
    workspaces: library,
    bell() {
      rings++;
    },
  });
  const ask = async (message: string) => {
    await session.engine.prompt("str", { message, to: "operator", on: session.engine.root });
    await until(session, () => session.desk.asks.some((one) => one.message === message));
    app().render();
  };
  try {
    await ask("First, while the terminal has the focus?");
    screen.renderer.emit("blur");
    await ask("Which name?");
    expect(notified).toEqual(["Main asks: Which name?"]);
    expect(rings).toBe(0);
    app().render();
    expect(notified).toHaveLength(1);
    shows = false;
    await ask("Which color?");
    expect(rings).toBe(1);
    screen.renderer.emit("focus");
    await ask("Which size?");
    expect(rings).toBe(1);
    expect(notified).toHaveLength(2);
  } finally {
    app().dispose();
    screen.renderer.destroy();
    await library.dispose();
  }
});

test("a recorded life hears each whole line that its file adds, and a line that is not whole waits", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-follow-"));
  try {
    const path = join(directory, "life.jsonl");
    writeFileSync(
      path,
      `${JSON.stringify({ recorded: true })}\n${JSON.stringify(["chain", "chain1", "operator", "", "root", ""])}\n`,
    );
    const reader = Reader.recorded(path, directory);
    expect(reader.facts).toHaveLength(1);
    expect(reader.follow()).toBe(false);
    appendFileSync(
      path,
      `${JSON.stringify(["chain", "chain2", "operator", "chain1", "two", ""])}\n["chain", "cha`,
    );
    expect(reader.follow()).toBe(true);
    expect(reader.facts.map((fact) => fact[1])).toEqual(["chain1", "chain2"]);
    appendFileSync(path, `in3", "operator", "chain1", "three", ""]\n`);
    expect(reader.follow()).toBe(true);
    expect(reader.facts.map((fact) => fact[1])).toEqual(["chain1", "chain2", "chain3"]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a reader holds of a long output the tail that its row shows, and gives the act whole from its file", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-cut-"));
  try {
    const path = join(directory, "life.jsonl");
    const output = Array.from({ length: 500 }, (_, at) => `line ${at} of a long output`).join("\n");
    const exit = {
      is: "Exit",
      code: 0,
      stdout: { is: "Text", content: output },
      stderr: { is: "Text", content: "" },
    };
    const facts = [
      ["chain", "chain1", "operator", "", "root", ""],
      ["bash", "bash1", "operator", "chain1", "seq 500", "", 600],
      ["started", "bash1", "bash"],
      ["done", "bash1", "bash", exit],
    ];
    writeFileSync(path, [{ recorded: true }, ...facts].map((one) => `${JSON.stringify(one)}\n`).join(""));
    const reader = Reader.recorded(path, directory);
    const held = reader.activity.acts.get("bash1") as unknown as { value: typeof exit; output: number };
    expect(held.value.stdout.content).toBe(output.slice(-2000));
    expect(held.output).toBe(output.length);
    expect(JSON.stringify(reader.facts).length).toBeLessThan(output.length);
    expect(reader.act("bash1")?.value).toEqual(exit);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("the turns of a chain that a reader gives again keep each turn that did not change", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-turns-"));
  try {
    const path = join(directory, "life.jsonl");
    const turn: Turn = ["assistant", "close(1)", [10, 1, 0, 0, 0], null];
    const facts = [
      ["chain", "chain1", "operator", "", "root", ""],
      ["tell", "chain1", "chain1", ["#chain1"]],
      ["reply", "reply1", "rung1", "chain1", "opus"],
      ["done", "reply1", "provider", turn],
    ];
    writeFileSync(path, [{ recorded: true }, ...facts].map((one) => `${JSON.stringify(one)}\n`).join(""));
    const reader = Reader.recorded(path, directory);
    const first = reader.turns();
    expect(first).toEqual([["user", "#chain1", null, null], turn, ["user", "", null, null]]);
    appendFileSync(path, `${JSON.stringify(["tell", "chain1", "chain1", ["#rung2 advance"]])}\n`);
    reader.follow();
    const again = reader.turns();
    expect(again[1]).toBe(first[1] as Turn);
    expect(again.at(-1)).toEqual(["user", "#rung2 advance", null, null]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("the theme Terminal shows the colors that the terminal says, its text a step softer than its foreground", () => {
  const dark = terminalPalette("#ffffff", "#000000");
  expect(dark?.background).toBe("#000000");
  expect(dark?.text).toBe("#d6d6d6");
  expect(dark?.accent).toBe(palettes.ink.accent);
  expect(terminalPalette("#202020", "#fafafa")?.accent).toBe(palettes.day.accent);
  expect(terminalPalette(null, "#000000")).toBeUndefined();
  own.palette = dark;
  try {
    expect(themeOf("terminal", "light")).toBe("own");
    expect(themeOf("paper", null)).toBe("paper");
    expect(paletteOf("own")).toBe(dark as NonNullable<typeof dark>);
  } finally {
    own.palette = undefined;
  }
});
