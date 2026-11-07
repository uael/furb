import { afterAll, expect, test } from "bun:test";
import { link, mkdir, mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { delimiter, extname, join } from "node:path";
import { isDone, isPaused, operated } from "furb/life";
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
  const line = (shown: string) =>
    shown.split("\n").find((row) => row.includes("await wait(60, __name__)")) ?? "";
  await composing(
    async ({ session, app, frame }) => {
      const rung = await session.engine.rung({ word: "await wait(60, __name__)", on: session.engine.root });
      // The daemon runs the word after the verb made its rung, so the window hears of the wait that the line awaits
      // only then: the line says what it waits for once the wait stands.
      await until(session, () => session.acts.some((act) => act.kind === "wait" && act.by === rung));
      await session.refresh();
      const running = await frame();
      expect(running).not.toContain("You ran");
      expect(line(running)).toMatch(/(?:\d|[◐◓◑◒◉]) +await wait\(60, __name__\) +waits 60s/);
      expect(app.scroll.getChildren().some((card) => card.id === rung)).toBe(true);
      expect(line(running)).toMatch(/^ +┃ /);
      const wait = session.acts.find((act) => act.kind === "wait" && act.by === rung);
      if (!wait) throw new Error("No pending wait.");
      await session.engine.close("null", { id: wait.id });
      await session.engine.result(rung);
      await session.refresh();
      const done = line(await frame());
      expect(done).not.toContain("◉");
      expect(done).toMatch(/ 1 {4}await wait\(60, __name__\) +woke/);
      session.show("transcript");
      await session.refresh();
      await frame();
      session.show("feed");
      await session.refresh();
      expect(line(await frame())).toBe(done);
      await session.submit("/run this is invalid python !!!");
      const refused = () =>
        session.activity.findLast(
          (act) => act.kind === "rung" && act.words[0] === "this is invalid python !!!",
        );
      // The daemon gates the word after the submit made its rung, and the window hears the refusal then.
      await until(session, () => refused()?.run?.run === "refused" && session.findings.length > 0);
      expect(session.error).toBe("");
      expect(session.findings.join("\n")).toContain("line 1");
      await session.refresh();
      const rows = (await frame()).split("\n");
      const at = rows.findIndex((row) => /1 {2}✗ this is invalid python !!!/.test(row));
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
      expect(shown).toMatch(/ 1 {4}await wait\(60, __name__\) +woke/);
    },
    // The later life boots on the checkpoint that the end of the first one wrote, and the word of that boot holds the
    // setup of the life and its values, which stands under the words of the first life, so the frame is tall enough to
    // hold both.
    { width: 140, height: 200, useMouse: true },
    session,
  );
});

