#!/usr/bin/env bun
import { appendFileSync } from "node:fs";
import { createInterface } from "node:readline";

const args = process.argv.slice(2);
if (process.env.FURB_FAKE_LOG)
  appendFileSync(process.env.FURB_FAKE_LOG, `${JSON.stringify({ args, pid: process.pid })}\n`);
let calls = 0;
const id = args[args.indexOf("--session-id") + 1] ?? "fake-session";
for await (const line of createInterface({ input: process.stdin })) {
  const request = JSON.parse(line);
  if (process.env.FURB_FAKE_LOG) appendFileSync(process.env.FURB_FAKE_LOG, `${JSON.stringify(request)}\n`);
  const text = JSON.stringify(request);
  if (text.includes("FAIL")) {
    process.stderr.write("deliberate failure\n");
    process.exit(2);
  }
  if (text.includes("WAIT")) continue;
  calls++;
  const word = `close(${JSON.stringify(`reply ${calls}`)})`;
  const think = text.includes("THINK");
  const say = (event: object) => process.stdout.write(`${JSON.stringify({ type: "stream_event", event })}\n`);
  const settle = (block: object) =>
    process.stdout.write(`${JSON.stringify({ type: "assistant", message: { content: [block] } })}\n`);
  const streamed = (index: number, block: object, deltas: object[]) => {
    say({ type: "content_block_start", index, content_block: block });
    for (const delta of deltas) say({ type: "content_block_delta", index, delta });
    say({ type: "content_block_stop", index });
  };
  say({ type: "message_start" });
  // claude says each block again, as an assistant event of its own, as soon as it is whole: before the next streams.
  let index = 0;
  if (text.includes("REDACT")) {
    const hidden = { type: "redacted_thinking", data: "opaque" };
    streamed(index++, hidden, []);
    settle(hidden);
  }
  if (think) {
    streamed(index++, { type: "thinking", thinking: "" }, [
      { type: "thinking_delta", thinking: "hmm" },
      { type: "signature_delta", signature: "sig" },
    ]);
    settle({ type: "thinking", thinking: "hmm", signature: "sig" });
  }
  streamed(index, { type: "text", text: "" }, [
    { type: "text_delta", text: word.slice(0, 6) },
    { type: "text_delta", text: word.slice(6) },
  ]);
  say({ type: "message_stop" });
  settle({ type: "text", text: word });
  process.stdout.write(
    `${JSON.stringify({
      type: "result",
      session_id: id,
      total_cost_usd: calls * 0.01,
      usage: {
        input_tokens: 10,
        output_tokens: 5,
        cache_read_input_tokens: 20,
        cache_creation_input_tokens: 3,
      },
    })}\n`,
  );
}
