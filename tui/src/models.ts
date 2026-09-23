import { builtinModels } from "@earendil-works/pi-ai/providers/all";
import type { WorldOptions } from "@furb/engine";
import { type ClaudeOptions, claudeProvider } from "@furb/engine/claude";

/** What a session asks of its worker, as plain data: the options of its World, and the Claude CLI it reaches. */
export type EngineOptions = Omit<WorldOptions, "models"> & { demo?: boolean; claude?: ClaudeOptions };

/** The models of the TUI: every provider of pi-ai, with the Claude CLI added to them as a provider of its own. */
export function hostModels(claude?: ClaudeOptions) {
  const cli = claudeProvider(claude);
  const models = builtinModels();
  models.setProvider(cli.provider);
  return {
    models,
    /** What the TUI offers when the operator names no roster: the models of the Claude CLI, the first by default. */
    roster: cli.provider.getModels().map((model) => `${model.provider}:${model.id}`),
    dispose: () => cli.dispose(),
  };
}
