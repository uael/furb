import { expect, test } from "bun:test";
import { chmodSync, mkdtempSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import { environment } from "../script/environment.ts";

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
