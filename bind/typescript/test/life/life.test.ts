import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const folder = join(import.meta.dir, "../../src/life");

test("the model of a life imports nothing but its own files, so that a window of the web reads it as a terminal does", () => {
  for (const name of readdirSync(folder)) {
    const text = readFileSync(join(folder, name), "utf8");
    const imported = [...text.matchAll(/^(?:import|export)[^;]*?from "([^"]+)"/gm)].map((match) => match[1]);
    for (const path of imported) expect([name, path]).toEqual([name, expect.stringMatching(/^\.\/\w+\.js$/)]);
  }
});
