import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  builtinExtensions,
  cacheDirectory,
  configDirectory,
  furbDirectory,
  type Instance,
  isInstance,
  loadWorldParts,
  remade,
  resolveExtensions,
  unwrapped,
  World,
} from "../src/index.ts";
import { exited, verb } from "./verbs.ts";

/** A project with a config of its own, and the config of the user, each written for one test and removed after it. */
async function project(
  local: object | undefined,
  home: object | undefined,
  run: (cwd: string) => Promise<void>,
) {
  const cwd = await mkdtemp(join(tmpdir(), "furb-extension-"));
  const config = join(configDirectory(), "config.json");
  try {
    if (local) {
      await mkdir(join(cwd, ".furb"));
      await writeFile(join(cwd, ".furb/config.json"), JSON.stringify({ extensions: local }));
    }
    if (home) {
      await mkdir(configDirectory(), { recursive: true });
      await writeFile(config, JSON.stringify({ extensions: home }));
    }
    await run(cwd);
  } finally {
    await rm(config, { force: true });
    await rm(cwd, { recursive: true, force: true });
  }
}

/** An extension in a directory: its manifest, its python part, and its part for a World in TypeScript. */
async function extension(root: string): Promise<void> {
  await mkdir(root, { recursive: true });
  await writeFile(
    join(root, "package.json"),
    JSON.stringify({ name: "echo", furb: { name: "echo", python: "echo.py", world: { ts: "world.ts" } } }),
  );
  await writeFile(
    join(root, "echo.py"),
    [
      "from furb.engine import Act, act, ending, idle, started",
      "",
      "",
      'def echo(text: str, on: str = "") -> Act[str]:',
      '  return act("echo", on, started(ending(idle)), text)',
      "",
    ].join("\n"),
  );
  await writeFile(
    join(root, "world.ts"),
    [
      'import type { Fact, WorldContext, WorldPart } from "@furb/engine";',
      "",
      "export default function echo(context: WorldContext): WorldPart {",
      "  return {",
      '    kinds: ["echo"],',
      "    *hears([kind, id]: Fact) {",
      '      if (kind !== "start") return;',
      '      const act = (yield { verb: "get", args: [id] }) as Fact;',
      '      if (act[0] === "echo") context.close(String(act[4]).toUpperCase(), id);',
      "    },",
      '    live: { born: (act) => "echoing " + String(act.words[0]) },',
      "  };",
      "}",
      "",
    ].join("\n"),
  );
}

test("the config of a project names its extensions over the config of the user, and false turns one off", async () => {
  expect(configDirectory()).toBe(String(process.env.FURB_CONFIG_DIR));
  expect(cacheDirectory()).toBe(String(process.env.FURB_CACHE_DIR));
  expect(builtinExtensions().map((one) => [one.name, one.builtin, one.requires])).toEqual([
    ["files", true, []],
    ["bash", true, ["files"]],
    ["grant", true, []],
  ]);
  await project({ grant: false }, { grant: true }, async (cwd) => {
    expect(resolveExtensions(cwd).map((one) => one.name)).toEqual(["files", "bash"]);
  });
  await project(undefined, { grant: false }, async (cwd) => {
    expect(resolveExtensions(cwd).map((one) => one.name)).toEqual(["files", "bash"]);
  });
  await project({ bash: false }, undefined, async (cwd) => {
    expect(resolveExtensions(cwd).map((one) => one.name)).toEqual(["files", "grant"]);
  });
});

test("an extension whose requirement is off is refused with what to turn off", async () => {
  await project({ files: false }, undefined, async (cwd) => {
    expect(() => resolveExtensions(cwd)).toThrow(
      "the extension bash requires files, which is off: turn bash off too, or turn files on",
    );
  });
});

test("an extension of a path plays its word, and its part for a World hears, asks the life, speaks later and gives a live value", async () => {
  await project({ echo: "../echo" }, undefined, async (cwd) => {
    await extension(join(cwd, "echo"));
    try {
      const [echo] = resolveExtensions(cwd).filter((one) => one.name === "echo");
      expect(echo?.builtin).toBe(false);
      expect(echo?.world.ts).toBe(join(cwd, "echo/world.ts"));
      expect(echo?.word?.split("\n")[0]).toBe("");
      expect(() => new World({ cwd, extensions: resolveExtensions(cwd) })).toThrow(
        "The extension echo has a World part",
      );
      const world = await World.load({ cwd });
      try {
        const life = world.open();
        const id = verb<string>(life, "echo", ["hi"]);
        expect(world.activity.acts.get(id)?.value).toBe("echoing hi");
        expect(await life.result<string>(id)).toBe("HI");
      } finally {
        await world.dispose();
      }
    } finally {
      await rm(join(cwd, "echo"), { recursive: true, force: true });
    }
  });
});

test("a part for a World must be the default export of its file, a function", async () => {
  const root = await mkdtemp(join(tmpdir(), "furb-bad-part-"));
  try {
    await writeFile(join(root, "world.ts"), "export const part = 1;\n");
    const bad = { name: "bad", builtin: false, requires: [], world: { ts: join(root, "world.ts") } };
    await expect(loadWorldParts([bad])).rejects.toThrow(
      `The World part of the extension bad, ${join(root, "world.ts")}, exports no default function.`,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("an instance crosses out as its fields, and a remade instance crosses back as its class", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-instance-"));
  const world = new World({ cwd });
  try {
    const life = world.open();
    const id = verb<string>(life, "bash", ["printf out"]);
    const exit = await life.result(id);
    expect(isInstance(exit, "Exit")).toBe(true);
    expect(isInstance(exit, "Text")).toBe(false);
    expect(await exited(life, id)).toEqual({
      code: 0,
      stdout: { path: `${id}/stdout`, content: "out", before: null },
      stderr: { path: `${id}/stderr`, content: "", before: null },
    });
    const stdout = (exit as Instance & { value: { stdout: Instance } }).value.stdout;
    const text = remade(stdout, { path: "made.txt", content: "made\n", before: null });
    expect(text).toEqual({
      is: "instance",
      class: stdout.class.id,
      fields: { path: "made.txt", content: "made\n", before: null },
    });
    expect(unwrapped<object>(verb(life, "write", [text]))).toEqual({
      path: join(cwd, "made.txt"),
      content: "made\n",
      before: null,
    });
    expect(await readFile(join(cwd, "made.txt"), "utf8")).toBe("made\n");
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("a verb said with no chain is said on the chain of who speaks, and the operator speaks on none", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-no-chain-"));
  const world = new World({ cwd });
  try {
    const life = world.open();
    expect(() => life.call("cwd", [], {})).toThrow("name 'cwd' is not defined, and no chain was said");
    expect(() => life.call("nothing", [], { on: life.root })).toThrow(
      "name 'nothing' is not defined on chain1",
    );
    expect(life.call<string>("scope", [life.root], {})).toBe(life.root);
  } finally {
    await world.dispose();
    await rm(cwd, { recursive: true, force: true });
  }
});

test("the directory of a project keeps its records out of version control but its config, whenever the rule is missing", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-ignore-"));
  try {
    await mkdir(join(cwd, ".furb"));
    expect(furbDirectory(cwd, "records")).toBe(join(cwd, ".furb/records"));
    expect(await readFile(join(cwd, ".furb/.gitignore"), "utf8")).toBe("*\n!config.json\n");
    await writeFile(join(cwd, ".furb/.gitignore"), "mine\n");
    furbDirectory(cwd);
    expect(await readFile(join(cwd, ".furb/.gitignore"), "utf8")).toBe("mine\n");
  } finally {
    await rm(cwd, { recursive: true, force: true });
  }
});
