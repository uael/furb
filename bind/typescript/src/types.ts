import { stripVTControlCharacters } from "node:util";
import type { ModelThinkingLevel } from "@earendil-works/pi-ai";
import type { Life } from "../index.cjs";

export const efforts = [
  "off",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
] as const satisfies readonly ModelThinkingLevel[];
export function actorParts(actor: string): { model: string; effort: string } {
  const at = actor.lastIndexOf("/");
  return at < 0
    ? { model: actor, effort: "off" }
    : { model: actor.slice(0, at), effort: actor.slice(at + 1) };
}
export const shapes = ["str", "None", "bool", "int", "float", "list", "dict"] as const;

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
export function display(value: unknown): string {
  if (typeof value === "string") return value;
  return JSON.stringify(value, null, 2) ?? "None";
}
export function safeText(value: string): string {
  // Text from files and processes must not become terminal control sequences.
  // biome-ignore lint/suspicious/noControlCharactersInRegex: Remove terminal control bytes from displayed text.
  return stripVTControlCharacters(value).replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g, "");
}
