import { stripVTControlCharacters } from "node:util";
import type { Life } from "../index.cjs";

export type Turn = Awaited<ReturnType<Life["turns"]>>[number];
export type Tag = Exclude<Turn[1][number], string>;
export type Fact = Awaited<ReturnType<Life["send"]>>;
export type Entry = [string, Fact, unknown?];
export type Usage = NonNullable<Turn[2]>;
export interface TextValue {
  is?: "Text";
  path: string;
  content: string;
}
export interface OperatorPrompt {
  id: string;
  shape: string;
  message: string;
  resolve(value: unknown): void;
  reject(error: Error): void;
}

export function isTag(value: unknown): value is Tag {
  return (
    Array.isArray(value) && value.length === 3 && typeof value[0] === "string" && Array.isArray(value[1])
  );
}
export function renderTag([name, held, body]: Tag): string {
  let attrs = "";
  const parts: string[] = [];
  for (const [key, value] of held) {
    const said = typeof value === "string" ? value : python(value);
    if (said.includes("\n") || said.includes('"')) parts.push(`<${key}>\n${said}\n</${key}>`);
    else attrs += ` ${key}="${said}"`;
  }
  if (typeof body === "string") parts.push(body);
  else if (Array.isArray(body)) parts.push(...body.map((one) => (isTag(one) ? renderTag(one) : python(one))));
  else if (body !== null) parts.push(python(body));
  const inner = parts.filter(Boolean).join("\n");
  return inner ? `<${name}${attrs}>\n${inner}\n</${name}>` : `<${name}${attrs}/>`;
}
export function rendered(content: Turn[1]): string {
  return content.map((one) => (typeof one === "string" ? one : renderTag(one))).join("\n");
}
export function python(value: unknown): string {
  if (value === null || value === undefined) return "None";
  if (typeof value === "boolean") return value ? "True" : "False";
  if (typeof value === "string") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(python).join(", ")}]`;
  if (typeof value === "object")
    return `{${Object.entries(value)
      .map(([key, item]) => `${python(key)}: ${python(item)}`)
      .join(", ")}}`;
  return String(value);
}
export function display(value: unknown): string {
  if (typeof value === "string") return value;
  return JSON.stringify(value, null, 2) ?? "None";
}
export function safeText(value: string): string {
  // Text from files and processes must not become terminal control sequences.
  // biome-ignore lint/suspicious/noControlCharactersInRegex: Remove terminal control bytes from displayed text.
  return stripVTControlCharacters(value).replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g, "");
}
