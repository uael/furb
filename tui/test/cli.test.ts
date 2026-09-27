import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readdir, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { store } from "@furb/engine";
import { alive, printPid, remove } from "../../bind/typescript/test/processes.ts";

/** The moment a condition holds, read again every 20 ms: a process in a terminal tells its state by no event. */
async function eventually(ready: () => boolean | Promise<boolean>, what: string): Promise<void> {
  for (const deadline = Date.now() + 15000; !(await ready()); await Bun.sleep(20))
    if (Date.now() > deadline) throw new Error(`Waited 15 seconds for ${what}.`);
}

/** The TUI in a terminal of its own: what it wrote, the keys typed into it, the signal that ends it, and its exit. */
interface Hosted {
  screen(): string;
  type(text: string): void;
  signal(): Promise<void>;
  exited: Promise<number>;
  kill(): void;
}

/** The TUI in a pseudo-terminal of Unix, where this process sends it the signal. */
function terminal(
  command: string[],
  env: Record<string, string | undefined>,
  signal: NodeJS.Signals,
): Hosted {
  let screen = "";
  const decoder = new TextDecoder();
  const pty = new Bun.Terminal({
    cols: 120,
    rows: 40,
    data(_terminal, data) {
      screen += decoder.decode(data, { stream: true });
    },
  });
  const child = Bun.spawn(command, { env, terminal: pty });
  return {
    screen: () => screen,
    type: (text) => void pty.write(text),
    signal: async () => void child.kill(signal),
    exited: child.exited,
    kill() {
      child.kill("SIGKILL");
      pty.close();
    },
  };
}

/** The TUI in a console of Windows of its own, which reads its keys from a pipe and writes its screen to another.
 * Windows has no signal that a process sends to another, so a helper makes the console and sends Ctrl+Break to it,
 * as a user does at the keyboard. */
function windowsConsole(
  command: string[],
  env: Record<string, string | undefined>,
  directory: string,
): Hosted {
  const trigger = join(directory, "break");
  const child = Bun.spawn(
    [process.execPath, join(import.meta.dir, "console-break.ts"), trigger, ...command],
    {
      env,
      stdin: "pipe",
      stdout: "pipe",
      stderr: "inherit",
    },
  );
  let screen = "";
  const decoder = new TextDecoder();
  void (async () => {
    for await (const chunk of child.stdout) screen += decoder.decode(chunk, { stream: true });
  })();
  return {
    screen: () => screen,
    type(text) {
      child.stdin.write(text);
      child.stdin.flush();
    },
    signal: () => writeFile(trigger, ""),
    exited: child.exited,
    kill: () => child.kill(),
  };
}

// Unix ends a process by a signal from another; Windows sends Ctrl+Break to the processes of a console.
const endings = process.platform === "win32" ? ["Ctrl+Break"] : ["SIGHUP", "SIGTERM"];
for (const ending of endings)
  test(`${ending} ends the TUI as its quit does: the draft is saved, commands end, and the record is free`, async () => {
    const directory = await mkdtemp(join(tmpdir(), "furb-signal-"));
    const project = join(directory, "project");
    const pidFile = join(directory, "command.pid");
    await mkdir(project);
    const cli = [process.execPath, join(import.meta.dir, "../src/cli.ts"), "--demo", "--cwd", project];
    const env = { ...process.env, FURB_CONFIG_DIR: join(directory, "config") };
    const tui =
      process.platform === "win32"
        ? windowsConsole(cli, env, directory)
        : terminal(cli, env, ending as NodeJS.Signals);
    try {
      await eventually(() => tui.screen().includes("Ready"), "the first frame");
      tui.type(`!${printPid} > '${pidFile}'; sleep 30\r`);
      const pid = async () => Number(await readFile(pidFile, "utf8").catch(() => ""));
      await eventually(async () => (await pid()) > 0, "the command to start");
      // The submit takes its text from the composer at once, so a draft typed after it stays.
      tui.type("draftmarker");
      await eventually(() => tui.screen().includes("draftmarker"), "the draft on the screen");
      await tui.signal();
      expect(await Promise.race([tui.exited, Bun.sleep(15000).then(() => "still running")])).toBe(0);
      const sessions = join(project, ".furb/sessions");
      const [record] = (await readdir(sessions)).filter(
        (name) => name.endsWith(".jsonl") && !name.endsWith(".changes.jsonl"),
      );
      const view = JSON.parse(await readFile(join(sessions, `${record}.ui.json`), "utf8")) as {
        drafts: Record<string, string>;
      };
      expect(Object.values(view.drafts)).toContain("draftmarker");
      const running = await pid();
      await eventually(() => !alive(running), "the command to end");
      store(join(sessions, record ?? "")).ear.dispose();
    } finally {
      tui.kill();
      await remove(directory);
    }
  }, 60000);
