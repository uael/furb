import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { RecordLock } from "@furb/engine";

/** The moment a condition holds, read again every 20 ms: a process in a terminal tells its state by no event. */
async function eventually(ready: () => boolean | Promise<boolean>, what: string): Promise<void> {
  for (const deadline = Date.now() + 15000; !(await ready()); await Bun.sleep(20))
    if (Date.now() > deadline) throw new Error(`Waited 15 seconds for ${what}.`);
}

/** Whether a process of this number still runs. */
function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

for (const signal of ["SIGHUP", "SIGTERM"] as const)
  test(`${signal} ends the TUI as its quit does: the draft is saved, commands end, and the record is free`, async () => {
    const directory = await mkdtemp(join(tmpdir(), "furb-signal-"));
    const project = join(directory, "project");
    const pidFile = join(directory, "command.pid");
    let screen = "";
    const terminal = new Bun.Terminal({
      cols: 120,
      rows: 40,
      data(_terminal, data) {
        screen += new TextDecoder().decode(data);
      },
    });
    await mkdir(project);
    const child = Bun.spawn(
      [process.execPath, join(import.meta.dir, "../src/cli.ts"), "--demo", "--cwd", project],
      {
        env: { ...process.env, FURB_CONFIG_DIR: join(directory, "config") },
        terminal,
      },
    );
    try {
      await eventually(() => screen.includes("Workspaces"), "the first frame");
      terminal.write(`!echo $$ > '${pidFile}'; sleep 30\r`);
      const pid = async () => Number(await readFile(pidFile, "utf8").catch(() => ""));
      await eventually(async () => (await pid()) > 0, "the command to start");
      // The submit takes its text from the composer at once, so a draft typed after it stays.
      terminal.write("draftmarker");
      await eventually(() => screen.includes("draftmarker"), "the draft on the screen");
      child.kill(signal);
      expect(await Promise.race([child.exited, Bun.sleep(15000).then(() => "still running")])).toBe(0);
      const sessions = join(project, ".furb/sessions");
      const [record] = (await readdir(sessions)).filter(
        (name) => name.endsWith(".jsonl") && !name.endsWith(".changes.jsonl"),
      );
      const view = JSON.parse(await readFile(join(sessions, `${record}.ui.json`), "utf8")) as {
        drafts: Record<string, string>;
      };
      expect(Object.values(view.drafts)).toContain("draftmarker");
      const command = await pid();
      await eventually(() => !alive(command), "the command to end");
      new RecordLock(join(sessions, record ?? "")).dispose();
    } finally {
      child.kill("SIGKILL");
      terminal.close();
      await rm(directory, { recursive: true, force: true });
    }
  }, 60000);
