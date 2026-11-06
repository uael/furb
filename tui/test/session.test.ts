import { afterAll, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { exitOf, isDone } from "furb/life";
import { alive, printPid, remove } from "../../bind/typescript/test/processes.ts";
import { until } from "../../bind/typescript/test/until.ts";
import { daemon } from "../src/client.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import type { Session } from "../src/session.ts";
import { withDemo } from "./composing.ts";
import { ended } from "./ended.ts";
import { idle } from "./idle.ts";

afterAll(removeDemoDirectories);

test("an @word that names no file is text of the message, and a word that names a file is read first", () =>
  withDemo(async (session) => {
    const message = "Add @dataclass to Point, as @README.md says, and install @types/node for @alice.";
    await session.submit(message);
    await idle(session);
    await session.refresh();
    const reads = session.acts.filter(
      (act) => act.kind === "rung" && act.by === "operator" && String(act.words[0]).startsWith("read("),
    );
    expect(reads.map((act) => [act.words[0], act.run?.run])).toEqual([['read("README.md")', "done"]]);
    expect(session.acts.filter((act) => session.isUserPrompt(act)).map((act) => act.words[1])).toEqual([
      message,
    ]);
  }));

test("a view that cannot be read opens the record with the default view and names the file", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-view-"));
  const record = join(directory, "life.jsonl");
  let session: Session | undefined;
  try {
    session = await demoSession({ record });
    await session.engine.result(await session.engine.rung({ word: "kept = 7", on: session.engine.root }));
    await session.dispose();
    for (const damaged of ["", '{"sessionName": "cut', "[1, 2]"]) {
      await writeFile(`${record}.ui.json`, damaged);
      session = await demoSession({ record });
      expect(session.notice).toContain(`Could not read ${record}.ui.json`);
      expect(session.sessionName).toBe(basename(directory));
      expect((await session.engine.inspect("kept", session.selected)).value).toBe(7);
      await session.dispose();
    }
    session = undefined;
  } finally {
    await session?.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a view whose field has another form opens that field empty and names it, and keeps the others", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-view-form-"));
  const record = join(directory, "life.jsonl");
  let session: Session | undefined;
  try {
    session = await demoSession({ record });
    await session.dispose();
    const view = JSON.parse(await readFile(`${record}.ui.json`, "utf8"));
    await writeFile(`${record}.ui.json`, JSON.stringify({ ...view, drafts: "one", models: [1] }));
    session = await demoSession({ record });
    expect(session.notice).toBe(
      `${record}.ui.json holds drafts, models in a form this release does not read: each opened empty.`,
    );
    expect([session.drafts, session.models]).toEqual([{}, []]);
    await session.dispose();
    session = undefined;
  } finally {
    await session?.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("a follow-up that an older ui.json holds goes to the queue of the life, once, and leaves the file", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-view-queue-"));
  const record = join(directory, "life.jsonl");
  let session: Session | undefined;
  try {
    session = await demoSession({ record });
    // A pause of the chain holds its queue, so the follow-up stays where the life puts it.
    await session.engine.pause(session.selected);
    const chain = session.selected;
    await session.dispose();
    const view = JSON.parse(await readFile(`${record}.ui.json`, "utf8"));
    const old = { id: "old-1", chain, text: "A follow-up of an older release", shape: "str" };
    await writeFile(`${record}.ui.json`, JSON.stringify({ ...view, queued: [old] }));
    const reopened = await demoSession({ record });
    session = reopened;
    await until(reopened, () => reopened.queued.some((one) => one.entry === "old-1"));
    expect(reopened.queued.map((one) => [one.entry, one.chain, one.message])).toEqual([
      ["old-1", chain, old.text],
    ]);
    const left = async () => "queued" in JSON.parse(await readFile(`${record}.ui.json`, "utf8"));
    for (let tries = 0; tries < 200 && (await left()); tries++) await Bun.sleep(25);
    expect(await left()).toBe(false);
  } finally {
    await session?.dispose();
    await rm(directory, { recursive: true, force: true });
  }
});

test("the setup of a demo life reads the doctrine of its own project, and no path of a home", async () => {
  const session = await demoSession();
  try {
    const read = session.acts.find(
      (act) => act.kind === "rung" && act.by === session.engine.root && /^read\(/.test(String(act.word)),
    );
    const path = /^read\('(.*)'\)$/.exec(String(read?.word))?.[1] ?? "";
    // The engine writes a path under the home folder with a ~, so the project is known by its name and its folder.
    // The word is Python, which writes each backslash of a Windows path twice.
    expect(
      path.replaceAll("\\\\", "\\").endsWith(join(basename(session.host.directory), ".furb", "DOCTRINE.md")),
    ).toBe(true);
  } finally {
    await session.dispose();
  }
});

test("a follow-up goes to the chain that the operator sent it on, with its images, whatever chain he shows meanwhile", async () => {
  const session = await demoSession();
  const { engine, host } = session;
  const queue = host.queue;
  try {
    const a = session.selected;
    const b = await engine.chain({ label: "Other" });
    const image = (name: string) =>
      ({ name, uri: `furb-image://${name}`, mime: "image/png", size: 1 }) as never;
    const asked: unknown[][] = [];
    for (const at of ["read", "queue"] as const) {
      let open = () => {};
      const gate = new Promise<void>((resolve) => {
        open = resolve;
      });
      // The read of the directory and the queue answer when the test lets them, after a change of chain.
      Object.assign(session, {
        engine: new Proxy(engine, {
          get: (target, key) =>
            key === "cwd" && at === "read"
              ? async (...args: Parameters<typeof engine.cwd>) => {
                  await gate;
                  return target.cwd(...args);
                }
              : Reflect.get(target, key),
        }),
      });
      host.queue = async (...args) => {
        asked.push(args);
        if (at === "queue") await gate;
      };
      await session.select(a);
      session.images = { [a]: [image(`a-${at}`)], [b]: [image(`b-${at}`)] };
      const sent = session.enqueue(`Sent on the first chain, ${at}`);
      await session.select(b);
      open();
      await sent;
      expect(asked.at(-1)?.slice(0, 1).concat(asked.at(-1)?.slice(2))).toEqual(["str", a]);
      expect(String(asked.at(-1)?.[1])).toContain(`a-${at}`);
      expect(String(asked.at(-1)?.[1])).not.toContain(`b-${at}`);
      expect(session.images[a]).toBeUndefined();
      expect(session.images[b]?.map((one) => one.name)).toEqual([`b-${at}`]);
    }
  } finally {
    Object.assign(session, { engine });
    host.queue = queue;
    await session.dispose();
  }
});

test("a message goes to the chain that the operator sent it on, with its images, whatever chain he shows meanwhile", async () => {
  const session = await demoSession();
  const { engine } = session;
  const prompt = engine.prompt;
  try {
    const a = session.selected;
    const b = await engine.chain({ label: "Other" });
    const image = (name: string) =>
      ({ name, uri: `furb-image://${name}`, mime: "image/png", size: 1 }) as never;
    const asked: { message?: string; on?: string }[] = [];
    for (const at of ["read", "prompt"] as const) {
      let open = () => {};
      const gate = new Promise<void>((resolve) => {
        open = resolve;
      });
      // The read of the directory and the prompt answer when the test lets them, after a change of chain.
      Object.assign(session, {
        engine: new Proxy(engine, {
          get: (target, key) => {
            if (key === "cwd" && at === "read")
              return async (...args: Parameters<typeof engine.cwd>) => {
                await gate;
                return target.cwd(...args);
              };
            if (key === "prompt")
              return async (...args: Parameters<typeof engine.prompt>) => {
                asked.push(args[1] ?? {});
                if (at === "prompt") await gate;
                return "prompt0";
              };
            return Reflect.get(target, key);
          },
        }),
      });
      await session.select(a);
      session.aim();
      session.images = { [a]: [image(`a-${at}`)], [b]: [image(`b-${at}`)] };
      const sent = session.submit(`Sent on the first chain, ${at}`);
      await session.select(b);
      open();
      await sent;
      expect(asked.at(-1)?.on).toBe(a);
      expect(asked.at(-1)?.message).toContain(`a-${at}`);
      expect(asked.at(-1)?.message).not.toContain(`b-${at}`);
      expect(session.images[a]).toBeUndefined();
      expect(session.images[b]?.map((one) => one.name)).toEqual([`b-${at}`]);
    }
  } finally {
    Object.assign(session, { engine });
    engine.prompt = prompt;
    await session.dispose();
  }
});

test("a session that cannot save its view still lets its life go, which ends in the daemon of the tests with its command", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-dispose-"));
  const record = join(directory, "life.jsonl");
  const session = await demoSession({ record });
  try {
    const command = await session.engine.bash(`${printPid}; sleep 30`, { on: session.engine.root });
    const pid = () => Number(exitOf(session.acts.find((act) => act.id === command))?.stdout?.content);
    await until(session, () => pid() > 0);
    // A directory where the save writes its file makes the save fail on every system.
    await mkdir(`${record}.ui.json.tmp`);
    const failed = await session.dispose().catch((error: unknown) => error);
    expect(String(failed)).toContain("life.jsonl.ui.json.tmp");
    expect(session.engine.disposed).toBe(true);
    // No window holds the life now, and the daemon of the tests gives a life no grace, so the life ends, and its
    // checkpoint cuts the command that ran.
    await ended(record);
    const listed = await daemon().ask({ verb: "list" });
    expect(listed.answer === "lives" && listed.data.find((one) => one.life === record)?.open).toBeFalsy();
    for (const until = Date.now() + 10_000; alive(pid()) && Date.now() < until; ) await Bun.sleep(50);
    expect(alive(pid())).toBe(false);
  } finally {
    await remove(directory);
  }
});

test("a snapshot asked before a model choice lands after it, and the choice holds for the next prompt", async () => {
  const session = await demoSession();
  const snapshot = session.host.snapshot.bind(session.host);
  const { promise: gate, resolve: release } = Promise.withResolvers<void>();
  try {
    await idle(session);
    const { promise: asked, resolve: taken } = Promise.withResolvers<void>();
    session.host.snapshot = async (chain) => {
      session.host.snapshot = snapshot;
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
    expect(session.actor).toBe("claude-cli:opus/high");
    await session.submit("Which model reads this?");
    const prompt = session.acts.findLast((act) => session.isUserPrompt(act));
    expect(prompt?.words).toEqual(["str", "Which model reads this?", ""]);
    await idle(session);
    await session.refresh();
    expect(session.acts.find((act) => act.kind === "rung" && act.by === prompt?.id)?.words[2]).toBe(
      "claude-cli:opus/high",
    );
    expect((await session.engine.inspect("actor", session.engine.root)).value).toBe("claude-cli:opus/high");
    expect(session.actor).toBe("claude-cli:opus/high");
  } finally {
    release();
    session.host.snapshot = snapshot;
    await session.dispose();
  }
});

/** The actor word of each rung that a prompt made, in order. */
const asked = (session: Session, prompt: string | undefined) =>
  session.acts.filter((act) => act.kind === "rung" && act.by === prompt).map((act) => act.words[2]);

/** Make the demo hold its prompt on a question to the operator, set the effort low while it is held, and wait until
 * that choice has run on the chain, so the next rung of the prompt comes after the choice. */
async function heldWhileEffortChanges(
  session: Session,
  during: () => void = () => {},
): Promise<string | undefined> {
  await session.submit("Look around.");
  await idle(session);
  // The demo asks the operator and closes nothing, so its prompt makes no next rung until the operator answers.
  await session.submit("Ask me first, then answer.");
  const prompt = session.acts.findLast((act) => session.isUserPrompt(act))?.id;
  await until(session, () => session.operatorPrompt !== undefined);
  during();
  await session.submit("/effort low");
  await until(session, () =>
    session.acts.some(
      (act) => act.by === "operator" && act.words[0] === 'actor = "claude-cli:sonnet/low"' && isDone(act),
    ),
  );
  await session.submit("Take the low effort.");
  await idle(session);
  return prompt;
}

test("/effort changes the actor of the next rung of a running chain", () =>
  withDemo(async (session) => {
    const prompt = await heldWhileEffortChanges(session);
    await session.refresh();
    expect(asked(session, prompt)).toEqual(["claude-cli:sonnet/high", "claude-cli:sonnet/low"]);
  }));

test("a follow-up goes to the actor of its chain when it is sent", () =>
  withDemo(async (session) => {
    await heldWhileEffortChanges(session, () =>
      session.enqueue("Check keyboard navigation after this answer."),
    );
    await until(session, () => session.queued.length === 0);
    await idle(session);
    await session.refresh();
    const follow = session.acts.findLast((act) => session.isUserPrompt(act));
    expect(asked(session, follow?.id)).toEqual(["claude-cli:sonnet/low"]);
    expect(follow?.words).toEqual(["str", "Check keyboard navigation after this answer.", ""]);
  }));

test("a follow-up that the operator takes out of the queue is not sent", async () => {
  const session = await demoSession();
  try {
    const root = session.selected;
    const other = await session.engine.chain({ label: "Other" });
    // A pause of each chain holds its queue while the operator takes a follow-up out.
    for (const chain of [root, other]) await session.engine.pause(chain);
    await session.enqueue("First follow-up on main");
    await session.select(other);
    await session.enqueue("Second follow-up on the other chain");
    await until(session, () => session.queued.length === 2);
    const later = session.queued.find((one) => one.chain === other);
    if (!later) throw new Error("No follow-up on the other chain.");
    await session.removeQueued(later.entry);
    for (const chain of [root, other]) await session.engine.wake(chain);
    await until(session, () => session.queued.length === 0);
    await idle(session);
    await session.refresh();
    expect(session.queued).toEqual([]);
    expect(session.acts.filter((act) => session.isUserPrompt(act)).map((act) => act.words[1])).toEqual([
      "First follow-up on main",
    ]);
    // The other chain holds only the rungs that run again the words of the setup of the root that its prefix holds,
    // the debug of the extensions and what it told, the read of the doctrine and what that read told, then the stand and the cd that it
    // plays on itself, as the root played its own.
    const setup = session.acts.filter((act) => act.kind === "rung" && act.by === session.engine.root);
    const [first, read] = setup;
    const stood = setup.at(-1);
    expect(first?.word).toBe('debug(t"{standing()[4]=}")');
    expect(read?.word).toMatch(/^read\('.*DOCTRINE\.md'\)$/);
    expect(stood?.word).toMatch(/^stand\(\)\ncd\('.*'\)$/);
    expect(
      session.acts
        .filter((act) => act.chain === other && act.kind === "rung")
        .map((act) => [act.by, act.words]),
    ).toEqual([
      [other, [first?.word, first?.id, ""]],
      [other, ["#rung1 debugged\nrung1_debug = 'standing()[4]=[]'", `${read?.id}_told`, ""]],
      [other, [read?.word, read?.id, ""]],
      [other, [expect.stringMatching(/^#read1\nread1_path = '.*DOCTRINE\.md'\n/), `${stood?.id}_told`, ""]],
      [other, [stood?.word, "", "claude-cli:sonnet/high"]],
    ]);
  } finally {
    await session.dispose();
  }
});

test("each /feed sends one line, and a /feed with no text closes the input", () =>
  withDemo(async (session) => {
    const command = await session.engine.bash('read -r a; read -r b; echo "a=[$a] b=[$b]"; cat', {
      stdin: null,
      on: session.engine.root,
    });
    await session.submit(`/feed ${command} yes`);
    await session.submit(`/feed ${command} two  words`);
    await until(session, () =>
      String(exitOf(session.acts.find((act) => act.id === command))?.stdout?.content).includes(
        "a=[yes] b=[two  words]\n",
      ),
    );
    expect((await session.engine.outcome(command)).done).toBe(false);
    await session.submit(`/feed ${command}`);
    expect(await session.engine.result(command)).toMatchObject({
      code: 0,
      stdout: { content: "a=[yes] b=[two  words]\n" },
    });
  }));

test("a path that starts with ~ is read from the home directory by /share, /export and /image", async () => {
  const home = await mkdtemp(join(tmpdir(), "furb-home-"));
  const script = join(home, "run.ts");
  const pixel =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";
  try {
    await writeFile(join(home, "pixel.png"), Buffer.from(pixel, "base64"));
    // The home directory of a process is read once, so a process of its own gives the test a home of its own.
    await writeFile(
      script,
      `import { demoSession, removeDemoDirectories } from ${JSON.stringify(join(import.meta.dir, "../src/demo.ts"))};
const session = await demoSession();
try {
  await session.submit("/share ~/shared/chat.html");
  await session.submit("/export ~/export.json");
  await session.attachImage("~/pixel.png");
  console.log(JSON.stringify({ images: session.images[session.selected]?.length }));
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
    expect(JSON.parse(output)).toEqual({ images: 1 });
    expect(await readFile(join(home, "shared/chat.html"), "utf8")).toContain("<!doctype html>");
    expect(JSON.parse(await readFile(join(home, "export.json"), "utf8")).chain).toBe("chain1");
    expect(existsSync(join(home, "~"))).toBe(false);
  } finally {
    await rm(home, { recursive: true, force: true });
  }
});

test("an undo leaves out of its branch a grant that came after the message it takes back", () =>
  withDemo(async (session) => {
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
  }));

test("/copy takes the last answer of the chain and says how many characters", () =>
  withDemo(async (session) => {
    await session.submit("hello");
    await idle(session);
    await session.refresh();
    const copied: string[] = [];
    session.on("copied", (text: string) => copied.push(text));
    await session.command("/copy");
    const answer = session.turns.findLast(([role]) => role === "assistant")?.[1];
    expect(answer).toBeTruthy();
    expect(copied).toEqual([answer ?? ""]);
    expect(session.notice).toBe(`Copied ${answer?.length} characters.`);
  }));

test("a message of a member of the operator is a message of the operator", () =>
  withDemo(async (session) => {
    const message = {
      id: "prompt90",
      kind: "prompt",
      by: "operator/a",
      on: session.selected,
      chain: session.selected,
      words: ["str", "Hello", ""],
      phase: { phase: "asked" as const },
      operator: false,
      began: 1,
      state: "runs" as const,
    };
    expect(session.isUserPrompt(message)).toBe(true);
  }));
