import { test } from "bun:test";
import { fileURLToPath } from "node:url";

test("probe: which worker lifecycle breaks a native event of OpenTUI", async () => {
  for (const variant of ["plain-worker", "opentui-worker-alive", "opentui-worker", "session"]) {
    const child = Bun.spawn(
      [process.execPath, fileURLToPath(new URL("native-probe.ts", import.meta.url)), variant],
      {
        stdout: "pipe",
        stderr: "pipe",
      },
    );
    const [out, err, code] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    console.log(
      `PROBE ${process.platform} ${variant} exit ${code}\n${out}${err.split("\n").slice(0, 14).join("\n")}`,
    );
  }
}, 120000);
