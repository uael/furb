import type { TuiPart } from "@furb/engine";
import { fileReferences } from "../files.ts";

/** The part of the files extension for the TUI: a read and a cd of the chain on screen, the path of a note of a read
 * or a write as a reference, and a read of each file that a message names with @ before the message is sent. */
export default function files(): TuiPart {
  return {
    commands: {
      read: {
        label: "Read file",
        argument: "<path>",
        detail: "Show a file to this chain",
        paths: true,
        values: (context) => (context.projectFiles() ?? []).map((path) => ({ value: path, detail: "" })),
        async run(path, context) {
          context.track(String(await context.call("rung", [`read(${JSON.stringify(path)})`])));
          context.show("feed");
        },
      },
      cd: {
        label: "Change directory",
        argument: "<path>",
        detail: "Change this chain's working directory",
        paths: true,
        values: (context) =>
          [
            ...new Set(
              (context.projectFiles() ?? []).flatMap((path) =>
                path.includes("/") ? [path.slice(0, path.lastIndexOf("/"))] : [],
              ),
            ),
          ]
            .sort()
            .map((path) => ({ value: path, detail: "" })),
        run: async (path, context) =>
          context.track(String(await context.call("rung", [`cd(${JSON.stringify(path)})`]))),
      },
    },
    paths: ["read", "write"],
    async prompting(message, context) {
      for (const path of await fileReferences(message, context.directory))
        await context.life.result(String(await context.call("rung", [`read(${JSON.stringify(path)})`])));
    },
  };
}
