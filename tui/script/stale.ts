/** The build of the TypeScript side, bind/typescript/index.cjs and bind/typescript/dist, stands for the commit that made
 * it. A switch of branch, a pull or a copy of a build from another commit leaves it behind the sources, and a test then
 * fails with "Export named earProgram not found", or fails as on main, with no sign of the cause. A source that changed
 * after the build that reads it shows that the build is stale. */
import { statSync } from "node:fs";
import { join } from "node:path";
import { Glob } from "bun";

/** What the native module reads: the files that Cargo.toml says the crate holds, less the tests and the notes. */
const crate = [
  "Cargo.toml",
  "Cargo.lock",
  "build.rs",
  "DOCTRINE.md",
  "src/**/*.{rs,py,pyi,json}",
  "extensions/*/furb.json",
  "extensions/*/*.py",
  "extensions/*/tui.ts",
  "extensions/*/*/SKILL.md",
];
/** What tsc reads, beside the types that the native build wrote. */
const side = ["bind/typescript/src/**/*", "bind/typescript/tsconfig.json", "bind/typescript/index.d.cts"];
/** Each file that a build writes at each run, with the sources that it reads. */
const builds: [string, string[]][] = [
  ["bind/typescript/index.cjs", crate],
  ["bind/typescript/dist/index.js", side],
];
/** A file that the build does not read. */
const apart = /(\.test\.rs|(^|\/)(AGENTS|CLAUDE)\.md)$/;

/** The time that a file changed, or 0 when it is gone. */
function changed(path: string): number {
  try {
    return statSync(path).mtimeMs;
  } catch {
    return 0;
  }
}

/** The lines that name each part of the build behind its sources in the checkout at `root`: none when the build is
 * current. */
export function stale(root: string): string[] {
  const lines: string[] = [];
  for (const [built, sources] of builds) {
    const made = changed(join(root, built));
    if (made === 0) {
      lines.push(`${built} is missing`);
      continue;
    }
    let latest = "";
    let at = made;
    for (const pattern of sources)
      for (const found of new Glob(pattern).scanSync({ cwd: root, onlyFiles: true })) {
        // Windows gives a path with backslashes.
        const file = found.replaceAll("\\", "/");
        const time = changed(join(root, file));
        if (time > at && !apart.test(file)) {
          latest = file;
          at = time;
        }
      }
    if (latest) lines.push(`${latest} changed after ${built}`);
  }
  return lines;
}

/** What to tell the reader about a stale build: the lines, and the command that makes the build again. */
export function warning(lines: string[]): string {
  return `The build is stale: ${lines.join("; ")}. Run bun run build, and trust no failure that reads the build before it.`;
}
