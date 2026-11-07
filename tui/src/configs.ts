import type { Layer } from "furb/protocol";

/** A key of the configs as the host gives it: the value that wins for it, the layer that gives that value, or none for
 * a default or an object of furb, and the property that a schema gives the key, with the extension that adds it. */
export interface Setting {
  key: string[];
  value: unknown;
  layer?: Layer;
  property?: {
    type: "boolean" | "integer" | "number" | "string" | "array" | "object";
    default: unknown;
    description: string;
    enum?: unknown[];
  };
  skill?: string;
}

/** The text of a value in the form of a key: a key that the schema types as a string shows its text as it is, and any
 * other value shows as JSON, so a string with no schema shows in quotes. */
export const shown = (value: unknown, property?: Setting["property"]): string =>
  property?.type === "string" && typeof value === "string" ? value : (JSON.stringify(value) ?? "");

/** The value that the operator typed for a key, the inverse of shown: a key that the schema types as a string takes
 * its text literally, so a numeric text, a quote and the text null stay strings, and any other key takes its JSON, or
 * the text itself when it is no JSON. */
export const typed = (text: string, property?: Setting["property"]): unknown => {
  if (property?.type === "string") return text;
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
};
