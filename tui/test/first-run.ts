// The first run of the TUI on a real life, which first-run.test.ts runs as a process of its own, in a home and an
// environment that it makes, so that the life finds only the claude command line and the keys that they hold. It
// prints the frame that the App draws once the check of the setup came, then the acts of the chain, one a line.
import { join } from "node:path";
import { createTestRenderer } from "@opentui/core/testing";
import { until } from "../../bind/typescript/test/until.ts";
import { App } from "../src/app.ts";
import { openEngine } from "../src/bridge.ts";
import { demoLibrary } from "../src/demo.ts";
import { Session } from "../src/session.ts";

const [directory = ".", size = "132x44"] = process.argv.slice(2);
const [width = 132, height = 44] = size.split("x").map(Number);
const opened = await openEngine({ cwd: directory, record: join(directory, "session.jsonl") });
const session = new Session(opened.engine, opened.host, true);
await session.refresh();
// The extensions play their words on the chain as it starts, and the setup comes from the worker.
await until(session, () => session.setup !== undefined && session.catalogRead && session.activity.length > 0);
await session.refresh();
const library = await demoLibrary(session);
const screen = await createTestRenderer({ exitSignals: [], width, height });
const app = new App(screen.renderer, session, { quit() {}, workspaces: library });
app.render();
await screen.flush();
app.render();
await screen.flush();
console.log(screen.captureCharFrame());
console.log("---");
for (const act of session.activity) console.log(`${act.id} ${act.by}`);
app.dispose();
screen.renderer.destroy();
await library.dispose();
process.exit(0);
