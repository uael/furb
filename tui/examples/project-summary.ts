import type { ExtensionAPI } from "../src/extensions.ts";

export default function setup(api: ExtensionAPI): void {
  api.registerCommand("project-summary", {
    label: "Summarize this project",
    description: "Read the README and ask for a short summary",
    async run(_argument, context) {
      await context.submit("Read @README.md and give a short project summary.");
      context.notify("Project summary requested.");
    },
  });
}
