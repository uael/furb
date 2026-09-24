import type { LiveAct } from "../activity.js";
import type { Spawned, WorldContext, WorldPart } from "../extension.js";
import { type Fact, isQuestion } from "../types.js";

/** The streams of a command as its act shows them while it runs: the text each has said so far. */
export interface Streams {
  code: number | null;
  stdout: { path: string; content: string };
  stderr: { path: string; content: string };
}

/** The part of the bash extension: a command of the shell, started in the directory its chain stands in, its streams
 * said as they come, fed while it runs, and ended at its timeout and at a control over it. */
export default function bash(context: WorldContext): WorldPart {
  /** Each command made, by its act, until it has exited. */
  const made = new Map<string, Fact>();
  const running = new Map<string, Spawned>();
  /** What was fed to a command before its process stood, in the order it was said. */
  const feeds = new Map<string, (string | null)[]>();
  /** The commands ended by a control, whose exit the World said already. */
  const over = new Set<string>();
  /** Whether the stderr of each command flows into its stdout, as the act answered the World. */
  const merged = new WeakMap<LiveAct, boolean>();
  function feed(id: string, text: string | null): void {
    const child = running.get(id)?.child;
    if (!child) feeds.set(id, [...(feeds.get(id) ?? []), text]);
    else if (text === null) child.stdin.end();
    else child.stdin.write(text);
  }
  function run(
    id: string,
    here: string,
    command: string,
    fed: boolean,
    timeout: number | null,
    mixed: boolean,
  ): void {
    const spawned = context.spawn(command, { cwd: context.at(here), merged: mixed, fed, timeout });
    const { child } = spawned;
    running.set(id, spawned);
    for (const text of feeds.get(id) ?? []) feed(id, text);
    feeds.delete(id);
    for (const name of ["stdout", "stderr"] as const) {
      child[name].setEncoding("utf8");
      child[name].on("data", (text: string) => context.speak("out", id, text, name));
    }
    child.on("error", (error) => {
      running.delete(id);
      context.close(context.refused(`${JSON.stringify(command)} did not start: ${error.message}`), id);
    });
    child.on("close", (code) => {
      running.delete(id);
      if (!over.delete(id)) context.speak("exited", id, spawned.late() ? null : code);
    });
  }
  return {
    kinds: ["bash"],
    *hears(fact: Fact) {
      const [kind, id] = fact;
      if (kind === "bash" && isQuestion(kind, id)) made.set(id, fact);
      else if (kind === "start" && made.has(id)) {
        const [, , , on, command, fed, timeout] = made.get(id) as Fact;
        const here = yield* context.where(String(on));
        const [, mixed] = (yield { verb: "ask", args: ["merged", on, id] }) as [unknown, unknown];
        run(
          id,
          here,
          String(command),
          Boolean(fed),
          typeof timeout === "number" ? timeout : null,
          Boolean(mixed),
        );
      } else if (kind === "feed" && made.has(id)) feed(id, fact[3] === null ? null : String(fact[3]));
      else if (kind === "cancel" || kind === "close") {
        for (const [one, spawned] of [...running])
          if (yield { verb: "covers", args: [fact, one] }) {
            over.add(one);
            spawned.stop();
            running.delete(one);
            yield ["exited", one, null];
          }
      } else if (kind === "exited" || (kind === "done" && made.has(id) && !running.has(id))) {
        made.delete(id);
        feeds.delete(id);
      }
    },
    live: {
      born: (act): Streams => ({
        code: null,
        stdout: { path: `${act.id}/stdout`, content: "" },
        stderr: { path: `${act.id}/stderr`, content: "" },
      }),
      hears: (fact, acts) => {
        const [kind, id, by] = fact;
        if (kind === "done" && isQuestion("merged", id)) {
          const act = acts.get(by);
          if (act?.kind === "bash") merged.set(act, Boolean(fact[3]));
          return undefined;
        }
        const act = acts.get(id);
        if (act?.kind !== "bash" || act.done) return undefined;
        const value = act.value as Streams;
        if (kind === "exited") {
          value.code = typeof fact[3] === "number" ? fact[3] : null;
          return act;
        }
        if (kind !== "out") return undefined;
        const stream = fact[4] === "stderr" && merged.get(act) === false ? value.stderr : value.stdout;
        stream.content += String(fact[3]);
        return act;
      },
    },
    dispose() {
      for (const spawned of running.values()) spawned.stop();
      running.clear();
    },
  };
}
