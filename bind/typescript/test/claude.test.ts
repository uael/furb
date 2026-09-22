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
  const cli = claudeProvider({ bin: path });
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
    const signal = new AbortController();
    const waiting = models.completeSimple(
      model,
      { systemPrompt: "ENGINE ONLY", messages: [{ role: "user", content: "WAIT", timestamp: 0 }] },
      { sessionId: "other", signal: signal.signal },
    );
    setTimeout(() => signal.abort(), 25);
    expect((await waiting).stopReason).toBe("aborted");
  } finally {
    cli.dispose();
    delete process.env.FURB_FAKE_LOG;
    await rm(cwd, { recursive: true });
  }
});
