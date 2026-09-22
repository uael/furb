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
  for (const event of [
    { type: "message_start" },
    { type: "content_block_start", index: 0, content_block: { type: "text", text: "" } },
    { type: "content_block_delta", index: 0, delta: { type: "text_delta", text: word.slice(0, 6) } },
    { type: "content_block_delta", index: 0, delta: { type: "text_delta", text: word.slice(6) } },
    { type: "content_block_stop", index: 0 },
    { type: "message_stop" },
  ])
    process.stdout.write(`${JSON.stringify({ type: "stream_event", event })}\n`);
  process.stdout.write(
    `${JSON.stringify({ type: "assistant", message: { content: [{ type: "text", text: word }] } })}\n`,
  );
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
