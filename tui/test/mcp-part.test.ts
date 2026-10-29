import { afterAll, expect, test } from "bun:test";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { demoSession } from "../src/demo.ts";
import { composing, pressEscape } from "./composing.ts";

const made: string[] = [];
afterAll(() => {
  for (const one of made) rmSync(one, { recursive: true, force: true });
});

/** The part of the extension mcp, and a word that answers its acts as the ear of mcp answers them, from two servers. */
const PART = resolve(import.meta.dir, "..", "..", "extensions", "mcp", "tui.ts");
const fixture = resolve(import.meta.dir, "..", "fixtures", "mcp");
const WORD = readFileSync(join(fixture, "servers.py"), "utf8").replace(
  /^from furb\.engine import .*\n+/m,
  "",
);

/** A project whose config turns on an extension with the part of mcp and the word of the fixture. */
function project(): string {
  const directory = mkdtempSync(join(tmpdir(), "furb-mcp-"));
  made.push(directory);
  const folder = join(directory, ".furb", "extensions", "servers");
  mkdirSync(folder, { recursive: true });
  cpSync(fixture, folder, { recursive: true });
  cpSync(PART, join(folder, "tui.ts"));
  writeFileSync(
    join(directory, ".furb", "config.json"),
    '{"extensions": {"servers": "extensions/servers"}}\n',
  );
  return directory;
}

/** The frame of the App once it shows a text, and once `ready` holds of it, drawn again until it does. */
async function shows(
  frame: () => Promise<string>,
  text: string,
  ready: (shown: string) => boolean = () => true,
): Promise<string> {
  let shown = await frame();
  for (let tries = 0; tries < 400 && !(shown.includes(text) && ready(shown)); tries++) {
    await Bun.sleep(25);
    shown = await frame();
  }
  expect(shown).toContain(text);
  return shown;
}

/** Waits until a test of the life holds. */
async function until(holds: () => boolean): Promise<void> {
  for (let tries = 0; tries < 400 && !holds(); tries++) await Bun.sleep(25);
  expect(holds()).toBe(true);
}

test("the part of mcp says the servers in the footer, and /mcp opens their dialog, whose keys act on a server through a word", async () => {
  const session = await demoSession({ cwd: project() });
  await composing(
    async ({ app, frame, screen }) => {
      await until(() => app.parts.loaded.some((one) => one.name === "servers"));
      app.composer.setText("/mcp");
      await app.submit();
      await until(() => app.session.notice.startsWith("No MCP server is configured"));
      expect(await frame()).not.toContain("MCP: ");
      await session.command(`/run ${WORD}\nmcp_servers()`);
      await shows(frame, "MCP: 2 servers, 1 failed");
      app.composer.setText("/mcp");
      await app.submit();
      let shown = await shows(frame, "MCP servers");
      expect(shown).toContain("connected · project · stdio · 1 tool");
      expect(shown).toContain("npx github-mcp");
      expect(shown).toContain("version 1.2.0");
      expect(shown).not.toContain("spawn slack-mcp ENOENT");
      screen.mockInput.pressArrow("down");
      shown = await shows(frame, "spawn slack-mcp ENOENT");
      screen.mockInput.pressKey("r");
      await shows(frame, "MCP: slack is connected.");
      await shows(frame, "MCP servers", (now) => !now.includes("failed"));
      const control = session.acts.filter((act) => act.kind === "mcp_control");
      expect(control.map((act) => act.words)).toEqual([["reconnect", "slack"]]);
      await pressEscape(screen);
    },
    { width: 180, height: 50 },
    session,
  );
});

test("the menu of a server lists its tools, each with its signature, and the parameters of a tool", async () => {
  const session = await demoSession({ cwd: project() });
  await composing(
    async ({ app, frame, screen }) => {
      await session.command(`/run ${WORD}\nmcp_servers()`);
      await shows(frame, "MCP: 2 servers, 1 failed");
      app.composer.setText("/mcp");
      await app.submit();
      await shows(frame, "MCP servers");
      screen.mockInput.pressEnter();
      await shows(frame, "github: connected");
      await screen.mockInput.typeText("Tools");
      await shows(frame, "1 tool");
      screen.mockInput.pressEnter();
      let shown = await shows(frame, "Tools of github");
      expect(shown).toContain("github.create_issue(title: str, body: str = None)");
      expect(shown).toContain("may destroy");
      screen.mockInput.pressEnter();
      shown = await shows(frame, "str, needed");
      expect(shown).toContain("The title");
      await pressEscape(screen);
      await shows(frame, "Tools of github");
      await pressEscape(screen);
      await shows(frame, "MCP servers");
      await pressEscape(screen);
    },
    { width: 180, height: 50 },
    session,
  );
});

test("each prompt of a connected server is a command whose words or form fill its arguments, and which sends its text to the chain", async () => {
  const session = await demoSession({ cwd: project() });
  await composing(
    async ({ app, frame, screen }) => {
      await session.command(`/run ${WORD}\nmcp_servers()`);
      await shows(frame, "MCP: 2 servers, 1 failed");
      expect(app.parts.command("mcp__github__review", app.seen())?.detail).toBe("Review a pull request");
      expect(app.parts.command("mcp__slack__digest", app.seen())).toBeUndefined();
      const sent = (text: string) => () =>
        session.acts.some((act) => act.kind === "prompt" && act.words[1] === text);
      app.composer.setText("/mcp__github__review 42");
      await app.submit();
      await until(sent("Review the pull request 42 of github."));
      app.composer.setText("/mcp__github__review");
      await app.submit();
      await shows(frame, "The number of the pull request");
      await screen.mockInput.typeText("9");
      screen.mockInput.pressEnter();
      await until(sent("Review the pull request 9 of github."));
      expect(session.acts.filter((act) => act.kind === "mcp_prompt").map((act) => act.words)).toEqual([
        ["github", "review", { pr: "42" }],
        ["github", "review", { pr: "9" }],
      ]);
    },
    { width: 180, height: 50 },
    session,
  );
});

test("@ completes the resources of the connected servers, and a call of a tool has a row with its arguments and its result", async () => {
  const session = await demoSession({ cwd: project() });
  await composing(
    async ({ app, frame, screen }) => {
      await session.command(
        `/run ${WORD}\nmcp_servers()\nmcp_call("github", "create_issue", {"title": "Bug"})`,
      );
      await shows(frame, "MCP: 2 servers, 1 failed");
      app.composer.setText("");
      await screen.mockInput.typeText("see @read");
      await shows(frame, "@github:repo://readme");
      screen.mockInput.pressEnter();
      await until(() => app.composer.plainText === "see @github:repo://readme ");
      const call = session.acts.find((act) => act.kind === "mcp_call");
      expect(call && app.partRow(call)).toEqual({
        label: 'github.create_issue({"title":"Bug"})',
        details: [
          [
            ["arguments ", "faint"],
            ['{\n  "title": "Bug"\n}', "muted"],
          ],
          [["Created the issue #7.", "text"]],
        ],
      });
    },
    { width: 180, height: 50 },
    session,
  );
});
