import type { SessionOptions } from "@furb/engine";

/** The model a session of the TUI asks when neither the operator nor its record names one. */
export const defaultModel = "claude-cli:sonnet";

/** The models of the demo, which it names in the catalog of the crate and asks none of. */
export const demoRoster = ["claude-cli:sonnet", "claude-cli:opus", "claude-cli:haiku", "claude-cli:fable"];

/** What a session asks of its worker, as plain data: the options of the session in it, and whether it is the demo. */
export type EngineOptions = Omit<SessionOptions, "answer" | "operator" | "ears"> & { demo?: boolean };

/** The roster of a session of the TUI: the models the operator names, or the default model, or those of the demo;
 * and the models the operator added to the session since, from the catalog of the crate. */
export function roster(options: EngineOptions, added: readonly string[] = []): string[] {
  const named = options.roster ?? (options.demo ? demoRoster : [defaultModel]);
  return [...new Set([...named, ...added])];
}
