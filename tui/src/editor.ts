import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { CliRenderer } from "@opentui/core";
import { shell } from "furb";

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
    const child = Bun.spawn([shell(), "-c", `exec ${editor} "$1"`, "furb-editor", path], {
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

/** The command that opens a file or URL in the application the system gives its type. Encoding the Windows command
 * keeps an ampersand in a URL query from becoming another shell command. */
export function opener(platform: NodeJS.Platform, path: string): string[] {
  if (platform === "darwin") return ["open", path];
  if (platform === "win32") {
    const script = `Start-Process -FilePath '${path.replaceAll("'", "''")}'`;
    return [
      "powershell.exe",
      "-NoProfile",
      "-NonInteractive",
      "-EncodedCommand",
      Buffer.from(script, "utf16le").toString("base64"),
    ];
  }
  return ["xdg-open", path];
}

/** Open a file in the application the system gives its type, and raise what the command said when it fails. */
export async function openFile(path: string): Promise<void> {
  const child = Bun.spawn(opener(process.platform, path), { stdout: "ignore", stderr: "pipe" });
  if (await child.exited) throw new Error(await new Response(child.stderr).text());
}
