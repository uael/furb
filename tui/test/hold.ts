// Another window of a test, in a process of its own as a second terminal is: it opens the life of a record, says held
// once it holds it, and lets it go once its input closes. Two windows of one process share one connection to the
// daemon, which counts them as one window, so a test of another window holds the life from here.
import { openEngine } from "../src/bridge.ts";

const [cwd, record] = process.argv.slice(2);
if (!cwd || !record) throw new Error("hold.ts needs the directory and the record of a life.");
const opened = await openEngine({ cwd, record });
console.log("held");
for await (const _ of Bun.stdin.stream());
await opened.host.dispose();
process.exit(0);
