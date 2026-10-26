import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { seedDemo } from "../demo.ts";
import type { Session } from "../session.ts";
import { audit, checklist } from "./audit.ts";
import { release, watching } from "./conductor.ts";
import { research } from "./rivals.ts";

/** Put the notes that the audit of the demo reads in the project, which the demo files do not hold. */
export async function seedResearch(directory: string): Promise<void> {
  for (const [path, content] of Object.entries(research)) {
    await mkdir(dirname(join(directory, path)), { recursive: true });
    await writeFile(join(directory, path), content);
  }
}

/** Wait until the life says nothing more for a while: each model that will answer has answered, and what stays open
 * waits for the operator, for a command that runs long, or for a model that thinks until a cancel. */
export async function quiet(session: Session, still = 1000, limit = 90_000): Promise<void> {
  const start = Date.now();
  let last = "";
  let since = Date.now();
  while (Date.now() - start < limit) {
    await session.refresh();
    const now = `${session.acts.length}/${session.acts.filter((act) => act.done).length}/${session.host.prompts.size}`;
    if (now !== last) {
      last = now;
      since = Date.now();
    } else if (Date.now() - since >= still) return;
    await new Promise((done) => setTimeout(done, 150));
  }
  throw new Error(`The life of the demo did not settle in ${limit} ms: ${last}`);
}

/** The life of a real project, as a conductor makes it: the owner gives one task on the root chain, and its model
 * hires a worker on a chain of its own for each part, which read, write, fail, mend and ask in turn. It goes on after
 * the small conversation of seedDemo, and gives the acts that stay open on purpose: the work that waits for the
 * operator, the command that runs long, and the conductor that waits for its workers. */
export async function seedLife(session: Session): Promise<Set<string>> {
  await seedDemo(session);
  await seedResearch(session.host.directory);
  const { engine } = session;
  await engine.prompt("str", { message: release, on: engine.root });
  await quiet(session);
  await engine.prompt("str", { message: watching, on: engine.root });
  await quiet(session);
  await engine.prompt("str", { message: audit, on: engine.root });
  await quiet(session);
  await engine.prompt("str", { message: checklist, on: engine.root });
  await quiet(session);
  await seedAsks(session);
  return new Set(session.acts.filter((act) => !act.done).map((act) => act.id));
}

/** The chain that a label names. */
function chainOf(session: Session, label: string): string {
  const found = session.chains.find((one) => one.words[0] === label)?.id;
  if (!found) throw new Error(`The life of the demo has no chain ${label}.`);
  return found;
}

/** The first prompt of a chain, which is the thread of its worker. */
function threadOf(session: Session, label: string): string {
  const chain = chainOf(session, label);
  const found = session.acts.find((act) => act.kind === "prompt" && act.on === chain);
  if (!found) throw new Error(`The chain ${label} has no prompt.`);
  return found.id;
}

const relay = [
  "w/storage asks the operator:",
  "",
  "A file .notes/notes.jsonl exists already, and the code that starts the app reads it whole. Before I build the restart, I need one ruling, since each answer changes the format of the file:",
  "",
  "1. A file of lines keeps the notes that the owner wrote by hand, and a bad line stops the start.",
  "2. A file with a version in its first line lets a later release change the format, and costs one line.",
  "3. SQLite needs a dependency that the project has not got, and gives a search by index for free.",
  "",
  "- **A.** Lines, as it stands: the smallest change, and a bad line is skipped with a notice.",
  "- **B.** Lines with a version in the first line.",
  "",
  "Answer A or B, with any change.",
].join("\n");

export async function seedAsks(session: Session): Promise<void> {
  const { engine } = session;
  const ask = (shape: string, message: string, label: string) =>
    engine.prompt(shape, { message, to: "operator", on: chainOf(session, label) });
  await engine.prompt("str", { message: relay, to: "operator", on: engine.root });
  await ask("Literal['bun test', 'node --test', 'both']", "Which runner should ci.sh call?", "w/ci");
  await ask("int", "Search over 2000 notes is fast. How many notes should the bench hold in CI?", "w/perf");
  await ask("None", "#412 w/search closed its report, and its 5 tests pass.", "w/search");
  await ask("None", "main is green at 3f9a2c1.", "w/types");
  await engine.prompt("str", {
    message: "Also make a search for cafe find a note that says Café.",
    on: threadOf(session, "w/search"),
  });
  const notes = await engine.chain({ label: "w/notes" });
  const held = await engine.prompt("str", { message: "Think until I cancel.", on: notes });
  await quiet(session);
  await engine.cancel(held);
  await quiet(session);
}
