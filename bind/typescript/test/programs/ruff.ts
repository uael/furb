// A compiled formatter fixture runs on Windows too, where the operating system does not execute a shebang.
// Its neighboring JSON file belongs to one test; the runner's warm-up has no file and exits without waiting.
import { appendFile, readFile } from "node:fs/promises";

type Settings = { blocked?: boolean; started?: string; gate?: string; log?: string };
const settings: Settings | undefined = await readFile(`${process.execPath}.json`, "utf8")
  .then((text) => JSON.parse(text) as Settings)
  .catch(() => undefined);
if (settings) {
  const text = await Bun.stdin.text();
  if (settings.started) await Bun.write(settings.started, String(process.pid));
  if (settings.blocked) await new Promise(() => setInterval(() => {}, 1000));
  if (settings.gate) while (!(await Bun.file(settings.gate).exists())) await Bun.sleep(5);
  if (settings.log) await appendFile(settings.log, `${text}\n`);
  console.log(text.replace(/x=(\d+)/g, "x = $1"));
}
