// A plain `bun test` loads this file through bunfig.toml. It gives the tests the folders of the runner, but it cannot
// hide the claude command line, since the native part reads the PATH that the process started with. So it says so.
import { setDefaultTimeout } from "bun:test";
import { existsSync } from "node:fs";
import { delimiter, join } from "node:path";
import { environment } from "./environment.ts";

const program = process.platform === "win32" ? "claude.exe" : "claude";
const found = (process.env.PATH ?? "")
  .split(delimiter)
  .some((folder) => folder && existsSync(join(folder, program)));
if (found && process.env.FURB_TESTS !== "1") {
  console.error(
    "A claude command line stands on the PATH, so tests that name a model can fail: run `bun run test <filter>`.",
  );
}
Object.assign(process.env, environment());
setDefaultTimeout(30000);
