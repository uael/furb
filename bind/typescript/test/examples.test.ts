import { expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { executable } from "./executable.ts";
import { remove } from "./processes.ts";

const examples = join(import.meta.dir, "..", "..", "..", "docs", "examples", "typescript");
const fake = join(import.meta.dir, "claude.ts");

/** The directory an example runs in, and the fake claude beside it, which answers with these words in order. */
async function yard(...words: string[]) {
  const root = await realpath(await mkdtemp(join(tmpdir(), "furb-example-")));
  const [at, home] = [join(root, "work"), join(root, "fake")];
  await mkdir(at);
  await mkdir(home);
  const claude = executable(fake, home, "claude");
  for (const [n, word] of words.entries()) await writeFile(join(home, `word.${n + 1}`), JSON.stringify(word));
  const env: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env))
    if (!key.startsWith("FURB_") && value) env[key] = value;
  Object.assign(env, {
    FURB_CLAUDE_BIN: claude,
    FURB_CONFIG_DIR: join(root, "config"),
    XDG_CACHE_HOME: join(root, "cache"),
  });
  /** The example started in the yard. */
  const start = (name: string, stdin = "") => {
    const run = Bun.spawn([process.execPath, join(examples, `${name}.ts`)], {
      cwd: at,
      env,
      stdin: new TextEncoder().encode(stdin),
      stdout: "pipe",
      stderr: "pipe",
    });
    return run;
  };
  /** What the example printed, once it ended well. */
  const run = async (name: string, stdin = "") => {
    const one = start(name, stdin);
    const [code, out, err] = await Promise.all([
      one.exited,
      new Response(one.stdout).text(),
      new Response(one.stderr).text(),
    ]);
    expect(err).toBe("");
    expect(code).toBe(0);
    return out;
  };
  return { root, at, home, start, run };
}

test("a life goes on in a second process after its first process ends", async () => {
  const { root, home, at, start, run } = await yard();
  try {
    await writeFile(join(home, "deaf"), "");
    const first = start("durable");
    while (!existsSync(join(home, "asked"))) await Bun.sleep(20);
    first.kill();
    await first.exited;
    await rm(join(home, "deaf"));
    await writeFile(join(home, "word.1"), JSON.stringify('close("Ship version 2 on Friday.")'));
    expect(await run("durable")).toBe("Ship version 2 on Friday.\n");
    const facts = (await readFile(join(at, "release.jsonl"), "utf8")).split("\n").filter(Boolean);
    const asked = facts
      .map((line) => JSON.parse(line)[0])
      .filter((fact) => fact[0] === "prompt" && fact[2] === "operator");
    expect(asked.map((fact) => fact[1])).toEqual(["prompt1"]);
  } finally {
    await remove(root);
  }
});

test("the deaf claude of a life ends when the process of the life is killed", async () => {
  const { root, home, start } = await yard();
  try {
    await writeFile(join(home, "deaf"), "");
    const first = start("durable");
    while (!existsSync(join(home, "asked"))) await Bun.sleep(20);
    first.kill("SIGKILL");
    await first.exited;
    const pid = Number(await readFile(join(home, "asked"), "utf8"));
    const lives = () => {
      try {
        process.kill(pid, 0);
        return true;
      } catch {
        return false;
      }
    };
    for (let n = 0; n < 60 && lives(); n++) await Bun.sleep(50);
    expect(lives()).toBe(false);
  } finally {
    await remove(root);
  }
});

test("a reviewer on a chain of its own reviews the draft of the root", async () => {
  const { root, run } = await yard('close("Login is twice as fast.")', 'close("The notes read well.")');
  try {
    expect(await run("chains")).toBe("The notes read well.\nreviewer under root\n");
  } finally {
    await remove(root);
  }
});

test("the model asks the operator before it deploys", async () => {
  const word =
    'go = await prompt(bool, "Deploy version 2 now?", to=OPERATOR)\nclose("deployed" if go else "held")';
  const { root, run } = await yard(word);
  try {
    expect(await run("questions", "yes\n")).toBe("Deploy version 2 now? deployed\n");
  } finally {
    await remove(root);
  }
});

test("a grant pauses the chain at its ceiling and a larger grant lets it go on", async () => {
  const { root, home, run } = await yard("files = 3", "files = 3", "close(1200)");
  try {
    await writeFile(join(home, "cost"), "0.3");
    expect(await run("grant")).toBe("paused at 0.60 dollars\n1200\n");
  } finally {
    await remove(root);
  }
});

test("an edit of the door of a thread changes the plan of the model", async () => {
  const { root, run } = await yard('await wait(3600)\nclose("released")', "await wait(3600)");
  try {
    expect(await run("door")).toBe('await wait(3600)\nclose("released")\nreleased\n');
  } finally {
    await remove(root);
  }
});
