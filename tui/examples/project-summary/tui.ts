import type { TuiPart } from "@furb/engine";

/** An extension with a part for the TUI alone: one command, which asks the chain on screen for a summary of the
 * project. It imports types alone, since its file runs from where the config names it, beside no copy of the TUI. */
export default function projectSummary(): TuiPart {
  return {
    commands: {
      "project-summary": {
        label: "Summarize this project",
        detail: "Read the README and ask for a short summary",
        async run(_argument, context) {
          await context.submit("Read @README.md and give a short project summary.");
          context.notify("Project summary requested.");
        },
      },
    },
  };
}
