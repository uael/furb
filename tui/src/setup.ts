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

/** The checks of the first run, from what the catalog of the crate found on this machine and how many models it
 * offers: the claude command line, the keys in the environment, the local server, and the models that answer. */
export function checks(info: SetupInfo, offered: number): Check[] {
  const ready = info.providers.filter((one) => one.offered > 0);
  const named = [
    ...usual.flatMap((name) => info.providers.filter((one) => one.provider === name)),
    ...info.providers.filter((one) => !usual.includes(one.provider)),
  ].flatMap((one) => one.env.find((name) => name.endsWith("_API_KEY")) ?? one.env.slice(0, 1));
  const [first = "ANTHROPIC_API_KEY", ...others] = named;
  const local = info.local;
  return [
    info.claude
      ? { label: "Claude command line", state: "ready", found: "Found. Your Claude plan answers." }
      : {
          label: "Claude command line",
          state: "missing",
          found: "Not found.",
          fix: "Install Claude Code from claude.com/claude-code and run claude once to sign in, or set FURB_CLAUDE_BIN to its path.",
        },
    ready.length
      ? {
          label: "Keys",
          state: "ready",
          found: `A key for ${ready.map((one) => one.provider).join(", ")}.`,
        }
      : {
          label: "Keys",
          state: "missing",
          found: "No key in the environment.",
          fix: `Set one in your shell, as export ${first}=your-key, then start furb again.${others.length ? ` Other names: ${others.slice(0, 3).join(", ")}.` : ""}`,
        },
    !local
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
          fix: "Fix one check above, then start furb again. Watch a life needs no model.",
        },
  ];
}
