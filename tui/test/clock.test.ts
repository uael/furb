import { expect, test } from "bun:test";
import { join } from "node:path";

const clock = join(import.meta.dir, "..", "src", "clock.ts");
const theme = join(import.meta.dir, "..", "src", "theme.ts");

/** What a new process prints for a line of code, with the environment that it is given. */
function printed(code: string, env: Record<string, string>): string {
  const run = Bun.spawnSync([process.execPath, "-e", code], { env: { ...process.env, ...env } });
  return run.stdout.toString().trim();
}

test("FURB_STILL_CLOCK holds the time of the view and the frame of the spinner at the time that it names", () => {
  const code = `const { clock } = await import(${JSON.stringify(clock)}); const { spin } = await import(${JSON.stringify(theme)}); const first = [clock(), spin()]; await Bun.sleep(250); console.log(JSON.stringify([first, [clock(), spin()]]));`;
  expect(JSON.parse(printed(code, { FURB_STILL_CLOCK: "1767225600000" }))).toEqual([
    [1767225600000, "⠋"],
    [1767225600000, "⠋"],
  ]);
});

test("with no FURB_STILL_CLOCK the view keeps the time of the machine", () => {
  const code = `const { clock } = await import(${JSON.stringify(clock)}); console.log(Math.abs(clock() - Date.now()) < 1000);`;
  expect(printed(code, { FURB_STILL_CLOCK: "" })).toBe("true");
});
