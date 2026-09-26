import { expect, test } from "bun:test";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createModels, type Message } from "@earendil-works/pi-ai";
import { Session } from "../src/index.ts";
import { claudeProvider } from "../src/providers/claude.ts";
import { executable } from "./executable.ts";
import { remove } from "./processes.ts";

/** The fake Claude CLI, compiled into a directory. */
const fake = (directory: string) => executable(join(import.meta.dir, "fake-claude.ts"), directory, "claude");

test("a CLI that exits before it reads a request fails that request, and a Node host lives on", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-gone-cli-"));
  await writeFile(join(cwd, "gone.ts"), "process.exit(3);\n");
  const bin = executable(join(cwd, "gone.ts"), cwd, "claude");
  // Node ends its process at an error event that nothing hears, so the host is Node, on the built package.
  const host = `
    import { createModels } from ${JSON.stringify(import.meta.resolve("@earendil-works/pi-ai"))};
    import { claudeProvider } from ${JSON.stringify(new URL("../dist/providers/claude.js", import.meta.url).href)};
    const cli = claudeProvider({ bin: ${JSON.stringify(bin)}, stallMs: 1000 });
    const models = createModels();
    models.setProvider(cli.provider);
    // A request over the buffer of the pipe is still being written when the CLI is gone.
    const reply = await models.completeSimple(
      models.getModel("claude-cli", "sonnet"),
      { messages: [{ role: "user", content: "x".repeat(4 * 1024 * 1024), timestamp: 0 }] },
      { sessionId: "gone" },
    );
    cli.dispose();
    process.stdout.write(reply.stopReason);
  `;
  try {
    const node = Bun.spawnSync(["node", "--input-type=module", "-e", host]);
    expect([node.exitCode, node.stdout.toString()]).toEqual([0, "error"]);
  } finally {
    await remove(cwd);
  }
}, 30000);

test("two lives on one provider keep a conversation each, though their chains share ids", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-two-lives-"));
  const bin = fake(cwd);
  const log = join(cwd, "cli.jsonl");
  process.env.FURB_FAKE_LOG = log;
  const cli = claudeProvider({ bin, stallMs: 1000 });
  const models = createModels();
  models.setProvider(cli.provider);
  const one = new Session({ cwd, models, model: "claude-cli:sonnet" });
  const two = new Session({ cwd, models, model: "claude-cli:sonnet" });
  try {
    const [first, second] = [one.open(), two.open()];
    expect(first.root).toBe(second.root);
    expect(
      await Promise.all([
        first.prompt("str", { message: "task", on: first.root }),
        second.prompt("str", { message: "task", on: second.root }),
      ]),
    ).toEqual(["reply 1", "reply 1"]);
    expect((await readFile(log, "utf8")).split("\n").filter((line) => line.includes('"pid"'))).toHaveLength(
      2,
    );
  } finally {
    await one.dispose();
    await two.dispose();
    cli.dispose();
    delete process.env.FURB_FAKE_LOG;
    await remove(cwd);
  }
}, 30000);

test("the pi-ai Claude provider forwards normalized system text, reuses a session, and charges each turn once", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-provider-"));
  const path = fake(cwd);
  const log = join(cwd, "cli.jsonl");
  process.env.FURB_FAKE_LOG = log;
  const cli = claudeProvider({ bin: path, stallMs: 1000 });
  const models = createModels();
  models.setProvider(cli.provider);
  const model = models.getModel("claude-cli", "sonnet");
  if (!model) throw new Error("Missing test model");
  try {
    const messages: Message[] = [{ role: "user", content: "first", timestamp: 0 }];
    const stream = models.streamSimple(
      model,
      { systemPrompt: "ENGINE ONLY", messages },
      { sessionId: "chain", reasoning: "xhigh" },
    );
    const deltas: string[] = [];
    for await (const event of stream) if (event.type === "text_delta") deltas.push(event.delta);
    const one = await stream.result();
    expect(deltas).toEqual(["close(", '"reply 1")']);
    expect(one.content).toEqual([{ type: "text", text: 'close("reply 1")' }]);
    expect(one.stopReason).toBe("stop");
    expect(one.usage.cost.total).toBe(0.01);
    messages.push(one, { role: "user", content: "second", timestamp: 0 });
    const two = await models.completeSimple(
      model,
      { systemPrompt: "ENGINE ONLY", messages },
      { sessionId: "chain", reasoning: "xhigh" },
    );
    expect(two.usage.cost.total).toBe(0.01);
    const lines = (await readFile(log, "utf8"))
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    expect(lines.filter((line) => line.pid)).toHaveLength(1);
    expect(lines[0].args).toContain("ENGINE ONLY");
    expect(lines[0].args).toContain("xhigh");
    expect(lines[0].args[lines[0].args.indexOf("--tools") + 1]).toBe("");
    expect(JSON.stringify(lines.at(-1))).not.toContain("first");
    const duplicate = await models.completeSimple(
      model,
      { systemPrompt: "ENGINE ONLY", messages },
      { sessionId: "chain", reasoning: "xhigh" },
    );
    expect(duplicate.stopReason).toBe("stop");
    expect((await readFile(log, "utf8")).split("\n").filter((line) => line.includes('"pid"'))).toHaveLength(
      1,
    );
    const stalled = await models.completeSimple(
      model,
      { messages: [{ role: "user", content: "WAIT", timestamp: 0 }] },
      { sessionId: "stall" },
    );
    expect(stalled.stopReason).toBe("error");
    expect(stalled.errorMessage).toContain("no progress");
    const signal = new AbortController();
    const waiting = models.streamSimple(
      model,
      { systemPrompt: "ENGINE ONLY", messages: [{ role: "user", content: "WAIT", timestamp: 0 }] },
      { sessionId: "other", signal: signal.signal },
    );
    // The request is aborted once it is in flight, which its first event says.
    for await (const event of waiting) if (event.type === "start") signal.abort();
    expect((await waiting.result()).stopReason).toBe("aborted");
  } finally {
    cli.dispose();
    delete process.env.FURB_FAKE_LOG;
    await remove(cwd);
  }
}, 30000);

test("a turn claude settles block by block keeps each block once, at the place it streamed", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-blocks-"));
  const cli = claudeProvider({ bin: fake(cwd), stallMs: 1000 });
  const models = createModels();
  models.setProvider(cli.provider);
  const model = models.getModel("claude-cli", "sonnet");
  if (!model) throw new Error("Missing test model");
  try {
    const one = await models.completeSimple(
      model,
      { systemPrompt: "ENGINE ONLY", messages: [{ role: "user", content: "THINK", timestamp: 0 }] },
      { sessionId: "think" },
    );
    expect(one.content).toEqual([
      { type: "thinking", thinking: "hmm", thinkingSignature: "sig" },
      { type: "text", text: 'close("reply 1")' },
    ]);
    const stream = models.streamSimple(
      model,
      { systemPrompt: "ENGINE ONLY", messages: [{ role: "user", content: "REDACT THINK", timestamp: 0 }] },
      { sessionId: "redact" },
    );
    // A block of a kind the provider does not read takes no place, so no partial reply has a hole.
    for await (const event of stream)
      if ("partial" in event) expect(event.partial.content.every((block) => block !== undefined)).toBe(true);
    const two = await stream.result();
    expect(two.content).toEqual([
      { type: "thinking", thinking: "hmm", thinkingSignature: "sig" },
      { type: "text", text: 'close("reply 1")' },
    ]);
  } finally {
    cli.dispose();
    await remove(cwd);
  }
}, 30000);
