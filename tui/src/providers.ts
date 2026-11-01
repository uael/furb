import { mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { saveFile } from "furb";

/** A provider that the settings enable or remove, with no credentials. */
export interface ProviderChoice {
  name: string;
  enabled: boolean;
  models: number;
}

function read(path: string): Record<string, unknown> {
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return {};
    throw error;
  }
  const value = JSON.parse(text);
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error(`The config at ${path} must be an object.`);
  if (
    value.providers !== undefined &&
    (!value.providers || typeof value.providers !== "object" || Array.isArray(value.providers))
  )
    throw new Error(`The providers at ${path} must be an object.`);
  return value;
}

/** Save one project provider and apply it. A refused change restores the previous file. */
export function changeProvider(directory: string, name: string, value: unknown, apply: () => void): void {
  if (!name || /[:/\s]/.test(name)) throw new Error("A provider name must contain no space, colon or slash.");
  const path = join(directory, ".furb", "config.json");
  let before: string | undefined;
  try {
    before = readFileSync(path, "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  const config = read(path);
  const previous = (config.providers as Record<string, unknown> | undefined)?.[name];
  if (typeof value === "boolean" && previous && typeof previous === "object")
    value = { ...previous, enabled: value };
  config.providers = { ...(config.providers as Record<string, unknown> | undefined), [name]: value };
  mkdirSync(dirname(path), { recursive: true });
  saveFile(path, `${JSON.stringify(config, null, 2)}\n`);
  try {
    apply();
  } catch (error) {
    if (before === undefined) rmSync(path);
    else saveFile(path, before);
    throw error;
  }
}
