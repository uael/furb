import type { TuiContext, TuiPart } from "@furb/engine";

/** A rung of the operator on the chain on screen, which the session follows in the feed. */
async function rung(context: TuiContext, word: string): Promise<void> {
  context.track(String(await context.call("rung", [word])));
  context.show("feed");
}

/** The part of the skills extension for the TUI: a list of the skills that the World finds, a skill read into the
 * chain on screen, and the skills told again to that chain, which tells what changed. */
export default function skills(): TuiPart {
  return {
    commands: {
      skills: {
        label: "List skills",
        detail: "List the skills that this project and your config hold",
        async run(_argument, context) {
          const [, heard] = (await context.life.call("ask", ["skills", context.chain], {})) as [
            unknown,
            unknown,
          ];
          const names = Array.isArray(heard)
            ? heard.map((one) => String((one as { name?: unknown }).name))
            : [];
          context.notify(
            names.length ? `Skills: ${names.join(", ")}. Use /skill with a name.` : "No skill found.",
          );
        },
      },
      skill: {
        label: "Load skill",
        argument: "<name>",
        detail: "Read a skill into this chain",
        run: (name, context) => rung(context, `skill(${JSON.stringify(name)})`),
      },
      "reload-skills": {
        label: "Reload skills",
        detail: "Tell this chain the skills that are new, changed or gone",
        run: (_argument, context) => rung(context, "skills()"),
      },
    },
  };
}
