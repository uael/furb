/**
 * The walk of the audit of the TUI, which the skill converge-tui runs: tui/scenarios/walk.ts drives a life of a
 * scripted model, then each record that FURB_CONVERGE_RECORDS names, one path on each line, a copy that opens
 * read-only, with keys, clicks and sizes, and writes each frame with the action that led to it, and walk.json, in the
 * folder FURB_WALK_OUT. It prints each finding of its probes, and it needs the program furb of the checkout, built.
 */
import { walk } from "../scenarios/walk.ts";
import { home } from "./stage.ts";

// The home of the user is a folder of its own, so that the preferences of the walk never touch those of the owner.
await home("furb-walk");
const out = process.env.FURB_WALK_OUT ?? "";
const records = (process.env.FURB_CONVERGE_RECORDS ?? "").split("\n").filter(Boolean);
const { steps, findings } = await walk(out, records);
console.log(`${steps.length} steps, ${findings.length} findings`);
for (const one of findings) console.log(`${one.step}\t${one.probe}\t${one.detail}`);
process.exit(0);
