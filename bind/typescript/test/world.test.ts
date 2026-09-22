import { expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { boot, decodeRecord, type Ear, Ears, type Fact, World } from "../src/index.ts";
import { RecordFile } from "../src/record.ts";

test("the default World serves files and streams commands without any TUI", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-world-"));
  const session = boot({ cwd });
  try {
    expect(session.life.cwd()).toBe(cwd);
    session.life.write({ path: "hello.txt", content: "hello\n" });
    expect(session.life.read("hello.txt").content).toBe("hello\n");
    const command = session.life.bash("printf 'snow: 雪\\n'; printf 'problem\\n' >&2", {
      showErr: { is: "name", name: "TAIL" },
    });
    const result = (await command) as {
      code: number;
      stdout: { content: string };
      stderr: { content: string };
    };
    expect(result.code).toBe(0);
    expect(result.stdout.content).toBe("snow: 雪\n");
    expect(result.stderr.content).toBe("problem\n");
    const input = session.life.bash("cat", { fed: true });
    session.life.write({ path: `${input.id}/stdin`, content: "fed\n" });
    session.life.write({ path: `${input.id}/stdin`, content: "" });
    expect(((await input) as { stdout: { content: string } }).stdout.content).toBe("fed\n");
  } finally {
    await session.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("unfinished model work and waits reopen paused until the host confirms resume", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-resume-"));
  const record = join(cwd, "life.jsonl");
  let calls = 0;
  const first = new World({ cwd, record, answer: () => new Promise(() => {}) });
  const life = first.open();
  const prompt = life.prompt("str", "pending").id;
  const wait = life.wait(0.1).id;
  await Bun.sleep(10);
  await first.dispose();
  const second = new World({
    record,
    answer: async () => {
      calls++;
      return ["assistant", ['close("resumed")'], null, null];
    },
  });
  const resumed = second.open();
  try {
    await Bun.sleep(30);
    expect(calls).toBe(0);
    expect(second.held.has(prompt)).toBe(true);
    expect(resumed.outcome(prompt).done).toBe(false);
    await second.resume();
    expect(await resumed.result<string>(prompt)).toBe("resumed");
    expect(await resumed.result(wait)).toBeNull();
    expect(calls).toBe(1);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("record ownership, a torn last line, and a damaged complete line are distinct", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-record-"));
  const path = join(cwd, "record.jsonl");
  try {
    await writeFile(path, '["x",["done","x","world",null]]\n["partial"');
    const record = new RecordFile(path);
    expect(record.entries).toHaveLength(1);
    expect(() => new RecordFile(path)).toThrow("Another process");
    record.keep(["y", ["done", "y", "world", 1]]);
    record.dispose();
    expect((await readFile(path, "utf8")).split("\n").filter(Boolean)).toHaveLength(2);
    await writeFile(path, "broken\n");
    expect(() => new RecordFile(path)).toThrow("Invalid record entry");
  } finally {
    await rm(cwd, { recursive: true });
  }
});

test("record numbers refuse precision loss and map keys keep their identity", () => {
  expect(decodeRecord('{"$serde_json::private::Number":"10"}')).toEqual({
    "$serde_json::private::Number": "10",
  });
  expect(() => decodeRecord("9007199254740993")).toThrow("safe integer");
  expect(() => decodeRecord("99999999999999999999999999999999")).toThrow("safe integer");
  expect(decodeRecord('"99999999999999999999999999999999"')).toBe("99999999999999999999999999999999");
});

test("integral floats keep their Python type in operator replies and records", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-float-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  const question = life.prompt<number>("float", "A number", { to: "operator" });
  await Bun.sleep(10);
  first.answer(question.id, "1.0");
  expect(await question).toBe(1);
  const id = question.id;
  await first.dispose();
  expect(await readFile(record, "utf8")).toContain('"is":"float"');
  const second = new World({ record });
  try {
    const next = second.open();
    expect(await next.result<number>(id)).toBe(1);
    expect(second.held.size).toBe(0);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("host ears yield nested bus calls and host shows remain callable", () => {
  const ears = new Ears({
    world: (function* (): Ear {
      for (;;) {
        const fact = (yield null) as Fact;
        if (fact?.[0] === "stand") yield ["done", fact[1], [[["operator", [], 200000]], "/tmp", "operator"]];
        if (fact?.[0] === "read") {
          const cwd = yield { verb: "cwd", kwargs: { on: fact[3] } };
          yield ["done", fact[1], { is: "Text", path: `${cwd}/file`, content: "one\ntwo\n" }];
        }
      }
    })(),
  });
  const life = ears.boot();
  try {
    const show = ears.callable((lines: string[]) => lines.map((_, index) => index + 1));
    expect(life.read("file", show).path).toBe("/tmp/file");
    expect(() => life.clock()).toThrow("no number");
  } finally {
    life.dispose();
  }
});

test("an interrupted command is reported on resume without repeating its side effect", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-command-resume-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const life = first.open();
  const command = life.bash("printf 'once\\n' >> count; printf ready; sleep 5").id;
  await Bun.sleep(40);
  await first.dispose();
  const second = new World({ record });
  try {
    const resumed = second.open();
    expect(second.held.has(command)).toBe(true);
    await second.resume();
    await expect(resumed.result(command)).rejects.toThrow("command process ended");
    expect(await readFile(join(cwd, "count"), "utf8")).toBe("once\n");
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});

test("an operator question survives a paused resume and validates its answer", async () => {
  const cwd = await mkdtemp(join(tmpdir(), "furb-operator-resume-"));
  const record = join(cwd, "life.jsonl");
  const first = new World({ cwd, record });
  const question = first.open().prompt("bool", "Continue?", { to: "operator" }).id;
  await Bun.sleep(10);
  await first.dispose();
  const second = new World({ record });
  try {
    const life = second.open();
    expect(second.prompts.size).toBe(0);
    await second.resume();
    await Bun.sleep(10);
    expect(second.prompts.get(question)?.message).toBe("Continue?");
    expect(() => second.answer(question, "maybe")).toThrow("yes or no");
    second.answer(question, "yes");
    expect(await life.result<boolean>(question)).toBe(true);
  } finally {
    await second.dispose();
    await rm(cwd, { recursive: true });
  }
});
