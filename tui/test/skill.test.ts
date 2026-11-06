import { expect, test } from "bun:test";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { delimiter, dirname, join } from "node:path";
import type { Frame } from "furb/protocol";
import { Connection, daemonPlace, type Fact } from "furb/skill";
import { remove } from "../../bind/typescript/test/processes.ts";
import { ownDaemon } from "../script/daemon.ts";
import { daemonFolder } from "../src/client.ts";

/** How long the test waits for each step of the life and of the process of its skill, in milliseconds. */
const PATIENCE = 20000;

/** The manifest of the skill of the test, whose process says where it stands, answers a message, and serves a view. */
const MANIFEST = `---
name: probe
description: Says where its process stands.
metadata:
  furb-word: probe.py
  furb-process: probe.ts
---
A probe.
`;

/** The process of the skill, on the client of the package as its source stands. */
const PROCESS = `import { Skill } from ${JSON.stringify(join(import.meta.dir, "../../bind/typescript/src/skill.ts"))};
const skill = await Skill.attach({ kinds: ["prompt"] });
skill.heard((fact) => {
  if (fact[2] !== "skill/probe" && fact[5] === "ping") void skill.run("pong = 1");
});
skill.served((view) => void view.tree({ id: "said", node: "text", runs: [{ text: "served" }] }));
await skill.prompt("None", "the probe stands", { to: "operator" });
await skill.closed;
process.exit(0);
`;

/** The first thing that a check finds, once it finds one, or a failure that names what the test waited for. */
async function until<T>(what: string, found: () => T | undefined): Promise<T> {
  for (const end = Date.now() + PATIENCE; ; await Bun.sleep(20)) {
    const got = found();
    if (got !== undefined) return got;
    if (Date.now() > end) throw new Error(`No ${what} in ${PATIENCE / 1000} s.`);
  }
}

test.skipIf(process.platform === "win32")(
  "the process of a skill hears a kind that it asked, says acts under its name, and serves a tree to a window",
  async () => {
    const home = await mkdtemp(join(tmpdir(), "furb-skill-"));
    // The cache holds the socket of the daemon, whose path has a bound.
    const cache = await mkdtemp(join("/tmp", "furb-cache-"));
    let ended = async () => {};
    let window: Connection | undefined;
    try {
      const project = join(home, "project");
      const folder = join(project, ".furb", "skills", "probe");
      await mkdir(folder, { recursive: true });
      await writeFile(join(folder, "SKILL.md"), MANIFEST);
      await writeFile(join(folder, "probe.py"), "PROBED = 1\n");
      await writeFile(join(folder, "probe.ts"), PROCESS);
      const env = {
        ...process.env,
        HOME: home,
        XDG_CACHE_HOME: cache,
        FURB_CONFIG_DIR: join(home, "config"),
        // The daemon runs the process of the skill with bun, from its PATH.
        PATH: [dirname(process.execPath), process.env.PATH ?? ""].join(delimiter),
      };
      ended = await ownDaemon(env);
      window = await Connection.dial(daemonPlace(daemonFolder(env)), "window");
      const facts: Fact[] = [];
      const frames: Frame[] = [];
      window.pushed((frame) => {
        frames.push(frame);
        if (frame.frame === "event" && frame.event.event === "fact") facts.push(frame.event.data as Fact);
      });
      const life = join(project, "a.jsonl");
      await window.ask({ verb: "open", life, cwd: project, attach: 0 });
      await window.ask({ verb: "run", life, word: 'act("skill", "", None, "probe")' });
      const said = (kind: string, word: string) =>
        facts.find((one) => one[0] === kind && one[2] === "skill/probe" && one.includes(word));
      await until("message of the process", () => said("prompt", "the probe stands"));
      await window.ask({ verb: "prompt", life, shape: "None", message: "ping", to: "operator" });
      await until("rung of the process", () => said("rung", "pong = 1"));
      const mounted = async () =>
        (
          await window?.ask({
            verb: "view.mount",
            viewer: "v",
            life,
            slot: { slot: "sidebar" },
            skill: "probe",
          })
        )?.answer;
      expect(await mounted()).toBe("done");
      const tree = await until("tree of the view", () => frames.find((one) => one.frame === "view.tree"));
      expect(tree).toEqual({
        frame: "view.tree",
        viewer: "v",
        slot: { slot: "sidebar" },
        version: 1,
        // The daemon gives a run the tone of plain text when the skill names none.
        tree: { id: "said", node: "text", runs: [{ text: "served", tone: "text" }] },
      });
    } finally {
      window?.close();
      await ended();
      await remove(home);
      await remove(cache);
    }
  },
  PATIENCE * 3,
);
