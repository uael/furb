import type { LiveAct, TuiContext, TuiPart } from "@furb/engine";
import { dollars, share } from "../format.ts";

/** The grant of a chain that stands: the last one that lives. */
const standing = (acts: readonly LiveAct[], chain: string) =>
  acts.findLast((act) => act.kind === "grant" && act.on === chain && !act.done);

/** A grant on the chain on screen, which the session refuses when the grant ended at once. */
async function grant(context: TuiContext, usd: number | null, part: number | null): Promise<void> {
  const id = String(await context.call("grant", [usd, part]));
  const got = await context.life.outcome(id);
  if (got.done) throw new Error(JSON.stringify(got.value));
}

/** The part of the grant extension for the TUI: a ceiling of dollars or of the share of the window, set by /grant and
 * /context, which stands over its chain, is no card, and shows in the sidebar as a row and a mark on the meter. */
export default function grants(): TuiPart {
  return {
    commands: {
      grant: {
        label: "Set budget",
        argument: "<dollars>",
        detail: "Pause at a dollar ceiling",
        values: () =>
          ["1", "2", "5", "10", "20"].map((value) => ({
            value,
            detail: `Pause this chain at ${dollars(Number(value))}`,
          })),
        async run(argument, context) {
          const amount = Number(argument);
          if (!argument || !Number.isFinite(amount) || amount < 0)
            throw new Error("Use /grant followed by a dollar amount.");
          await grant(context, amount, null);
          context.notify(`Budget set to ${dollars(amount)}. Use /wake if paused.`);
        },
      },
      context: {
        label: "Set context ceiling",
        argument: "<fraction>",
        detail: "Pause at a share of the model window",
        values: () =>
          ["0.5", "0.7", "0.8", "0.9"].map((value) => ({
            value,
            detail: `Pause this chain at ${share(Number(value))}`,
          })),
        async run(argument, context) {
          const amount = Number(argument);
          if (!argument || !Number.isFinite(amount) || amount < 0 || amount > 1)
            throw new Error("Use /context with a number from 0 to 1.");
          await grant(context, null, amount);
        },
      },
    },
    acts: {
      grant: {
        subject: (act) =>
          [
            act.words[0] === null ? "" : `${dollars(Number(act.words[0]))} ceiling`,
            act.words[1] === null ? "" : share(Number(act.words[1])),
          ]
            .filter(Boolean)
            .join("  "),
        standing: true,
        hidden: true,
        fields: ["dollar ceiling", "context ceiling"],
      },
    },
    quiet: ["ledger"],
    sidebar({ acts, chain }) {
      const held = standing(acts, chain);
      if (!held) return undefined;
      const [usd, part] = held.words;
      return {
        rows:
          usd === null || usd === undefined
            ? []
            : [{ name: "Ceiling", value: dollars(Number(usd)), tone: "muted" }],
        ...(part === null || part === undefined
          ? {}
          : {
              meter: { mark: Number(part), tip: `, pauses at ${Number((Number(part) * 100).toFixed(1))}%` },
            }),
      };
    },
  };
}
