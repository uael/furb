// The walk of the audit of the TUI goes to the end of its journey on a life of the scripted model. What its probes find
// is for the audit to judge against main, so the test prints it, fails on a stuck step or an input that takes no key, and holds the walk alone otherwise: each step, its capture and
// the action before it.
import { expect } from "bun:test";
import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "./harness.ts";
import { END, walk } from "./walk.ts";

test("the walk of the audit drives a life with keys to the end of its journey, and keeps each frame with its action", async () => {
  const out = await mkdtemp(join(tmpdir(), "furb-walk-"));
  try {
    const { steps, findings } = await walk(out, []);
    for (const one of findings) console.log(`${one.step} ${one.probe}: ${one.detail}`);
    // A step that is stuck and a key that reaches no input fail the walk: the rest is for the audit to judge.
    expect(findings.filter((one) => one.probe === "stuck" || one.probe === "input")).toEqual([]);
    // A key that is a toggle undoes itself: ⌥D, the details.
    expect(findings.filter((one) => one.detail.includes("⌥D"))).toEqual([]);
    expect(steps.at(-1)?.action).toBe(END);
    expect(steps.length).toBeGreaterThan(40);
    const missing = steps.flatMap(({ name }) =>
      ["png", "json", "txt"].map((kind) => `${name}.${kind}`).filter((file) => !existsSync(join(out, file))),
    );
    expect(missing).toEqual([]);
    expect(existsSync(join(out, "walk.json"))).toBe(true);
  } finally {
    await rm(out, { recursive: true, force: true });
  }
}, 900000);
