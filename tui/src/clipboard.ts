import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { imageType } from "@furb/engine";

export async function clipboardImage<T>(use: (path: string) => Promise<T>): Promise<T> {
  // Each reader prints the image on the clipboard: as base64 text through macOS AppKit and through the .NET of
  // Windows PowerShell, and as bytes through wl-paste and xclip.
  const readers: [command: string[], base64: boolean][] =
    process.platform === "darwin"
      ? [
          [
            [
              "osascript",
              "-l",
              "JavaScript",
              "-e",
              "ObjC.import('AppKit'); const image = $.NSImage.alloc.initWithPasteboard($.NSPasteboard.generalPasteboard); if (image.isNil()) { '' } else { const rep = $.NSBitmapImageRep.imageRepWithData(image.TIFFRepresentation); const data = rep.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $({})); ObjC.unwrap(data.base64EncodedStringWithOptions(0)); }",
            ],
            true,
          ],
        ]
      : process.platform === "win32"
        ? [
            [
              [
                "powershell",
                "-NoProfile",
                "-NonInteractive",
                "-STA",
                "-Command",
                "Add-Type -AssemblyName System.Windows.Forms, System.Drawing; $image = [System.Windows.Forms.Clipboard]::GetImage(); if ($image) { $stream = New-Object System.IO.MemoryStream; $image.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png); [Convert]::ToBase64String($stream.ToArray()) }",
              ],
              true,
            ],
          ]
        : ["image/png", "image/jpeg", "image/gif", "image/webp"].flatMap((mime): [string[], boolean][] => [
            [["wl-paste", "--no-newline", "--type", mime], false],
            [["xclip", "-selection", "clipboard", "-t", mime, "-o"], false],
          ]);
  let bytes: Buffer | undefined;
  for (const [command, base64] of readers) {
    try {
      const binary = Bun.which(command[0] ?? "", { PATH: process.env.PATH });
      if (!binary) continue;
      const child = Bun.spawn([binary, ...command.slice(1)], { stdout: "pipe", stderr: "pipe" });
      const output = Buffer.from(await new Response(child.stdout).arrayBuffer());
      if (await child.exited) continue;
      bytes = base64 ? Buffer.from(output.toString().trim(), "base64") : output;
      if (bytes.length) break;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
  }
  if (!bytes?.length) throw new Error("No clipboard image is available. Use /image with a file path.");
  const type = imageType(bytes);
  const directory = await mkdtemp(join(tmpdir(), "furb-clipboard-"));
  const path = join(directory, `pasted-image.${type.extension}`);
  try {
    await writeFile(path, bytes, { mode: 0o600 });
    return await use(path);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}
