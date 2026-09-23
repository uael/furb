import { expect, test } from "bun:test";
import { chmod, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createModels, type Message } from "@earendil-works/pi-ai";
import { claudeProvider } from "../src/providers/claude.ts";

test("the pi-ai Claude provider forwards normalized system text, reuses a session, and charges each turn once", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-provider-"));
  const path = new URL("fake-claude.ts", import.meta.url).pathname;
  await chmod(path, 0o755);
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
    await rm(cwd, { recursive: true });
  }
});

test("a turn claude settles block by block keeps each block once, at the place it streamed", async () => {
  const path = new URL("fake-claude.ts", import.meta.url).pathname;
  await chmod(path, 0o755);
  const cli = claudeProvider({ bin: path, stallMs: 1000 });
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
  }
});
