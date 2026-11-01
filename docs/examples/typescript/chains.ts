import { open } from "furb";

await using life = await open("notes.jsonl", { roster: ["claude-cli:opus", "claude-cli:sonnet"] });
const reviewer = life.chain("reviewer", { model: "claude-cli:sonnet" });
const draft = await life.prompt("str", "Write the release notes of version 2.");
console.log(await reviewer.prompt("str", `Review these release notes:\n${draft}`));
for (const chain of life.chains()) console.log(chain.label, "under", chain.parent?.label);
