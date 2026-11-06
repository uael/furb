import { expect, test } from "bun:test";
import { chmodSync, mkdtempSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import { environment } from "../script/environment.ts";
import { daemonFolder, daemonPlace } from "../src/client.ts";

/** A folder that holds a claude command line and node, as /opt/homebrew/bin does. */
function shared(): string {
  const folder = mkdtempSync(join(tmpdir(), "furb-env-"));
  writeFileSync(join(folder, "claude"), "#!/bin/sh\n");
  chmodSync(join(folder, "claude"), 0o755);
  symlinkSync(process.execPath, join(folder, "node"));
  return folder;
}

test("the environment of the tests keeps every folder of the PATH, node among them", () => {
  const folder = shared();
  const path = [folder, "/usr/bin"].join(delimiter);
  expect(environment({ PATH: path }).PATH).toBe(path);
});

test("the environment of the tests hides the claude command line with an empty FURB_CLAUDE_BIN", () => {
  const given = environment({ PATH: shared(), FURB_CLAUDE_BIN: "/somewhere/claude" });
  expect(given.FURB_CLAUDE_BIN).toBe("");
});

test("the socket of the daemon of the tests fits a Unix socket in a worktree at a long path, and two worktrees keep two caches", () => {
  const long = join("/", "a".repeat(60), "b".repeat(60), "worktree");
  const here = environment({}, long);
  expect(Buffer.byteLength(daemonPlace(daemonFolder(here)))).toBeLessThan(104);
  expect(here.XDG_CACHE_HOME).not.toBe(environment({}, `${long}2`).XDG_CACHE_HOME);
  expect(here.FURB_CONFIG_DIR).toBe(join(long, ".furb/tests"));
});

test("on Windows the cache of the tests stands in the temporary directory of the machine, whatever yard a process has", () => {
  const platform = Object.getOwnPropertyDescriptor(process, "platform");
  Object.defineProperty(process, "platform", { value: "win32" });
  try {
    const machine = "/machine";
    const runner = environment({ FURB_MACHINE_TMPDIR: machine });
    const test = environment({ FURB_MACHINE_TMPDIR: machine, TMPDIR: "/yard", TEMP: "/yard", TMP: "/yard" });
    expect(runner.XDG_CACHE_HOME?.startsWith(join(machine, "furb-cache-"))).toBe(true);
    expect(test.XDG_CACHE_HOME).toBe(runner.XDG_CACHE_HOME);
  } finally {
    if (platform) Object.defineProperty(process, "platform", platform);
  }
});