test("queued follow-ups wait for current work, attach files, and message undo and redo keep their branches", async () => {
  let session = await demoSession();
  try {
    await session.submit("show live progress");
    await session.enqueue("Explain @README.md after this answer.");
    expect(
      session.acts.some((act) => act.kind === "prompt" && String(act.words[1]).startsWith("Explain")),
    ).toBe(false);
    await until(
      session,
      () =>
        session.queued.length === 0 &&
        session.acts.some(
          (act) =>
            session.isUserPrompt(act) &&
            act.words[1] === "Explain @README.md after this answer." &&
            isDone(act),
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
    await session.enqueue("Keep this follow-up through exit.");
    await until(session, () => session.queued.length === 1);
    expect(
      session.acts.some((act) => act.kind === "prompt" && act.words[1] === session.queued[0]?.message),
    ).toBe(false);
    const record = session.host.record;
    if (!record) throw new Error("No record for the queued message.");
    await session.dispose();
    // The record keeps the queue, so the reopened life holds the follow-up until its chain wakes.
    session = await demoSession({ record });
    await until(session, () => session.queued[0]?.message === "Keep this follow-up through exit.");
    await session.submit("/wake");
    await until(session, () =>
      session.acts.some(
        (act) =>
          session.isUserPrompt(act) && act.words[1] === "Keep this follow-up through exit." && isDone(act),
      ),
    );
    await idle(session);
    await session.refresh();
    const resent = (await readFile(record, "utf8"))
      .split("\n")
      .filter((line) => line.startsWith('[["prompt",') && line.includes("Keep this follow-up through exit."));
    expect(resent).toHaveLength(1);
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
    await session.command("/skills");
    expect(session.notice).toBe("This life runs no skill.");
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

test("/skills lists what a life runs, and a cancel of a word of the operator leaves the watcher of its memory", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-extensions-"));
  try {
    const opened = await openEngine({ cwd: directory, record: join(directory, "session.jsonl") });
    await composing(
      async ({ session, frame, click }) => {
        await session.command("/skills");
        expect(session.notice).toBe(
          "This life runs catchup with catchup(), memory with await recall(), skills with await listed(), compaction with compacted(); fitted(), naming with name_session(). Run a word of one with /run.",
        );
        await session.command("/run await wait(60, __name__)");
        // The wait runs under the word, so the cancel asks first.
        await until(session, () => session.activity.some((act) => act.kind === "wait" && !isDone(act)));
        await session.command("/cancel");
        await click("Cancel it");
        await until(session, () => session.activity.every((act) => act.kind !== "wait" || isDone(act)));
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

test("the setup of the root stands as code: a comment that names the setup, then the call that ends each word, with no ids of acts", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-ear-code-"));
  try {
    await writeFile(join(directory, "a.txt"), "one");
    const opened = await openEngine({ cwd: directory, record: join(directory, "session.jsonl") });
    await composing(
      async ({ session, frame, click }) => {
        // The setup of the root ends with its last rung, the stand and the cd. A word of the operator that comes
        // sooner ends it, and that rung then stands after the word, as it does where the roster is slow to come.
        const stood = () =>
          session.activity.some(
            (act) =>
              act.kind === "rung" &&
              act.by === act.chain &&
              isDone(act) &&
              /^stand\(\)\ncd\(/.test(act.word ?? ""),
          );
        for (let step = 0; step < 200 && !stood(); step++) {
          await Bun.sleep(25);
          await session.refresh();
        }
        expect(stood()).toBe(true);
        await session.command("/run x = 1");
        await until(session, () =>
          session.activity.some((act) => act.kind === "rung" && operated(act.by) && isDone(act)),
        );
        await session.refresh();
        // The root writes its setup on itself, each rung to the one default actor and with no rung whose word it
        // retells: the debug of the extensions, the read of the doctrine, the word rung(read('<file>', HIDDEN).content) of each extension in the
        // order of the life, the life words in one word, then the stand and the cd.
        const setup = session.activity.filter((act) => act.kind === "rung" && act.by === act.chain);
        expect(setup.map((act) => act.chain)).toEqual(Array(setup.length).fill(session.engine.root));
        expect(setup.map((act) => act.words.slice(1))).toEqual(
          Array(setup.length).fill(["", setup[0]?.words[2]]),
        );
        const words = setup.map((act) => act.word ?? "");
        expect(words[0]).toBe('debug(t"{standing()[4]=}")');
        // A path of Windows stands with its separators doubled, as python shows a backslash.
        expect(words[1]).toMatch(/^read\('.*[/\\]DOCTRINE\.md'\)$/);
        expect(words.at(-1)).toMatch(/^stand\(\)\ncd\('.*'\)$/);
        // The word of each extension is read in the order of the life, an extension that plays on the root alone
        // after the others, and each life word stands as a rung of its own.
        const reads = words.filter((word) => /^rung\(read\('.*', HIDDEN\)\.content\)$/.test(word));
        expect(reads.map((word) => /[/\\]+words[/\\]+(\w+)[/\\]+/.exec(word)?.[1])).toEqual([
          "catchup",
          "memory",
          "skills",
          "compaction",
          "naming",
        ]);
        expect(words.slice(2, -1).filter((word) => !reads.includes(word))).toEqual([
          "catchup()",
          "await recall()",
          "await listed()",
          "compacted(); fitted()",
          "name_session()",
        ]);
        let rows = (await frame()).split("\n");
        for (let step = 0; step < 80 && !rows.some((row) => /earlier steps/.test(row)); step++) {
          await Bun.sleep(25);
          rows = (await frame()).split("\n");
        }
        // The card of the setup holds more steps than a card shows, so its first steps and its comment fold.
        await click(rows.join("\n").match(/\d+ earlier steps/)?.[0] ?? "earlier steps");
        for (let step = 0; step < 80 && !rows.some((row) => /# setup/.test(row)); step++) {
          await Bun.sleep(25);
          rows = (await frame()).split("\n");
        }
        const at = rows.findIndex((row) => /# setup/.test(row));
        expect(at).toBeGreaterThanOrEqual(0);
        expect(rows[at]?.trim().replace(/\s{2,}.*$/, "")).toBe("# setup");
        // Each word stands in a block under its label, so the lines of the setup are read past the labels and the blank
        // rows between the blocks.
        const shown = rows
          .slice(at + 1)
          .map((row) =>
            row
              .slice(0, 80)
              .trim()
              .replace(/^(\d+) {4}/, "$1 ")
              .replace(/\s{2,}.*$/, ""),
          )
          .filter((row) => /^\d+ /.test(row))
          .slice(0, 18);
        expect(shown[0]).toBe('1 debug(t"{standing()[4]=}")');
        // The path of the doctrine is cut to the width of the card, so only its call shows whole.
        expect(shown[1]).toMatch(/^2 read\('.*'\)$/);
        // A word whose file path is long breaks into more lines, as ruff format lays it out, so a path of a longer checkout
        // splits the word of each extension: the test reads the call that each line starts, not the width of the path.
        expect(shown.slice(2, 6).map((row) => /^\d+ rung\(/.test(row))).toEqual(Array(4).fill(true));
        // The life words stand as the calls that end their one word, each on its line, then the stand and the cd,
        // and the card ends there.
        expect(shown.slice(6, 10)).toEqual([
          "7 catchup()",
          "8 await recall()",
          "9 await listed()",
          "10 compacted(); fitted()",
        ]);
        // The extension naming plays on the root alone, so its word and its life word come after the others.
        expect(shown[10]).toMatch(/^11 rung\(/);
        expect(shown.slice(11, 13)).toEqual(["12 name_session()", "13 stand()"]);
        expect(shown[13]).toMatch(/^14 cd\('.*'\)$/);
        expect(shown[14] ?? "").not.toMatch(/^15 /);
        expect(rows.join("\n")).not.toMatch(/rung\d+…\d+|remember1|ouroboros1|catchup1/);
      },
      { width: 140, height: 90 },
      new Session(opened.engine, opened.host, true),
    );
  } finally {
    await remove(directory);
  }
});

test("an act that ended and told nothing shows nothing in the feed, as a read of a folder that memory made", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-untold-"));
  try {
    await writeFile(join(directory, "a.txt"), "one");
    const opened = await openEngine({ cwd: directory, record: join(directory, "session.jsonl") });
    await composing(
      async ({ session, frame }) => {
        await session.command('/run read("a.txt")');
        await until(session, () =>
          session.activity.some(
            (act) => act.kind === "read" && String(act.words[0]).endsWith("/") && isDone(act),
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
      session.chains.filter((chain) => [first, second].includes(chain.id) && isPaused(chain));
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
