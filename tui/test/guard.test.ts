import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { realConfigs, watch } from "../script/guard.ts";

test("a write, a change and a removal in a watched folder each show by their path, once", () => {
  const folder = mkdtempSync(join(tmpdir(), "furb-guard-"));
  mkdirSync(join(folder, "sub"));
  writeFileSync(join(folder, "ui.json"), "{}");
  const written = watch([folder]);
  expect(written()).toEqual([]);
  writeFileSync(join(folder, "workspaces.json"), "{}");
  writeFileSync(join(folder, "sub", "one"), "a");
  expect(written()).toEqual([join(folder, "sub", "one"), join(folder, "workspaces.json")]);
  expect(written()).toEqual([]);
  utimesSync(join(folder, "ui.json"), new Date(0), new Date(0));
  rmSync(join(folder, "workspaces.json"));
  expect(written()).toEqual([join(folder, "ui.json"), join(folder, "workspaces.json")]);
  rmSync(folder, { recursive: true, force: true });
  expect(written()).toEqual([join(folder, "sub", "one"), join(folder, "ui.json")]);
});

test("the words that the engine installs in the folder change with no test to blame", () => {
  const folder = mkdtempSync(join(tmpdir(), "furb-guard-"));
  const written = watch([folder]);
  mkdirSync(join(folder, "words"));
  writeFileSync(join(folder, "words", "todo.py"), "x");
  expect(written()).toEqual([]);
});

test("a folder that is not there is watched as empty, and a file that appears in it shows", () => {
  const folder = join(mkdtempSync(join(tmpdir(), "furb-guard-")), "later");
  const written = watch([folder]);
  expect(written()).toEqual([]);
  mkdirSync(folder);
  writeFileSync(join(folder, "ui.json"), "{}");
  expect(written()).toEqual([join(folder, "ui.json")]);
});

test("the config directories that no test may write are the native one and the one of the user, but not that of the tests", () => {
  const env = { XDG_CONFIG_HOME: "/home/someone/.config" };
  expect(realConfigs("/root/.furb/tests", "/root/.furb/tests", env)).toEqual(
    ["/home/someone/.config/furb"].map((one) => resolve(one)),
  );
  expect(realConfigs("/root/.furb/tests", "/elsewhere", env)).toEqual(
    ["/elsewhere", "/home/someone/.config/furb"].map((one) => resolve(one)),
  );
  expect(realConfigs("/root/.furb/tests", "/home/someone/.config/furb", env)).toEqual([
    resolve("/home/someone/.config/furb"),
  ]);
});
