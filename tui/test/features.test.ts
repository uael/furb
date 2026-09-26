import { afterAll, expect, test } from "bun:test";
import { link, mkdir, mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { delimiter, extname, join } from "node:path";
import { executable } from "../../bind/typescript/test/executable.ts";
import { remove } from "../../bind/typescript/test/processes.ts";
import { until } from "../../bind/typescript/test/until.ts";
import { openEngine } from "../src/bridge.ts";
import { clipboardImage } from "../src/clipboard.ts";
import { demoSession, removeDemoDirectories } from "../src/demo.ts";
import { externalEditor, opener } from "../src/editor.ts";
import { Extensions } from "../src/extensions.ts";
import { fileReferences, projectFiles } from "../src/files.ts";
import { Session } from "../src/session.ts";
import { publishShare, shareHtml, shareMarkdown } from "../src/share.ts";
import { composing } from "./composing.ts";
import { idle } from "./idle.ts";
import { transcriptOf } from "./transcript.ts";

afterAll(removeDemoDirectories);

test("rungs retain clicked folds across views and reopen, with running, failed, and done labels and gate findings", async () => {
  let rung = "";
  let record: string | undefined;
  await composing(
    async ({ session, app, screen }) => {
      rung = await session.engine.rung({ word: "await wait(60)", on: session.engine.root });
      await session.refresh();
      app.render();
      await screen.flush();
      let card = app.scroll.getChildren().find((card) => card.id === rung);
      let heading = card?.getChildren()[0];
      if (!heading) throw new Error("No rung header.");
      expect(screen.captureCharFrame()).toContain(`${rung}  by you  running`);
      expect(card?.getChildren().length).toBeGreaterThan(1);
      await screen.mockMouse.click(heading.x + 1, heading.y);
      app.render();
      await screen.flush();
      expect(
        app.scroll
          .getChildren()
          .find((card) => card.id === rung)
          ?.getChildren(),
      ).toHaveLength(1);
      const wait = session.acts.find((act) => act.kind === "wait" && act.by === rung);
      if (!wait) throw new Error("No pending wait.");
      await session.engine.close(null, { id: wait.id });
      await session.engine.result(rung);
      await session.refresh();
      app.render();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain(`✓ ${rung}`);
      expect(screen.captureCharFrame()).not.toContain(`${rung}  by you  running`);
      expect(
        app.scroll
          .getChildren()
          .find((card) => card.id === rung)
          ?.getChildren(),
      ).toHaveLength(1);
      session.show("transcript");
      await session.refresh();
      app.render();
      await screen.flush();
      session.show("feed");
      await session.refresh();
      app.render();
      await screen.flush();
      card = app.scroll.getChildren().find((card) => card.id === rung);
      expect(card?.getChildren()).toHaveLength(1);
      heading = card?.getChildren()[0];
      if (heading) await screen.mockMouse.click(heading.x + 1, heading.y);
      app.render();
      await screen.flush();
      expect(
        app.scroll
          .getChildren()
          .find((card) => card.id === rung)
          ?.getChildren().length,
      ).toBeGreaterThan(1);
      await session.submit("/run this is invalid python !!!");
      const refused = session.activity.findLast(
        (act) => act.kind === "rung" && act.words[0] === "this is invalid python !!!",
      );
      expect(refused?.run?.status).toBe("failed");
      await session.refresh();
      app.render();
      await screen.flush();
      expect(screen.captureCharFrame()).toContain("line 1");
      expect(screen.captureCharFrame()).toContain("failed");
      record = session.host.record;
    },
    { width: 140, height: 42, useMouse: true },
  );
  if (!record) throw new Error("No record.");
  const reopened = await openEngine({ record, demo: true });
  const session = new Session(reopened.engine, reopened.host, true);
  await session.refresh();
  await composing(
    async ({ app }) => {
      expect(session.folds[rung]).toBe(false);
      expect(session.acts.find((act) => act.id === rung)?.run?.status).toBe("done");
      expect(
        app.scroll
          .getChildren()
          .find((card) => card.id === rung)
          ?.getChildren().length,
      ).toBeGreaterThan(1);
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
    let reopened = await openEngine({ record, demo: true });
    session = new Session(reopened.engine, reopened.host, true);
    await session.refresh();
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
    reopened = await openEngine({ record, demo: true });
    session = new Session(reopened.engine, reopened.host, true);
    await session.refresh();
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
  // One reader of the clipboard under the name of each system's reader: AppKit and PowerShell print base64.
  await writeFile(
    join(directory, "clipboard.ts"),
    `import { basename } from "node:path"; const data = ${JSON.stringify(data)}; process.stdout.write(/^(osascript|powershell)(\\.exe)?$/.test(basename(process.execPath)) ? data : Buffer.from(data, "base64"));\n`,
  );
  const reader = executable(join(directory, "clipboard.ts"), directory, "reader");
  for (const name of ["osascript", "powershell", "wl-paste", "xclip"])
    await link(reader, join(bin, `${name}${extname(reader)}`));
  const log = join(directory, "uploaded.json");
  await writeFile(
    join(directory, "gh.ts"),
    `import { readFileSync, writeFileSync } from "node:fs"; const args = process.argv.slice(2); writeFileSync(${JSON.stringify(log)}, JSON.stringify({ args, markdown: readFileSync(args.at(-2), "utf8"), html: readFileSync(args.at(-1), "utf8") })); console.log("https://gist.github.com/furb/test-link");\n`,
  );
  const gh = executable(join(directory, "gh.ts"), bin, "gh");

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

test("a model request failure shows in the feed as the failure of an act, and not as an error of the view", async () => {
  const directory = await mkdtemp(join(tmpdir(), "furb-model-failure-"));
  try {
    const opened = await openEngine({
      cwd: directory,
      record: join(directory, "session.jsonl"),
      claude: {
        bin: executable(
          join(import.meta.dir, "../../bind/typescript/test/fake-claude.ts"),
          directory,
          "claude",
        ),
      },
    });
    await composing(
      async ({ session, frame }) => {
        await session.submit("FAIL");
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

test("file and shell shortcuts, an external editor, extensions, and a safe standalone share use the real session", async () => {
  const session = await demoSession();
  const oldEditor = process.env.EDITOR,
    oldVisual = process.env.VISUAL;
  const extensions = new Extensions(() => session);
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
    const extension = join(directory, "extension.ts");
    await writeFile(
      extension,
      'export default (api) => { api.registerCommand("test-extension", { label: "Test extension", description: "Bind a value", async run(_, ctx) { await ctx.engine.result(await ctx.engine.rung({ word: "extension_value = 23", on: ctx.chain })); ctx.notify("Extension finished."); } }); };',
    );
    await extensions.load(extension);
    await extensions.run("test-extension", "");
    expect((await session.engine.inspect("extension_value", session.selected)).value).toBe(23);
    expect(session.notice).toBe("Extension finished.");
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
    await extensions.dispose();
    await session.dispose();
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
      actor: "operator",
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
    let opened = await openEngine({ record, demo: true });
    session = new Session(opened.engine, opened.host, true);
    await session.refresh();
    expect(session.queued).toHaveLength(0);
    expect(session.dispatched).toContain(entry.id);
    expect(session.acts.filter((act) => act.kind === "prompt")).toHaveLength(1);
    expect(session.acts.some((act) => act.id === sent)).toBe(true);
    await session.dispose();
    await writeFile(record, `${lines.slice(0, begin + 1).join("\n")}\n`);
    await writeFile(`${record}.ui.json`, JSON.stringify(state));
    opened = await openEngine({ record, demo: true });
    session = new Session(opened.engine, opened.host, true);
    await session.refresh();
    expect(session.queued).toHaveLength(1);
    const manual = await session.engine.prompt(entry.shape, {
      message: entry.text,
      on: entry.chain,
      to: entry.actor,
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
  expect(opener("win32", path)).toEqual(["cmd", "/c", "start", "", path]);
  expect(opener("linux", path)).toEqual(["xdg-open", path]);
  expect(opener("freebsd", path)).toEqual(["xdg-open", path]);
});
