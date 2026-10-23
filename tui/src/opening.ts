import { statSync } from "node:fs";
import { basename } from "node:path";
import { BoxRenderable, type CliRenderer, type KeyEvent, TextRenderable } from "@opentui/core";
import { shortenHome } from "./files.ts";
import { setTheme, spacing, type ThemeName, theme } from "./theme.ts";
import { bold, styled } from "./ui.ts";

/** What the first frame says it opens: a record, with its size, or a new life. */
export function openingLine(record?: string): string {
  if (!record) return "Opening a new life";
  const bytes = statSync(record, { throwIfNoEntry: false })?.size ?? 0;
  const size = bytes / 2 ** 20;
  return `Opening ${shortenHome(record)}, ${size.toFixed(size < 10 ? 1 : 0)} MB`;
}

/** The first frame of the TUI, which it draws before it reads the record of the life: the top line, with the name and
 * the directory of the life, and what it opens. A record of a long life takes seconds to read, and this frame shows
 * at once that it loads, and Control C quits while it loads. It gives the function that removes the frame. */
export function opening(
  renderer: CliRenderer,
  {
    directory,
    record,
    name,
    quit,
  }: { directory: string; record?: string; name: ThemeName; quit: () => void },
): () => void {
  setTheme(name);
  const root = new BoxRenderable(renderer, {
    flexDirection: "column",
    width: "100%",
    height: "100%",
    paddingX: spacing.between,
  });
  root.add(
    new TextRenderable(renderer, {
      content: styled([
        [basename(directory), theme.text, bold],
        [`   ${shortenHome(directory)}`, theme.faint],
      ]),
      height: spacing.bar,
      truncate: true,
    }),
  );
  const middle = new BoxRenderable(renderer, { flexGrow: 1, justifyContent: "center", alignItems: "center" });
  middle.add(new TextRenderable(renderer, { content: openingLine(record), fg: theme.muted, truncate: true }));
  root.add(middle);
  renderer.root.add(root);
  renderer.requestRender();
  const key = (event: KeyEvent) => {
    if (event.ctrl && event.name === "c") quit();
  };
  renderer.keyInput.on("keypress", key);
  return () => {
    renderer.keyInput.off("keypress", key);
    root.destroyRecursively();
  };
}
