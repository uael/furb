import { builtinModels } from "@earendil-works/pi-ai/providers/all";
import type { SessionOptions } from "@furb/engine";
import { type ClaudeOptions, claudeProvider } from "@furb/engine/claude";

/** The model a session of the TUI asks when neither the operator nor its record names one. */
export const defaultModel = "claude-cli:sonnet";

/** What a session asks of its worker, as plain data: the options of the session in it, and the Claude CLI it
 * reaches. */
export type EngineOptions = Omit<SessionOptions, "models"> & { demo?: boolean; claude?: ClaudeOptions };

/** The models of the TUI: every provider of pi-ai, with the Claude CLI added to them as a provider of its own. */
export function hostModels(claude?: ClaudeOptions) {
  const cli = claudeProvider(claude);
  const models = builtinModels();
  models.setProvider(cli.provider);
  return {
    models,
    /** What the TUI offers when the operator names no roster: the models of the Claude CLI, the default first. */
    roster: [
      defaultModel,
      ...cli.provider
        .getModels()
        .map((model) => `${model.provider}:${model.id}`)
        .filter((name) => name !== defaultModel),
    ],
    dispose: () => cli.dispose(),
  };
}
