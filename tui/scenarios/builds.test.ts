// The rule of the windows and the hosts: several windows serve one life at once, and no window ends a host of any
// build. A window of another build attaches to the host that runs the life, and a click on a workspace opens a new
// session in it, never one that stands.
import { expect } from "bun:test";
import { copyFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { aboutOf, hostPlace } from "../src/hosting.ts";
import { alive, test, until, Window, Yard } from "./harness.ts";

const SCRIPT = [
  { cue: "Scenario: one.", word: 'close("One done.")' },
  { cue: "Scenario: two.", word: 'close("Two done.")' },
];

test("a window of another build on a live session leaves its host alive, and reads the life and goes on with it", async () => {
  const yard = await Yard.make(SCRIPT);
  let old: Window | undefined;
  let now: Window | undefined;
  try {
    // A window of an earlier build starts the host of the life, of its build.
    process.env.FURB_BUILD = "0.0.0+0ld0b0i1d000";
    try {
      old = await Window.open(yard);
    } finally {
      delete process.env.FURB_BUILD;
    }
    await old.type("Scenario: one.");
    await old.shows((frame) => frame.includes("One done."), "the first answer");
    const pid = yard.host() ?? 0;
    expect(pid).toBeGreaterThan(0);
    now = await Window.open(yard);
    expect(yard.host()).toBe(pid);
    expect(alive(pid)).toBe(true);
    await now.shows((frame) => frame.includes("One done."), "the life in the window of this build");
    await now.type("Scenario: two.");
    await now.shows((frame) => frame.includes("Two done."), "the answer in the window of this build");
    await old.shows((frame) => frame.includes("Two done."), "the answer in the window of the earlier build");
    expect(old.session.host.lost).toBe("");
    expect(alive(pid)).toBe(true);
  } finally {
    await now?.close(false).catch(() => {});
    await old?.close(true).catch(() => {});
    await yard.remove();
  }
});

test("a click on a workspace makes a new session in it, and opens no session that stands there", async () => {
  const yard = await Yard.make(SCRIPT);
  let window: Window | undefined;
  try {
    // The other workspace holds a session that stands, a copy of the record of the life of the yard.
    const other = join(yard.root, "other");
    await mkdir(join(other, ".furb", "sessions"), { recursive: true });
    window = await Window.open(yard);
    await window.type("Scenario: one.");
    await window.shows((frame) => frame.includes("One done."), "the first answer");
    const standing = join(other, ".furb", "sessions", "standing.jsonl");
    await copyFile(yard.record, standing);
    const library = window.library;
    const group = await library.add(other);
    await until(() => group.sessions.some((entry) => entry.path === standing), "the session that stands");
    await window.click("other");
    await until(() => library.groupOf() === group, "the workspace to take the focus");
    const current = library.current;
    expect(current?.path).not.toBe(standing);
    expect(group.sessions.find((entry) => entry.path === standing)?.session).toBeUndefined();
    const place = hostPlace(standing);
    expect(place && aboutOf(place)).toBeFalsy();
  } finally {
    await window?.close(true).catch(() => {});
    await yard.remove();
  }
});
