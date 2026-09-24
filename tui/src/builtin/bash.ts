import type { LiveAct, TuiPart } from "@furb/engine";

/** The streams of a command as its act holds them, and its code once it exited. */
interface Streams {
  code?: number | null;
  stdout?: { content: string };
  stderr?: { content: string };
}
const streams = (act: LiveAct): Streams =>
  (act.value && typeof act.value === "object" ? act.value : {}) as Streams;

/** The part of the bash extension for the TUI: a command of the shell, said by /bash or by ! before it, input fed to
 * a command, and the card of a command, which shows the tail of what it prints while it runs and the whole once it
 * opens. A pause holds no command, and a command ends when it exits. */
export default function bash(): TuiPart {
  return {
    commands: {
      bash: {
        label: "Run command",
        argument: "<command>",
        detail: "Stream a shell command",
        keys: "!",
        async run(command, context) {
          context.track(String(await context.call("bash", [command])));
          context.show("feed");
        },
      },
      feed: {
        label: "Feed command",
        argument: "<id> [text]",
        detail: "Send a line of input to a command; no text closes its input",
        values: (context) =>
          context.acts
            .filter(
              (act) => act.on === context.chain && act.kind === "bash" && !act.done && act.words[1] === true,
            )
            .map((act) => ({ value: act.id, detail: String(act.words[0] ?? "").slice(0, 48), more: true })),
        async run(argument, context) {
          const space = argument.indexOf(" ");
          const id = space < 0 ? argument : argument.slice(0, space);
          const text = space < 0 ? "" : argument.slice(space + 1);
          if (!id) throw new Error("Use /feed followed by an act id and text.");
          // A fed text is one line of input, and no text closes the input.
          await context.life.call("ask", ["write", context.chain, `${id}/stdin`, text && `${text}\n`], {});
        },
      },
    },
    prefixes: { "!": "bash" },
    acts: {
      bash: {
        preview(act) {
          if (act.done) return undefined;
          const { stdout, stderr } = streams(act);
          const text = [stdout?.content, stderr?.content].filter(Boolean).join("\n");
          return text ? { text, tail: true } : undefined;
        },
        details(act) {
          const { code, stdout, stderr } = streams(act);
          const both = Boolean(stdout?.content && stderr?.content);
          const [, input, timeout] = act.words;
          return {
            streams: [
              { name: both ? "stdout" : "", content: stdout?.content ?? "" },
              { name: both ? "stderr" : "", content: stderr?.content ?? "", failure: true },
            ].filter((stream) => stream.content),
            notes: [
              ...(act.done
                ? [
                    {
                      label: "exit",
                      text: String(code ?? "timeout"),
                      tone: code === 0 ? ("success" as const) : ("danger" as const),
                    },
                  ]
                : []),
              ...(input === true ? [{ text: "input open", tone: "faint" as const }] : []),
              ...(typeof timeout === "number" && timeout !== 600
                ? [{ text: `times out after ${timeout}s`, tone: "faint" as const }]
                : []),
            ],
          };
        },
        runsPaused: true,
        ends: ["exited"],
      },
    },
  };
}
