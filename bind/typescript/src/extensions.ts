import { pathToFileURL } from "node:url";
import type { Extension } from "../index.cjs";
import bash from "./builtin/bash.js";
import files from "./builtin/files.js";
import type { WorldExtension } from "./extension.js";

/** The parts for a World in TypeScript of the builtin extensions, by the name of their extension. Grant has none. */
export const builtinWorldParts: Readonly<Record<string, WorldExtension>> = { files, bash };

/** The default export of the file of an extension, which must be a function: its part for a World or for a TUI. */
export async function imported<T>(extension: Extension, file: string, part: string): Promise<T> {
  const module = (await import(pathToFileURL(file).href)) as { default?: unknown };
  if (typeof module.default !== "function")
    throw new Error(`The ${part} part of the extension ${extension.name}, ${file}, exports no default function.`);
  return module.default as T;
}

/** The parts for a World of the extensions, by name: the part of each builtin, the part the host gives, and the file
 * each other extension names for a World in TypeScript, imported. */
export async function loadWorldParts(
  extensions: readonly Extension[],
  given: Readonly<Record<string, WorldExtension>> = {},
): Promise<Record<string, WorldExtension>> {
  const parts: Record<string, WorldExtension> = { ...given };
  for (const one of extensions)
    if (!parts[one.name] && !one.builtin && one.world.ts)
      parts[one.name] = await imported<WorldExtension>(one, one.world.ts, "World");
  return parts;
}
