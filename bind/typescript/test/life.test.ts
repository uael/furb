import { afterEach, expect, test } from "bun:test";
import { type Entry, type Life, WorldAdapter, type WorldRequest } from "../src/index.ts";

const lives: Life[] = [];
afterEach(async () => {
  for (const life of lives.splice(0)) await life.dispose();
});

async function open(record: unknown[] = [], answer = 'close("hello")') {
  const entries: unknown[] = [];
  const facts: unknown[] = [];
  const files = new Map<string, string>([["a", "one\ntwo\n"]]);
  // A wait of no seconds is over at once; any other is over when the test says so, and never by the clock.
  const waits: (() => void)[] = [];
  let asks = 0;
  const world = ({ kind, args }: WorldRequest): unknown => {
    switch (kind) {
      case "Stand":
        return [
          [
            ["model", ["low"], 200000],
            ["operator", [], 200000],
          ],
          "/tmp",
          "model/low",
        ];
      case "Keep":
        entries.push(args[0]);
        return null;
      case "Clock":
        return 123.5;
      case "Chance":
        return 0.25;
      case "Read":
        if (!files.has(String(args[1]))) throw new Error("missing file");
        return { path: args[1], content: files.get(String(args[1])) };
      case "Write":
        files.set(String(args[1]), String(args[2]));
        return { path: args[1], content: args[2] };
      case "Ask":
        asks++;
        return Promise.resolve(["assistant", [answer], [20, 8, 0, 0, 0.001], null]);
      case "Wait":
        return Number(args[0]) === 0 ? null : new Promise((resolve) => waits.push(() => resolve(null)));
      case "Prompt":
        return "operator answer";
      default:
        return null;
    }
  };
  const life = new WorldAdapter(world, (batch) => facts.push(...batch)).boot(record as Entry[]);
  lives.push(life);
  const release = () => {
    for (const done of waits.splice(0)) done();
  };
  return { life, entries, facts, release, files, asks: () => asks };
}

test("native queries are synchronous and acts await the real engine and async World", async () => {
  const { life, asks } = await open();
  expect(life.root).toBe("chain://operator.1");
  expect(life.clock()).toBe(123.5);
  expect(life.chance()).toBe(0.25);
  expect(await life.wait(0)).toBeNull();
  const prompt = life.prompt("str", "Say hello").id;
  expect(await life.result<string>(prompt)).toBe("hello");
  expect(asks()).toBe(1);
  expect((await life.turns()).some((turn) => turn[0] === "assistant")).toBe(true);
  expect(await life.gate("this is not python !!!")).not.toEqual([]);
});

test("a pending result leaves JavaScript and other native operations available", async () => {
  const { life, release } = await open();
  const id = life.wait(60).id;
  const pending = life.result(id);
  expect(await life.cwd()).toBe("/tmp");
  expect(await life.outcome(id)).toEqual({ done: false, value: null });
  release();
  expect(await pending).toBeNull();
});

test("pause holds a model response until wake and cancel rejects a native await", async () => {
  const { life, asks } = await open();
  await life.pause(life.root);
  const id = life.prompt("str", "Say hello").id;
  // A paused chain asks no model, so nothing answers the prompt until the wake.
  expect(asks()).toBe(0);
  expect((await life.outcome(id)).done).toBe(false);
  await life.wake(life.root);
  expect(await life.result<string>(id)).toBe("hello");
  const later = life.wait(60).id;
  const result = life.result(later).then(
    () => "resolved",
    (error: Error) => error.message,
  );
  await life.cancel(later);
  expect(await result).toContain("CancelledError");
});

test("text, command results, and engine callables cross N-API", async () => {
  const { life } = await open();
  expect(await life.read<Record<string, string>>("a")).toEqual({
    is: "Text",
    path: "a",
    content: "one\ntwo\n",
  });
  expect(() => life.read("missing")).toThrow("missing file");
  const show = await life.span(1, 1);
  expect(await life.made<number[]>(show.id, [["one", "two"]], {})).toEqual([1]);
  await life.forget(show.id);
  const command = life.bash("fake", { fed: true }).id;
  await life.send("out", command, ["hello\n", "stdout"]);
  await life.send("exited", command, [0]);
  const exit = await life.result(command);
  expect(exit).toMatchObject({ is: "Exit", code: 0, stdout: { is: "Text", content: "hello\n" } });
});

test("a second life replays model answers and durable rung effects without asking again", async () => {
  const first = await open();
  const prompt = first.life.prompt("str", "Say hello").id;
  await first.life.result<string>(prompt);
  const rung = first.life.rung('write(Text("b", "saved"))').id;
  await first.life.result(rung);
  const second = await open(first.entries);
  expect(await second.life.result<string>(prompt)).toBe("hello");
  expect(second.asks()).toBe(0);
  expect(second.files.has("b")).toBe(false);
  expect(second.entries).toHaveLength(0);
  const fork = second.life.chain("branch", second.life.root).id;
  expect(await second.life.cwd(fork)).toBe("/tmp");
});

test("dispose rejects pending native results and further operations", async () => {
  const { life } = await open();
  const result = life.result(life.root).then(
    () => "resolved",
    (error: Error) => error.message,
  );
  await life.dispose();
  expect(await result).toContain("disposed");
  expect(() => life.cwd()).toThrow("disposed");
});

test("map order and live Python values cross without losing their meaning", async () => {
  const { life } = await open();
  await life.rung('ordered = {"z": 1, "a": 2}\nvalue = 1.0');
  const ordered = life.inspect("ordered");
  expect(Object.keys(ordered.value as object)).toEqual(["z", "a"]);
  expect(life.inspect("value").kind).toBe("float");
});
