import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { saveFile } from "furb";
import { realConfigs, watch } from "../script/guard.ts";

test("a file that appears or goes in a watched folder shows by its path, once", () => {
  const folder = mkdtempSync(join(tmpdir(), "furb-guard-"));
  mkdirSync(join(folder, "sub"));
  writeFileSync(join(folder, "ui.json"), "{}");
  const written = watch([folder]);
  expect(written()).toEqual([]);
  writeFileSync(join(folder, "workspaces.json"), "{}");
  writeFileSync(join(folder, "sub", "one"), "a");
  expect(written()).toEqual([join(folder, "sub", "one"), join(folder, "workspaces.json")]);
  expect(written()).toEqual([]);
  rmSync(join(folder, "workspaces.json"));
  expect(written()).toEqual([join(folder, "workspaces.json")]);
  rmSync(folder, { recursive: true, force: true });
  expect(written()).toEqual([join(folder, "sub", "one"), join(folder, "ui.json")]);
});

test("a draft that another process makes and renames is no write of the run", () => {
  const folder = mkdtempSync(join(tmpdir(), "furb-guard-"));
  const written = watch([folder], new Set());
  writeFileSync(join(folder, "workspaces.json.tmp"), "{}");
  expect(written()).toEqual([]);
  rmSync(join(folder, "workspaces.json.tmp"));
  expect(written()).toEqual([]);
  rmSync(folder, { recursive: true, force: true });
});

test("a file that another process changed is no write of the run, and one that the run saved is", () => {
  const folder = mkdtempSync(join(tmpdir(), "furb-guard-"));
  writeFileSync(join(folder, "ui.json"), "{}");
  writeFileSync(join(folder, "workspaces.json"), "{}");
  const written = watch([folder]);
  // The program of the user keeps its files here while the tests run: a new size and a new time, and no save of this run.
  writeFileSync(join(folder, "ui.json"), '{"other":1}');
  utimesSync(join(folder, "workspaces.json"), new Date(0), new Date(0));
  expect(written()).toEqual([]);
  // The preload gives the tests a saveFile that notes each save, so a test that saves there is found by its path.
  saveFile(join(folder, "workspaces.json"), '{"mine":1}');
  expect(written()).toEqual([join(folder, "workspaces.json")]);
  rmSync(folder, { recursive: true, force: true });
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
