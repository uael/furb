import { models, type SessionOptions } from "@furb/engine";

/** The model a session of the TUI asks when neither the operator nor its record names one, while the catalog of the
 * crate offers it. */
export const defaultModel = "claude-cli:sonnet";

/** What a session asks of its worker, as plain data: the options of the session in it, and whether it is the demo. */
export type EngineOptions = Omit<SessionOptions, "answer" | "operator" | "ears"> & { demo?: boolean };

/** The roster of the TUI when the operator names none: every model the catalog of the crate offers, the default
 * model first, which a session takes when its record names no model. */
export function offered(): string[] {
  const names = models().map((model) => model.name);
  return names.includes(defaultModel)
    ? [defaultModel, ...names.filter((name) => name !== defaultModel)]
    : names;
}
