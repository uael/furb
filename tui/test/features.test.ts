import { afterAll, expect, test } from "bun:test";
import { link, mkdir, mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { delimiter, extname, join } from "node:path";
import { executable, programs } from "../../bind/typescript/test/executable.ts";
import { remove } from "../../bind/typescript/test/processes.ts";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { clipboardImage } from "../src/clipboard.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import { externalEditor, opener } from "../src/editor.ts";
import { fileReferences, projectFiles } from "../src/files.ts";
import { Session } from "../src/session.ts";
import { publishShare, shareHtml, shareMarkdown } from "../src/share.ts";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";
import { transcriptOf } from "./transcript.ts";

afterAll(removeDemoDirectories);

test("a word of the operator stands as his messages stand, as its program in a panel of his color: its line that waits is lit until its act ends, then says what the act came to, in each view and in a later life, and a word that the gate refused shows a cross and its finding", async () => {
  let record: string | undefined;
  const line = (shown: string) => shown.split("\n").find((row) => row.includes("await wait(60)")) ?? "";
  await composing(
    async ({ session, app, frame }) => {
      const rung = await session.engine.rung({ word: "await wait(60)", on: session.engine.root });
      await session.refresh();
      const running = await frame();
      expect(running).not.toContain("You ran");
      expect(line(running)).toMatch(/1 await wait\(60\) +◉ # waits 60s/);
      expect(app.scroll.getChildren().some((card) => card.id === rung)).toBe(true);
      expect(line(running)).toMatch(/^ +┃ /);
      const wait = session.acts.find((act) => act.kind === "wait" && act.by === rung);
      if (!wait) throw new Error("No pending wait.");
      await session.engine.close(null, { id: wait.id });
      await session.engine.result(rung);
      await session.refresh();
      const done = line(await frame());
      expect(done).not.toContain("◉");
      expect(done).toMatch(/ 1 await wait\(60\) +# woke/);
      session.show("transcript");
      await session.refresh();
      await frame();
      session.show("feed");
      await session.refresh();
      expect(line(await frame())).toBe(done);
      await session.submit("/run this is invalid python !!!");
      const refused = session.activity.findLast(
        (act) => act.kind === "rung" && act.words[0] === "this is invalid python !!!",
      );
      expect(refused?.run?.status).toBe("failed");
      expect(session.error).toBe("");
      expect(session.findings.join("\n")).toContain("line 1");
      await session.refresh();
      const rows = (await frame()).split("\n");
      const at = rows.findIndex((row) => /1 this is invalid python !!! +✗/.test(row));
      expect(at).toBeGreaterThan(0);
      expect(rows[at + 1]).toMatch(/^ +┃ +# \S/);
      record = session.host.record;
    },
    { width: 140, height: 42, useMouse: true },
  );
  if (!record) throw new Error("No record.");
  const session = await demoSession({ record });
  await composing(
    async ({ frame }) => {
      const shown = line(await frame());
      expect(shown).not.toContain("◉");
      expect(shown).toMatch(/ 1 await wait\(60\) +# woke/);
    },
    { width: 140, height: 42, useMouse: true },
    session,
  );
});

test("queued follow-ups wait for current work, attach files, and message undo and redo keep their branches", async () => {
  let session = await demoSession();
  try {
    await session.submit("show live progress");
    session.enqueue("Explain @README.md after this answer.");
    expect(session.queued).toHaveLength(1);
    expect(
      session.acts.some((act) => act.kind === "prompt" && String(act.words[1]).startsWith("Explain")),
    ).toBe(false);
    await until(
      session,
      () =>
        session.queued.length === 0 &&
        session.acts.some(
          (act) =>
            session.isUserPrompt(act) && act.words[1] === "Explain @README.md after this answer." && act.done,
        ),
    );
    await idle(session);
    await session.refresh();
    expect(session.acts.filter((act) => session.isUserPrompt(act)).map((act) => act.words[1])).toEqual([
      "show live progress",
      "Explain @README.md after this answer.",
    ]);
    const source = session.selected;
    await session.undo();
    expect(session.selected).not.toBe(source);
    expect((await transcriptOf(session.engine, session.selected)).join("\n")).not.toContain(
      "Explain @README.md after this answer.",
    );
    await session.submit("/redo");
    expect(session.selected).toBe(source);
    expect((await transcriptOf(session.engine, source)).join("\n")).toContain(
      "Explain @README.md after this answer.",
    );
    await session.submit("/pause");
    session.enqueue("Keep this follow-up through exit.");
    session.save();
    const saved = JSON.parse(await readFile(`${session.host.record}.ui.json`, "utf8"));
    expect(saved.queued[0].text).toBe("Keep this follow-up through exit.");
    expect(session.acts.some((act) => act.kind === "prompt" && act.words[1] === saved.queued[0].text)).toBe(
      false,
    );
    const record = session.host.record;
    if (!record) throw new Error("No record for the queued message.");
    await session.dispose();
    session = await demoSession({ record });
    expect(session.queueHeld).toBe(true);
    expect(session.queued[0]?.text).toBe("Keep this follow-up through exit.");
    await session.submit("/wake");
    session.queueHeld = false;
    await session.drainQueue();
    await until(session, () =>
      session.acts.some(
        (act) =>
          session.isUserPrompt(act) && act.words[1] === "Keep this follow-up through exit." && act.done,
      ),
    );
    await idle(session);
    await session.refresh();
    const sent = session.acts.filter((act) => session.isUserPrompt(act)).length;
    await session.dispose();
    const stale = JSON.parse(await readFile(`${record}.ui.json`, "utf8"));
    stale.queued = saved.queued;
    await writeFile(`${record}.ui.json`, JSON.stringify(stale));
    session = await demoSession({ record });
    expect(session.queued).toHaveLength(0);
    expect(session.acts.filter((act) => session.isUserPrompt(act))).toHaveLength(sent);
  } finally {
    await session.dispose();
  }
});

test("clipboard import and share publication use bounded files and the explicitly chosen uploader", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-desktop-"));
  const bin = join(directory, "bin");
  await mkdir(bin);
  const oldPath = process.env.PATH;
  const data = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/l9sAAAAASUVORK5CYII=";
  // One program under the name of each system's reader of the clipboard, and under the name gh.
  await writeFile(join(bin, "clipboard"), data);
  const reader = executable(join(programs, "clipboard.ts"), directory, "reader");
  for (const name of ["osascript", "powershell", "wl-paste", "xclip"])
    await link(reader, join(bin, `${name}${extname(reader)}`));
  const log = join(directory, "uploaded.json");
  const gh = executable(join(programs, "clipboard.ts"), bin, "gh");

  try {
    process.env.PATH = `${bin}${delimiter}${oldPath ?? ""}`;
    expect(Bun.which("gh", { PATH: process.env.PATH })).toBe(gh);
    let temporary = "";
    const bytes = await clipboardImage(async (path) => {
      temporary = path;
      return await readFile(path);
    });
    expect(bytes.toString("base64")).toBe(data);
    expect(await stat(temporary).catch(() => null)).toBeNull();
    const html = join(directory, "export.html");
    await writeFile(html, "<p>Conversation</p>");
    expect(await publishShare(html, "# Conversation", "Test conversation")).toBe(
      "https://gist.github.com/furb/test-link",
    );
    const uploaded = JSON.parse(await readFile(log, "utf8"));
    expect(uploaded.args.slice(0, 4)).toEqual(["gist", "create", "--desc", "Test conversation"]);
    expect(uploaded.args).not.toContain("--public");
    expect(uploaded.markdown).toBe("# Conversation");
    expect(uploaded.html).toBe("<p>Conversation</p>");
  } finally {
    if (oldPath === undefined) delete process.env.PATH;
    else process.env.PATH = oldPath;
    await remove(directory);
  }
});

/** A claude command line in the directory that ends with a failure as it starts. */
function failing(directory: string): string {
  return executable(join(programs, "failing.ts"), directory, "claude");
}

test("a model request failure shows in the feed as the failure of an act, and not as an error of the view", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-model-failure-"));
  try {
    const opened = await openEngine({
      cwd: directory,
      record: join(directory, "session.jsonl"),
      // A claude command line that fails at once fails every request, as a model that is not there does. A program
      // that is missing would be tried again for a minute, as a command line that updates itself is.
      claude: failing(directory),
      roster: ["claude-cli:sonnet"],
    });
    await composing(
      async ({ session, frame }) => {
        await session.submit("Say something.");
        // The chain pauses at the second failure in a row, and no work runs after that.
        await until(session, () => session.paused);
        await session.refresh();
        const shown = await frame();
        expect(shown).not.toContain("Refresh view");
        expect(shown).toContain("answered nothing");
      },
      { width: 140, height: 42 },
      new Session(opened.engine, opened.host, true),
    );
  } finally {
    await remove(directory);
  }
});

test("an App ends with its renderer while a model request fails and a redraw waits, and draws on no freed buffer", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-teardown-"));
  try {
    const opened = await openEngine({
      cwd: directory,
      record: join(directory, "session.jsonl"),
      claude: failing(directory),
      roster: ["claude-cli:sonnet"],
    });
    await composing(
      async ({ session, app, screen }) => {
        // A pipe that a model command line closes early sends a signal that ends no test terminal.
        expect(process.listenerCount("SIGPIPE")).toBe(0);
        await session.submit("Say something.");
        app.composer.setText("a draft to keep");
        session.notice = "A redraw waits.";
        expect(Reflect.get(app, "redraw")).toBeDefined();
        // The renderer ends on its own, as on a signal: the App ends first, and keeps the draft while it can.
        screen.renderer.destroy();
        expect(Reflect.get(app, "redraw")).toBeUndefined();
        expect(session.drafts[session.draftKey]).toBe("a draft to keep");
        await until(session, () => session.paused);
        await Bun.sleep(100);
        app.render();
        app.dispose();
      },
      { width: 140, height: 42 },
      new Session(opened.engine, opened.host, true),
    );
  } finally {
    await remove(directory);
  }
});

test("file and shell shortcuts, an external editor, and a safe standalone share use the real session", async () => {
  const session = await demoSession();
  const oldEditor = process.env.EDITOR,
    oldVisual = process.env.VISUAL;
  try {
    const directory = session.host.directory;
    await writeFile(join(directory, "review notes.txt"), "Unique context for this check.");
    expect(await projectFiles(directory)).toContain("review notes.txt");
    expect(await fileReferences('Read @"review notes.txt" and @README.md', directory)).toEqual([
      "review notes.txt",
      "README.md",
    ]);
    await session.submit('Read @"review notes.txt".');
    await idle(session);
    await session.refresh();
    expect(JSON.stringify(session.turns)).toContain("Unique context for this check.");
    await session.submit("!printf shell-shortcut");
    const shell = session.activity.findLast(
      (act) => act.kind === "bash" && act.words[0] === "printf shell-shortcut",
    );
    if (!shell) throw new Error("No shell shortcut act.");
    expect(await session.engine.result(shell.id)).toMatchObject({
      code: 0,
      stdout: { content: "shell-shortcut" },
    });
    const editor = join(directory, "editor.sh");
    await writeFile(editor, "#!/bin/sh\nprintf 'edited outside the TUI' > \"$1\"\n", { mode: 0o700 });
    process.env.EDITOR = `/bin/sh '${editor.replaceAll("'", "'\\''")}'`;
    delete process.env.VISUAL;
    const transitions: string[] = [];
    expect(
      await externalEditor(
        {
          suspend: () => {
            transitions.push("suspend");
          },
          resume: () => {
            transitions.push("resume");
          },
        },
        "old draft",
        false,
        directory,
      ),
    ).toBe("edited outside the TUI");
    expect(transitions).toEqual(["suspend", "resume"]);
    await session.command("/extensions");
    expect(session.notice).toBe("This life runs no extension.");
    session.sessionName = '<script>alert("name")</script>';
    await session.submit('<script>alert("message")</script> [bad link](javascript:alert(1))');
    await idle(session);
    await session.refresh();
    const html = shareHtml(session);
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>");
    expect(html).toContain("Content-Security-Policy");
    expect(html).not.toContain('href="javascript:');
    expect(html).toContain("<strong>search shortcut</strong>");
    expect(shareMarkdown(session)).toContain("Unique context");
    const output = join(directory, "shared.html");
    await session.command(`/share ${output}`);
    expect(await readFile(output, "utf8")).toContain("Exact model transcript");
  } finally {
    if (oldEditor === undefined) delete process.env.EDITOR;
    else process.env.EDITOR = oldEditor;
    if (oldVisual === undefined) delete process.env.VISUAL;
    else process.env.VISUAL = oldVisual;
    await session.dispose();
  }
});

test("/extensions lists what a life runs, and a cancel of a word of the operator leaves the watcher of its memory", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-extensions-"));
  try {
    const opened = await openEngine({ cwd: directory, record: join(directory, "session.jsonl") });
    await composing(
      async ({ session, frame, click }) => {
        await session.command("/extensions");
        expect(session.notice).toBe(
          "This life runs catchup with catchup(), memory with await recall(), skills with await listed(), compaction, ouroboros, web, mcp with await mcp_index(). Run a word of one with /run.",
        );
        await session.command("/run await wait(60)");
        // The wait runs under the word, so the cancel asks first.
        await until(session, () => session.activity.some((act) => act.kind === "wait" && !act.done));
        await session.command("/cancel");
        await click("Cancel it");
        await until(session, () => session.activity.every((act) => act.kind !== "wait" || act.done));
        await session.refresh();
        // The watcher lives on, and shows no work in the feed.
        expect((await session.engine.outcome("remember1")).done).toBe(false);
        expect(await frame()).not.toContain("running");
      },
      { width: 140, height: 42 },
      new Session(opened.engine, opened.host, true),
    );
  } finally {
    await remove(directory);
  }
});

test("the words of the extensions stand as code: a comment that names their ear, then the call that ends each word, with no ids of acts", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-ear-code-"));
  try {
    await writeFile(join(directory, "a.txt"), "one");
    const opened = await openEngine({ cwd: directory, record: join(directory, "session.jsonl") });
    await composing(
      async ({ session, frame, click }) => {
        await session.command("/run x = 1");
        await until(session, () =>
          session.activity.some((act) => act.kind === "rung" && act.by === "operator" && act.done),
        );
        await session.refresh();
        // The setup of the root is the turns of the default actor that the ear extensions wrote: the read of the
        // doctrine, the read of the file of each extension in the order of the life, the life words in one word, then
        // the stand and the cd.
        const setup = session.activity.filter((act) => act.kind === "rung" && act.by === "extensions");
        expect(setup.map((act) => act.word)).toEqual([
          // A path of Windows stands with its separators doubled, as python shows a backslash.
          expect.stringMatching(/^read\('.*[/\\]DOCTRINE\.md'\)$/),
          ...["catchup", "memory", "skills", "compaction", "ouroboros", "web", "mcp"].map((name) =>
            expect.stringMatching(
              new RegExp(`^read\\('.*[/\\\\]+extensions[/\\\\]+${name}[/\\\\]+${name}\\.py'\\)$`),
            ),
          ),
          "catchup()\nawait recall()\nawait listed()\nawait mcp_index()",
          expect.stringMatching(/^stand\(\)\ncd\('.*'\)$/),
        ]);
        let rows = (await frame()).split("\n");
        for (let step = 0; step < 80 && !rows.some((row) => /earlier steps/.test(row)); step++) {
          await Bun.sleep(25);
          rows = (await frame()).split("\n");
        }
        // The card of the ear holds more steps than a card shows, so its first steps and its comment fold.
        await click(rows.join("\n").match(/\d+ earlier steps/)?.[0] ?? "earlier steps");
        for (let step = 0; step < 80 && !rows.some((row) => /# extensions/.test(row)); step++) {
          await Bun.sleep(25);
          rows = (await frame()).split("\n");
        }
        const at = rows.findIndex((row) => /# extensions/.test(row));
        expect(at).toBeGreaterThanOrEqual(0);
        const shown = rows.slice(at + 1, at + 15).map((row) => row.trim().replace(/\s{2,}.*$/, ""));
        expect(shown.slice(0, 8).map((row) => /^\d+ read\('/.test(row))).toEqual(Array(8).fill(true));
        // The life words stand as the calls that end their one word, each on its line, and the stand and the cd come
        // after them.
        expect(shown.slice(8, 12)).toEqual([
          "9 catchup()",
          "10 await recall()",
          "11 await listed()",
          "12 await mcp_index()",
        ]);
        expect([shown[12], shown[13]?.startsWith("14 cd('")]).toEqual(["13 stand()", true]);
        expect(rows.join("\n")).not.toMatch(/rung\d+…\d+|remember1|ouroboros1|catchup1/);
      },
      { width: 140, height: 42 },
      new Session(opened.engine, opened.host, true),
    );
  } finally {
    await remove(directory);
  }
});

test("an act that ended and told nothing shows nothing in the feed, as a question of memory that found no file", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-untold-"));
  try {
    await writeFile(join(directory, "a.txt"), "one");
    const opened = await openEngine({ cwd: directory, record: join(directory, "session.jsonl") });
    await composing(
      async ({ session, frame }) => {
        await session.command('/run read("a.txt")');
        await until(session, () =>
          session.activity.some(
            (act) => act.kind === "memory" && String(act.words[0]).endsWith("a.txt") && act.done,
          ),
        );
        await session.refresh();
        const shown = await frame();
        expect(shown).toContain('read("a.txt")');
        // The word of the rung that defines memory still shows, but no act named memory does.
        expect(shown).not.toContain("✓ memory");
      },
      { width: 140, height: 42 },
      new Session(opened.engine, opened.host, true),
    );
  } finally {
    await remove(directory);
  }
});

test("a queued dispatch recovers both sides of the prompt-write boundary without sending twice", async () => {
  let session = await demoSession();
  try {
    const record = session.host.record;
    if (!record) throw new Error("No record.");
    const entry = {
      id: "recovery-check",
      chain: session.selected,
      shape: "str",
      text: "A queued question",
    };
    session.queued = [entry];
    session.queueHeld = true;
    session.save();
    const sent = await session.host.sendQueued(entry);
    await session.refresh();
    await session.dispose();
    const lines = (await readFile(record, "utf8")).trimEnd().split("\n");
    const begin = lines.findIndex(
      (line) => JSON.parse(line)[0][0] === "queue" && JSON.parse(line)[0][3] === "begin",
    );
    expect(JSON.parse(lines[begin + 1] ?? "null")[0][0]).toBe("prompt");
    const state = JSON.parse(await readFile(`${record}.ui.json`, "utf8"));
    state.queued = [entry];
    await writeFile(`${record}.ui.json`, JSON.stringify(state));
    await writeFile(
      record,
      `${lines.filter((line) => !(JSON.parse(line)[0][0] === "queue" && JSON.parse(line)[0][3] === "sent")).join("\n")}\n`,
    );
    session = await demoSession({ record });
    expect(session.queued).toHaveLength(0);
    expect(session.dispatched).toContain(entry.id);
    expect(session.acts.filter((act) => act.kind === "prompt")).toHaveLength(1);
    expect(session.acts.some((act) => act.id === sent)).toBe(true);
    await session.dispose();
    await writeFile(record, `${lines.slice(0, begin + 1).join("\n")}\n`);
    await writeFile(`${record}.ui.json`, JSON.stringify(state));
    session = await demoSession({ record });
    expect(session.queued).toHaveLength(1);
    const manual = await session.engine.prompt(entry.shape, {
      message: entry.text,
      on: entry.chain,
    });
    await session.refresh();
    expect(session.queued).toHaveLength(1);
    const dispatched = await session.host.sendQueued(entry);
    expect(dispatched).not.toBe(manual);
    expect(await session.host.sendQueued(entry)).toBe(dispatched);
    await session.refresh();
    expect(session.acts.filter((act) => act.kind === "prompt")).toHaveLength(2);
  } finally {
    await session.dispose();
  }
});

test("a file opens with the command of the system it runs on", () => {
  const path = "C:\\Users\\me\\a picture.png";
  expect(opener("darwin", path)).toEqual(["open", path]);
  expect(opener("linux", path)).toEqual(["xdg-open", path]);
  expect(opener("freebsd", path)).toEqual(["xdg-open", path]);
});

test.skipIf(process.platform !== "win32")("a browser URL reaches Windows with its query intact", async () => {
  const url = "https://example.invalid/authorize?code=a&state=one's%20value&next=$(ignored)";
  const command = opener("win32", url);
  const body = Buffer.from(command.at(-1) ?? "", "base64").toString("utf16le");
  const capture =
    "function Start-Process { param([string]$FilePath) ConvertTo-Json -InputObject $FilePath -Compress }; ";
  const child = Bun.spawn(
    [...command.slice(0, -1), Buffer.from(capture + body, "utf16le").toString("base64")],
    { stdout: "pipe", stderr: "pipe" },
  );
  const output = await new Response(child.stdout).text();
  expect(await child.exited).toBe(0);
  expect(JSON.parse(output)).toBe(url);
});

test("/wake with no chain wakes every paused chain of the life, and /wake with a chain wakes that one alone", () =>
  composing(async ({ session }) => {
    const first = session.selected;
    await session.submit("/pause");
    await session.submit("/chain second");
    const second = session.selected;
    await session.submit("/pause");
    const stopped = () =>
      session.chains.filter((chain) => [first, second].includes(chain.id) && chain.paused);
    await until(session, () => stopped().length === 2);
    await session.submit(`/wake ${first}`);
    await until(session, () => stopped().length === 1);
    expect(stopped()[0]?.id).toBe(second);
    await session.submit(`/pause ${first}`);
    await until(session, () => stopped().length === 2);
    await session.submit("/wake");
    await until(session, () => stopped().length === 0);
    expect(session.notice).toBe("Work resumed on 2 chains.");
  }));
