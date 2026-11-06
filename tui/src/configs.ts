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
  extension?: string;
}

/** The value that the operator typed for a key: its JSON, or the text itself when it is no JSON, so a string needs no
 * quotes. */
export const typed = (text: string): unknown => {
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
};
