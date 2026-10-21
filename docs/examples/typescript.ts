import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { boot, type Turn } from "@furb/engine";

const PLAN = "Plan the release.";
const REVIEW = "Review the plan.";
const COUNT = "Count the files.";
const WAIT = "Wait for a new plan.";
const WORDS: Record<string, string> = {
  [PLAN]: [
    'reviewer = chain("reviewer")',
    'review = await prompt(str, "Review the plan.", on=reviewer)',
    'ship = await prompt(str, "Ship it?", to=OPERATOR)',
    'close(f"{review}, and the operator says {ship}")',
  ].join("\n"),
  [REVIEW]: 'close("looks good")',
  [COUNT]: "files = 3",
  [WAIT]: "await wait(3600)",
};
const DOLLARS = 0.3;
const asked: string[] = [];

function check(holds: boolean, claim: string): void {
  if (!holds) throw new Error(`the example fails: ${claim}`);
}

async function model(request: { messages: unknown[] }): Promise<Turn> {
  const said = JSON.stringify(request.messages);
  const task = Object.keys(WORDS).find((one) => said.includes(one)) ?? "";
  asked.push(task);
  return ["assistant", WORDS[task] ?? "", [100, 10, 0, 0, DOLLARS], null];
}

function life(record: string, answer: string | null) {
  return boot({
    cwd: process.cwd(),
    record,
    model: "claude-cli:opus",
    extensions: false,
    answer: model,
    operator: () => (answer === null ? new Promise(() => {}) : Promise.resolve(answer)),
  });
}

async function until(holds: () => boolean): Promise<void> {
  while (!holds()) await new Promise((resolve) => setTimeout(resolve, 10));
}

async function first(record: string): Promise<void> {
  const session = life(record, null);
  const { engine } = session;
  const root = engine.root;
  const plan = engine.prompt("str", { message: PLAN, on: root });
  await until(() =>
    engine.transcript({ on: root }).some((one) => one[0] === "prompt" && one[6] === "operator"),
  );
  await session.dispose();
  process.stdout.write(`${plan.id}\n`);
}

async function second(record: string, plan: string): Promise<void> {
  const session = life(record, "yes");
  const { engine } = session;
  const root = engine.root;
  try {
    await session.resume();
    check((await engine.result(plan)) === "looks good, and the operator says yes", "the life resumes");
    check(asked.length === 0, "the record answers each word of the first life");
    const reviewer = engine.transcript({ on: root }).find((one) => one[0] === "chain");
    check(engine.get(String(reviewer?.[1]))?.[4] === "reviewer", "the model opens a chain of its own");

    const budget = String(engine.chain({ label: "budget", on: root }));
    engine.grant({ usd: 0.5, on: budget });
    engine.prompt("int", { message: COUNT, on: budget });
    await until(() => session.isPaused(budget));
    check(asked.join() === [COUNT, COUNT].join(), "the grant pauses the chain at its ceiling");

    const door = String(engine.chain({ label: "door", on: root }));
    const waiting = engine.prompt("int", { message: WAIT, on: door });
    await until(() => engine.transcript({ on: door }).some((one) => one[0] === "wait"));
    engine.write({ path: waiting.id, content: "close(4)" }, { on: door });
    check((await waiting) === 4, "a write of the door of a prompt changes the plan of its ladder");
  } finally {
    await session.dispose();
  }
}

const [given] = process.argv.slice(2);
if (given) {
  await first(given);
} else {
  const yard = mkdtempSync(join(tmpdir(), "furb-example-"));
  const record = join(yard, "life.jsonl");
  const ran = execFileSync(process.execPath, [process.argv[1] ?? "", record], { encoding: "utf8" });
  const plan = ran.trim().split("\n").at(-1) ?? "";
  process.stdout.write(`the first process ended while the operator was asked; ${plan} waits in ${record}\n`);
  await second(record, plan);
  rmSync(yard, { recursive: true, force: true });
  process.stdout.write("the second process resumed it from the record, and every claim holds\n");
}
