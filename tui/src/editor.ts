import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { shell } from "@furb/engine";
import type { CliRenderer } from "@opentui/core";

export async function externalEditor(
  renderer: Pick<CliRenderer, "suspend" | "resume">,
  text: string,
  python: boolean,
  cwd: string,
): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), "furb-editor-"));
  const path = join(directory, python ? "draft.py" : "draft.md");
  await writeFile(path, text, { mode: 0o600 });
  renderer.suspend();
  try {
    const editor = process.env.VISUAL || process.env.EDITOR || "vi";
    const child = Bun.spawn([shell, "-c", `exec ${editor} "$1"`, "furb-editor", path], {
      cwd,
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    });
    if (await child.exited) throw new Error("The editor exited without saving the draft to furb.");
    return await readFile(path, "utf8");
  } finally {
    renderer.resume();
    await rm(directory, { recursive: true, force: true });
  }
}
