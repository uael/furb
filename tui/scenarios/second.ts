// A second window of a scenario, in a process of its own as a second terminal is: it opens the life of the record of
// a yard, which the first window holds, sends a line, and ends once its frame shows the reply.
import { Window, Yard } from "./harness.ts";

const [root, work, model, record, cache, line, reply] = process.argv.slice(2);
if (!root || !work || !model || !record || !cache || !line || !reply)
  throw new Error(
    "second.ts needs the root, the work, the model, the record and the cache of a yard, a line and a reply.",
  );
const window = await Window.open(new Yard(root, work, model, record, cache));
await window.type(line);
await window.shows((frame) => frame.includes(reply), "the reply in the second window");
await window.close();
process.exit(0);
