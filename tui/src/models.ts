import { models, type SessionOptions } from "@furb/engine";

/** The models of the demo, which it names in the catalog of the crate and asks none of. */
export const demoRoster = ["claude-cli:sonnet", "claude-cli:opus", "claude-cli:haiku", "claude-cli:fable"];

/** What a session asks of its worker, as plain data: the options of the session in it, and whether it is the demo. */
export type EngineOptions = Omit<SessionOptions, "answer" | "operator" | "ears"> & { demo?: boolean };

/** The roster of a session of the TUI: the models the operator names, or those of the demo, or the model that the
 * catalog of the crate offers first, which a life of the crate stands on when its host names none; and the models
 * the operator added to the session since, from the catalog. */
export function roster(options: EngineOptions, added: readonly string[] = []): string[] {
  const named =
    options.roster ??
    (options.demo
      ? demoRoster
      : models(options.claude)
          .slice(0, 1)
          .map(({ name }) => name));
  return [...new Set([...named, ...added])];
}
