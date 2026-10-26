import type { SetupInfo } from "furb";

/** One check of the first run: what it checks, whether it stands, what it found, and how to fix it when it does not.
 * A check that is optional stands as a choice, and nothing is wrong when it is not set. */
export interface Check {
  label: string;
  state: "ready" | "missing" | "optional";
  found: string;
  fix?: string;
}

/** The providers whose keys the fix names first, as most users hold one of them. */
const usual = ["anthropic", "openai", "google", "openrouter"];

/** How many models of the catalog answer: each one that it offers, less those of a claude command line that is not
 * signed in, since it answers no prompt. */
export function answering(info: SetupInfo, catalog: readonly (readonly [string, ...unknown[]])[]): number {
  return catalog.filter(([name]) => info.signed || !name.startsWith("claude-cli:")).length;
}

/** The checks of the first run, from what the catalog of the crate found on this machine and how many models it
 * offers: the claude command line, the other providers, the local server, and the models that answer. */
export function checks(info: SetupInfo, offered: number): Check[] {
  const providers = info.providers.filter((one) => one.enabled !== false);
  const disabled = (name: string) => info.providers.some((one) => one.provider === name && !one.enabled);
  const other = providers.filter((one) => !["local", "claude-cli"].includes(one.provider));
  const ready = other.filter((one) => one.offered > 0);
  const named = [
    ...usual.flatMap((name) => other.filter((one) => one.provider === name)),
    ...other.filter((one) => !usual.includes(one.provider)),
  ].flatMap((one) => one.env.find((name) => name.endsWith("_API_KEY")) ?? one.env.slice(0, 1));
  const configured = providers.find((one) => one.provider === "local");
  const local = info.local;
  return [
    disabled("claude-cli")
      ? { label: "Claude command line", state: "optional", found: "Disabled in the provider settings." }
      : info.claude && info.signed
        ? { label: "Claude command line", state: "ready", found: "Found. Your Claude plan answers." }
        : info.claude
          ? {
              label: "Claude command line",
              state: "missing",
              found: "Found, but not signed in.",
              fix: "Run claude in a terminal, sign in, then press r here to check again.",
            }
          : {
              label: "Claude command line",
              state: "missing",
              found: "Not found.",
              fix: "Install Claude Code from claude.com/claude-code and run claude once to sign in, or set FURB_CLAUDE_BIN to its path.",
            },
    ready.length
      ? {
          label: "Other providers",
          state: "ready",
          found: `Models available from ${ready.map((one) => one.provider).join(", ")}.`,
        }
      : {
          label: "Other providers",
          state: "missing",
          found: "No model available.",
          fix: `Add a provider with /providers, or edit the config and run /reload.${named.length ? ` Environment key names: ${named.slice(0, 4).join(", ")}.` : ""}`,
        },
    disabled("local")
      ? { label: "Local server", state: "optional", found: "Disabled in the provider settings." }
      : configured
        ? configured.offered
          ? {
              label: "Local server",
              state: "ready",
              found: `${configured.offered} ${configured.offered === 1 ? "model" : "models"} in the provider settings.`,
            }
          : {
              label: "Local server",
              state: "missing",
              found: "No model from the local provider settings.",
              fix: "Check this provider's URL, model and key in the config, then run /reload.",
            }
        : !local
          ? {
              label: "Local server",
              state: "optional",
              found: "Not set.",
              fix: "To use a model on this machine, start a server such as Ollama, then set FURB_LOCAL_URL=http://localhost:11434.",
            }
          : local.offered
            ? { label: "Local server", state: "ready", found: `${local.offered} models at ${local.address}.` }
            : {
                label: "Local server",
                state: "missing",
                found: local.failed
                  ? `No answer from ${local.address}.`
                  : `No model for chat at ${local.address}.`,
                fix: local.failed
                  ? "Start the server, or set FURB_LOCAL_URL to its address."
                  : "Load a model for chat on the server.",
              },
    offered
      ? {
          label: "Models",
          state: "ready",
          found: `${offered} ${offered === 1 ? "model answers" : "models answer"}.`,
        }
      : {
          label: "Models",
          state: "missing",
          found: "No model answers.",
          fix: "Fix one check above, then press r here to check again, or run /reload. Watch a life needs no model.",
        },
  ];
}
