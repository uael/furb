import type { SessionOptions } from "furb";

/** The models of the demo, which it names in the catalog of the crate and asks none of. */
export const demoRoster = ["claude-cli:sonnet", "claude-cli:opus", "claude-cli:haiku", "claude-cli:fable"];

/** What a session asks of its worker, as plain data: the options of the session in it, whether it is the demo, and
 * whether it is the demo that the operator watches, which streams each turn and spends nothing. */
export type EngineOptions = Omit<SessionOptions, "answer" | "operator" | "ears"> & {
  demo?: boolean;
  watch?: boolean;
};

/** The roster of a session of the TUI: the models the operator names, or those of the demo, and the models the
 * operator added to the session since, from the catalog; or nothing, when the operator names none outside the demo,
 * so the session offers every model that the catalog of the crate offers. */
export function roster(options: EngineOptions, added: readonly string[] = []): string[] | undefined {
  const named = options.roster ?? (options.demo ? demoRoster : undefined);
  return named && [...new Set([...named, ...added])];
}
