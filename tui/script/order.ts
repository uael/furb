import { readFileSync } from "node:fs";

/** What a test file says of its longest test: the largest limit of time, in milliseconds, that it gives a test. The
 * limit is the last argument of a test: after the closing brace of its function, or on a line of its own. */
export function weight(source: string): number {
  const limits = [...source.matchAll(/\},\s*(\d[\d_]{3,})\)|^\s*(\d[\d_]{3,}),\n\)/gm)].map(
    ([, one = "", other = ""]) => Number((one || other).replaceAll("_", "")),
  );
  return Math.max(0, ...limits);
}

/** The test files, the longest limit first, and the largest file first among equal limits. A small file can hold the
 * longest test, as the gallery does, so the size of a file alone would start it last and leave it to hold the run open. */
export function longestFirst(paths: string[]): string[] {
  const read = new Map(paths.map((path) => [path, readFileSync(path, "utf8")]));
  const size = (path: string) => read.get(path)?.length ?? 0;
  return [...paths].sort(
    (one, other) => weight(read.get(other) ?? "") - weight(read.get(one) ?? "") || size(other) - size(one),
  );
}
