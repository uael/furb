// The model of the scenarios of tui/scenarios: a claude command line that answers from a script, as the model of the
// owner answers his life.
// The script is the JSON file script.json beside the program, a list of rules, each a cue and the word that answers it.
// A turn takes the word of the rule whose cue stands last in the text that the turn reads, as the model answers the
// last thing that it was told. A turn that no cue answers waits an hour, so a scenario that misses a cue fails on the
// answer that it waited for, and no loop of refusals floods the life. A rule can stream its word in parts of
// `chunk` characters, `pause` milliseconds apart, and hold its answer until the file that `hold` names stands beside
// the program, so a scenario holds a reply open as long as it wants. A rule with a `thought` streams it first, as a
// block of thinking in the same parts, before it holds, so a scenario sees a long thought grow. Each turn appends a line to the file turns beside
// the program: the cue that it answered, or null, and the text that it read.
import { appendFileSync, existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { createInterface } from "node:readline";

interface Rule {
  cue: string;
  word: string;
  chunk?: number;
  pause?: number;
  hold?: string;
  thought?: string;
}

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

/** Each text that a value of the input holds, in order. */
function texts(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) return value.flatMap(texts);
  if (value && typeof value === "object") return Object.values(value).flatMap(texts);
  return [];
}

if (process.env.FURB_CLAUDE_DISCOVER) {
  // Furb asks the models of the command line, which has no discovery, so furb offers its fixed models at once.
  for await (const line of createInterface({ input: process.stdin })) {
    const request_id = (JSON.parse(line) as { request_id?: string }).request_id;
    console.log(
      JSON.stringify({
        type: "control_response",
        response: { subtype: "error", request_id, error: "unsupported" },
      }),
    );
  }
  process.exit(0);
}

let n = 0;
for await (const line of createInterface({ input: process.stdin })) {
  let input: { type?: string; message?: unknown; request_id?: string };
  try {
    input = JSON.parse(line);
  } catch {
    continue;
  }
  if (input.type === "control_request") {
    console.log(
      JSON.stringify({
        type: "control_response",
        response: { subtype: "success", request_id: input.request_id, response: {} },
      }),
    );
    continue;
  }
  if (input.type !== "user") continue;
  n++;
  const text = texts(input.message).join("\n");
  const rules = JSON.parse(readFileSync(beside("script.json"), "utf8")) as Rule[];
  let rule: Rule | undefined;
  let last = -1;
  for (const one of rules) {
    const found = text.lastIndexOf(one.cue);
    if (found >= 0 && found >= last) [rule, last] = [one, found];
  }
  appendFileSync(beside("turns"), `${JSON.stringify({ cue: rule?.cue ?? null, text })}\n`);
  say({ type: "message_start" });
  // A thought streams first, in the parts of the word, as its own block.
  const thought = rule?.thought ?? "";
  const block = thought ? 1 : 0;
  if (thought) {
    say({ type: "content_block_start", index: 0, content_block: { type: "thinking", thinking: "" } });
    const part = rule?.chunk ?? thought.length;
    for (let from = 0; from < thought.length; from += part) {
      say({
        type: "content_block_delta",
        index: 0,
        delta: { type: "thinking_delta", thinking: thought.slice(from, from + part) },
      });
      if (rule?.pause) await Bun.sleep(rule.pause);
    }
    say({ type: "content_block_stop", index: 0 });
  }
  say({ type: "content_block_start", index: block, content_block: { type: "text", text: "" } });
  if (rule?.hold) {
    const { cue, hold } = rule;
    while (!existsSync(beside(hold))) await Bun.sleep(20);
    // A scenario may write the word of a held rule while it holds, from what the life made since.
    const now = (JSON.parse(readFileSync(beside("script.json"), "utf8")) as Rule[]).find(
      (one) => one.cue === cue,
    );
    if (now) rule = now;
  }
  const word = rule?.word ?? "await wait(3600, __name__)";
  const size = rule?.chunk ?? word.length;
  for (let from = 0; from < word.length; from += size) {
    say({
      type: "content_block_delta",
      index: block,
      delta: { type: "text_delta", text: word.slice(from, from + size) },
    });
    if (rule?.pause) await Bun.sleep(rule.pause);
  }
  say({ type: "content_block_stop", index: block });
  say({ type: "message_stop" });
  console.log(
    JSON.stringify({ type: "assistant", message: { content: [{ type: "text", text: word }], usage } }),
  );
  console.log(JSON.stringify({ type: "result", session_id: id, total_cost_usd: n * 0.01, usage }));
}
