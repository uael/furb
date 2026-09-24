import { afterAll, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { RecordLock } from "@furb/engine";
import { alive, printPid, remove } from "../../bind/typescript/test/processes.ts";
import { until } from "../../bind/typescript/test/until.ts";
import { type Engine, openEngine } from "../src/bridge.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import { Session } from "../src/session.ts";
import { idle } from "./idle.ts";

afterAll(removeDemoDirectories);

test("an @word that names no file is text of the message, and a word that names a file is read first", async () => {
  const session = await demoSession();
  try {
    const message = "Add @dataclass to Point, as @README.md says, and install @types/node for @alice.";
    await session.submit(message);
    await idle(session);
    await session.refresh();
    const reads = session.acts.filter(
      (act) => act.kind === "rung" && act.by === "operator" && String(act.words[0]).startsWith("read("),
    );
    expect(reads.map((act) => [act.words[0], act.run?.status])).toEqual([['read("README.md")', "done"]]);
    expect(session.acts.filter((act) => session.isUserPrompt(act)).map((act) => act.words[1])).toEqual([
      message,
    ]);
  } finally {
    await session.dispose();
  }
}, 30000);

test("a view that cannot be read opens the record with the default view and names the file", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-view-"));
  const record = join(directory, "life.jsonl");
  let session: Session | undefined;
  try {
    let opened = await openEngine({ cwd: directory, record, demo: true });
    session = new Session(opened.life, opened.world, true);
    session.sessionName = "Named";
    await session.life.result(await session.life.rung("kept = 7"));
    await session.dispose();
    for (const damaged of ["", '{"sessionName": "cut', "[1, 2]"]) {
      await writeFile(`${record}.ui.json`, damaged);
      opened = await openEngine({ cwd: directory, record, demo: true });
      session = new Session(opened.life, opened.world, true);
      expect(session.notice).toContain(`Could not read ${record}.ui.json`);
      expect(session.sessionName).toBe(basename(directory));
      expect(await session.life.held("modules", [session.selected, "kept"], "at")).toBe(7);
      await session.dispose();
    }
    session = undefined;
  } finally {
    await session?.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("a session that cannot save its view still ends its World, its commands and its lease", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-dispose-"));
  const record = join(directory, "life.jsonl");
  const opened = await openEngine({ cwd: directory, record, demo: true });
  const session = new Session(opened.life, opened.world, true);
  try {
    const command = await session.life.bash(`${printPid}; sleep 30`);
    const pid = () =>
      Number(
        (session.acts.find((act) => act.id === command)?.value as { stdout?: { content: string } })?.stdout
          ?.content,
      );
    await until(session, () => pid() > 0);
    // A directory where the save writes its file makes the save fail on every system.
    await mkdir(`${record}.ui.json.tmp`);
    const failed = await session.dispose().catch((error: unknown) => error);
    expect(String(failed)).toContain("life.jsonl.ui.json.tmp");
    expect(session.life.disposed).toBe(true);
    for (let tries = 0; alive(pid()) && tries < 100; tries++) await Bun.sleep(20);
    expect(alive(pid())).toBe(false);
    new RecordLock(record).dispose();
  } finally {
    await remove(directory);
  }
}, 30000);

test("a snapshot asked before a model choice lands after it, and the choice holds for the next prompt", async () => {
  const session = await demoSession();
  const snapshot = session.world.snapshot.bind(session.world);
  let release = () => {};
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  try {
    await idle(session);
    let taken = () => {};
    const asked = new Promise<void>((resolve) => {
      taken = resolve;
    });
    session.world.snapshot = async (chain) => {
      session.world.snapshot = snapshot;
      const early = await snapshot(chain);
      taken();
      await gate;
      return early;
    };
    const reading = session.refresh();
    await asked;
    await session.command("/model claude-cli:opus");
    release();
    await reading;
    expect(session.actor).toBe("claude-cli:opus/low");
    await session.submit("Which model reads this?");
    const prompt = session.acts.findLast((act) => session.isUserPrompt(act));
    expect(prompt?.words).toEqual(["str", "Which model reads this?", "claude-cli:opus/low"]);
    await idle(session);
    await session.refresh();
    expect(await session.life.held("modules", [session.life.root, "actor"], "at")).toBe(
      "claude-cli:opus/low",
    );
    expect(session.actor).toBe("claude-cli:opus/low");
  } finally {
    release();
    session.world.snapshot = snapshot;
    await session.dispose();
  }
}, 30000);

test("a follow-up that the operator removes while an earlier one is sent is not sent", async () => {
  const session = await demoSession();
  const send = session.world.sendQueued.bind(session.world);
  try {
    const other = await session.life.chain("Other");
    session.queueHeld = true;
    session.enqueue("First follow-up on main");
    await session.select(other);
    session.enqueue("Second follow-up on the other chain, about @README.md");
    session.world.sendQueued = async (entry) => {
      const later = session.queued.find((item) => item.id !== entry.id);
      if (later) session.removeQueued(later.id);
      return send(entry);
    };
    session.queueHeld = false;
    await session.drainQueue();
    await idle(session);
    await session.refresh();
    expect(session.queued).toEqual([]);
    expect(session.acts.filter((act) => session.isUserPrompt(act)).map((act) => act.words[1])).toEqual([
      "First follow-up on main",
    ]);
    expect(session.acts.filter((act) => act.on === other && act.kind === "rung")).toEqual([]);
  } finally {
    session.world.sendQueued = send;
    await session.dispose();
  }
}, 30000);

test("a follow-up that the operator removes while its files are read is not sent", async () => {
  const session = await demoSession();
  const life = session.life;
  try {
    session.queueHeld = true;
    session.enqueue("Explain @README.md");
    const [entry] = session.queued;
    if (!entry) throw new Error("No follow-up.");
    // The operator removes the follow-up as the read of its file starts.
    const reading = new Proxy(life, {
      get(target, key) {
        const value = Reflect.get(target, key);
        if (key !== "rung") return value;
        return (...args: Parameters<Engine["rung"]>) => {
          if (session.queued.includes(entry)) session.removeQueued(entry.id);
          return life.rung(...args);
        };
      },
    });
    Object.assign(session, { life: reading });
    session.queueHeld = false;
    await session.drainQueue();
    Object.assign(session, { life });
    await idle(session);
    await session.refresh();
    expect(session.queued).toEqual([]);
    expect(session.acts.some((act) => act.kind === "rung" && act.words[0] === 'read("README.md")')).toBe(
      true,
    );
    expect(session.acts.filter((act) => session.isUserPrompt(act))).toEqual([]);
  } finally {
    Object.assign(session, { life });
    await session.dispose();
  }
}, 30000);

test("each /feed sends one line, and a /feed with no text closes the input", async () => {
  const session = await demoSession();
  try {
    const command = await session.life.bash('read -r a; read -r b; echo "a=[$a] b=[$b]"; cat', { fed: true });
    await session.submit(`/feed ${command} yes`);
    await session.submit(`/feed ${command} two  words`);
    await until(session, () =>
      String(
        (session.acts.find((act) => act.id === command)?.value as { stdout?: { content: string } })?.stdout
          ?.content,
      ).includes("a=[yes] b=[two  words]\n"),
    );
    expect((await session.life.outcome(command)).done).toBe(false);
    await session.submit(`/feed ${command}`);
    expect(await session.life.result(command)).toMatchObject({
      code: 0,
      stdout: { content: "a=[yes] b=[two  words]\n" },
    });
  } finally {
    await session.dispose();
  }
}, 30000);

test("/close carries a whole float as a float, as the operator wrote it", async () => {
  const session = await demoSession();
  try {
    const prompt = await session.life.prompt("float", "A number?", { to: "operator" });
    await session.submit(`/close ${prompt} 2.0`);
    expect(await session.life.result(prompt)).toBe(2);
    const whole = await session.life.prompt("float", "Another number?", { to: "operator" });
    expect(String(await session.submit(`/close ${whole} 2`).catch((error: unknown) => error))).toContain(
      "2 not float",
    );
  } finally {
    await session.dispose();
  }
}, 30000);

test("/model finds a model of the roster by the rule of the World, so an id with a colon names it", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-models-"));
  const opened = await openEngine({
    cwd: directory,
    model: "openai:gpt-4o",
    roster: ["amazon-bedrock:amazon.nova-lite-v1:0"],
  });
  const session = new Session(opened.life, opened.world);
  try {
    await session.refresh();
    await session.submit("/model amazon.nova-lite-v1:0");
    expect(session.actor).toBe("amazon-bedrock:amazon.nova-lite-v1:0/off");
    expect(String(await session.submit("/model 0").catch((error: unknown) => error))).toContain(
      "Choose one of",
    );
    await session.submit("/model gpt-4o");
    expect(session.actor).toBe("openai:gpt-4o/off");
  } finally {
    await session.dispose();
    await rm(directory, { recursive: true, force: true });
  }
}, 30000);

test("the answers to the questions of the snapshots stay out of the facts of the host", async () => {
  const session = await demoSession();
  try {
    await session.submit("A question.");
    await idle(session);
    for (const view of ["feed", "transcript", "changes"] as const) {
      session.show(view);
      await session.refresh();
    }
    const acts = new Set(session.acts.map((act) => act.id));
    expect(
      session.world.facts.filter(
        ([kind, id]) => kind === "done" && /^\w+:\/\/operator\.\d+$/.test(id) && !acts.has(id),
      ),
    ).toEqual([]);
  } finally {
    await session.dispose();
  }
}, 30000);

test("a path that starts with ~ is read from the home directory by /share, /export, /image and /extension", async () => {
  const home = await mkdtemp(join(tmpdir(), "furb-home-"));
  const script = join(home, "run.ts");
  const pixel =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";
  try {
    await writeFile(join(home, "pixel.png"), Buffer.from(pixel, "base64"));
    await writeFile(
      join(home, "extension.ts"),
      'export default (api) => api.registerCommand("home-probe", { label: "Home", description: "A probe", run() {} });',
    );
    // The home directory of a process is read once, so a process of its own gives the test a home of its own.
    await writeFile(
      script,
      `import { demoSession, removeDemoDirectories } from ${JSON.stringify(join(import.meta.dir, "../src/demo.ts"))};
import { Extensions } from ${JSON.stringify(join(import.meta.dir, "../src/extensions.ts"))};
const session = await demoSession();
const extensions = new Extensions(() => { throw new Error("No context is asked."); });
try {
  await session.submit("/share ~/shared/chat.html");
  await session.submit("/export ~/export.json");
  await session.attachImage("~/pixel.png");
  await extensions.load(session.path("~/extension.ts"));
  console.log(JSON.stringify({ images: session.images[session.selected]?.length, commands: [...extensions.commands.keys()] }));
} finally {
  await session.dispose();
  await removeDemoDirectories();
}
`,
    );
    // Windows reads the home directory from USERPROFILE, and every other system from HOME.
    const child = Bun.spawn([process.execPath, script], {
      cwd: home,
      env: { ...process.env, HOME: home, USERPROFILE: home },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [code, output, errors] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    expect(errors).toBe("");
    expect(code).toBe(0);
    expect(JSON.parse(output)).toEqual({ images: 1, commands: ["home-probe"] });
    expect(await readFile(join(home, "shared/chat.html"), "utf8")).toContain("<!doctype html>");
    expect(JSON.parse(await readFile(join(home, "export.json"), "utf8")).chain).toBe("chain1");
    expect(existsSync(join(home, "~"))).toBe(false);
  } finally {
    await rm(home, { recursive: true, force: true });
  }
}, 30000);

test("an undo leaves out of its branch a grant that came after the message it takes back", async () => {
  const session = await demoSession();
  try {
    await session.submit("Explore this project.");
    await idle(session);
    await session.submit("/grant 1.5");
    await session.refresh();
    const grant = session.activity.find((act) => act.kind === "grant");
    if (!grant) throw new Error("The grant made no act.");
    await session.undo();
    await session.refresh();
    const told = session.turns.map(([, python]) => python).join("\n");
    expect(told).not.toContain(`#${grant.id}`);
    expect(session.activity.some((act) => act.kind === "grant")).toBe(false);
  } finally {
    await session.dispose();
  }
});
