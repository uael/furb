// The reload of the servers of MCP as the operator uses it: a server whose tools change on its own side, /mcp reload,
// and the changed tool found and called with no restart of the session.
import { expect } from "bun:test";
import { copyFile, mkdir, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { type Rule, test, Window, Yard } from "./harness.ts";

/** A server of MCP over stdio that reads the names of its tools, one on each line, from the file tools.txt beside it at
 * each tools/list, and answers a call of a tool with its name. */
const SERVER = `import { readFileSync } from "node:fs";
import { createInterface } from "node:readline";
const names = () => readFileSync(new URL("tools.txt", import.meta.url), "utf8").split("\\n").filter(Boolean);
const send = (message) => process.stdout.write(JSON.stringify({ jsonrpc: "2.0", ...message }) + "\\n");
for await (const line of createInterface({ input: process.stdin })) {
  const { id, method, params } = JSON.parse(line);
  if (id === undefined) continue;
  if (method === "initialize")
    send({ id, result: { protocolVersion: params.protocolVersion, capabilities: { tools: {} }, serverInfo: { name: "tools", version: "1" } } });
  else if (method === "tools/list")
    send({ id, result: { tools: names().map((name) => ({ name, description: "Tool " + name + ".", inputSchema: { type: "object" } })) } });
  else if (method === "tools/call") send({ id, result: { content: [{ type: "text", text: "called " + params.name }] } });
  else send({ id, result: {} });
}
`;

const LOAD = 'await skill("mcp")\nclose("mcp played")';
const LIST = 'await mcp_servers()\nclose(await mcp_tools("tools"))';
const CALL = 'await mcp_servers()\nclose((await mcp_call("tools", "fresh", {}))["text"])';
const script: Rule[] = [
  { cue: "Scenario: load.", word: LOAD },
  { cue: "Scenario: list.", word: LIST },
  { cue: "Scenario: call.", word: CALL },
];

test("/mcp reload lists again the tools that a server changed, and the changed tool is called with no restart", async () => {
  const yard = await Yard.make(script);
  let window: Window | undefined;
  try {
    const folder = join(yard.work, "server");
    await mkdir(folder, { recursive: true });
    await writeFile(join(folder, "server.ts"), SERVER);
    await writeFile(join(folder, "tools.txt"), "hello\n");
    await mkdir(join(yard.work, ".furb"), { recursive: true });
    const server = { command: process.execPath, args: [join(folder, "server.ts")] };
    await writeFile(join(yard.work, ".furb", "mcp.json"), JSON.stringify({ mcpServers: { tools: server } }));
    // The window loads the part of mcp from the configs of the project, so a skill of the project holds that part.
    const view = join(yard.work, ".furb", "skills", "mcpview");
    await mkdir(view, { recursive: true });
    await copyFile(resolve(import.meta.dir, "..", "..", ".agents", "skills", "mcp", "tui.ts"), join(view, "tui.ts"));
    await writeFile(
      join(view, "SKILL.md"),
      "---\nname: mcpview\ndescription: The part of mcp for the TUI.\nmetadata:\n  furb-view: tui.ts\n---\nThe part of mcp.\n",
    );
    await writeFile(join(yard.work, ".furb", "config.json"), '{"skills": {"mcpview": "skills/mcpview"}}\n');
    window = await Window.open(yard);
    await window.type("Scenario: load.");
    await window.shows((frame) => frame.includes("mcp played"), "the skill mcp played");
    await window.type("Scenario: list.");
    const before = await window.shows((frame) => frame.includes("hello(): Tool hello."), "the tool hello of the server");
    expect(before).not.toContain("fresh(): Tool fresh.");
    // The tools change on the side of the server, with no restart of the session.
    await writeFile(join(folder, "tools.txt"), "hello\nfresh\n");
    await window.type("/mcp reload");
    await window.shows((frame) => frame.includes("MCP: tools is connected."), "the state of the server after the reload");
    // The command ends in the dialog of the servers, which Escape closes.
    await window.shows((frame) => frame.includes("MCP servers"), "the dialog of the servers");
    window.screen.mockInput.pressEscape();
    await window.shows((frame) => !frame.includes("MCP servers"), "the dialog closed");
    await window.type("Scenario: list.");
    await window.shows((frame) => frame.includes("fresh(): Tool fresh."), "the tool fresh that the reload lists");
    await window.type("Scenario: call.");
    await window.shows((frame) => frame.includes("called fresh"), "the call of the tool fresh");
  } finally {
    await window?.close(true).catch(() => {});
    await yard.remove();
  }
});
