// The claude command line of the tests of the examples, which answers from a script, as src/world/provider/fake-claude.sh
// does where a shell runs it. The word of a turn is the JSON text in the file word.N beside it, for the Nth turn of all,
// and each turn costs the dollars of the file cost beside it. When the file deaf stands beside it, it writes the file
// asked, reads no input and lives on.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline";

const here = dirname(process.execPath);
const beside = (name: string) => join(here, name);
const args = process.argv.slice(2);
const at = args.findIndex((one) => one === "--session-id" || one === "--resume");
const id = at >= 0 ? args[at + 1] : "none";
const say = (event: unknown) => console.log(JSON.stringify({ type: "stream_event", event }));
const usage = {
  input_tokens: 10,
  output_tokens: 5,
  cache_read_input_tokens: 20,
  cache_creation_input_tokens: 3,
};

if (existsSync(beside("deaf"))) {
  writeFileSync(beside("asked"), "");
  setInterval(() => {}, 1 << 30);
} else {
  const cost = existsSync(beside("cost")) ? Number(readFileSync(beside("cost"), "utf8")) : 0.01;
  let n = 0;
  for await (const _ of createInterface({ input: process.stdin })) {
    n++;
    const count = (existsSync(beside("count")) ? Number(readFileSync(beside("count"), "utf8")) : 0) + 1;
    writeFileSync(beside("count"), String(count));
    const word = existsSync(beside(`word.${count}`))
      ? JSON.parse(readFileSync(beside(`word.${count}`), "utf8"))
      : `close("reply ${n}")`;
    say({ type: "message_start" });
    say({ type: "content_block_start", index: 0, content_block: { type: "text", text: "" } });
    say({ type: "content_block_delta", index: 0, delta: { type: "text_delta", text: word } });
    say({ type: "content_block_stop", index: 0 });
    say({ type: "message_stop" });
    console.log(
      JSON.stringify({ type: "assistant", message: { content: [{ type: "text", text: word }], usage } }),
    );
    console.log(JSON.stringify({ type: "result", session_id: id, total_cost_usd: n * cost, usage }));
  }
}
