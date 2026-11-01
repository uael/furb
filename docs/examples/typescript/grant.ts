import { open } from "furb";

await using life = await open("budget.jsonl");
const budget = life.chain("budget");
budget.grant({ usd: 0.5 });
const count = budget.prompt("int", "Count the lines of python in this directory.");
await budget.paused();
console.log(`paused at ${budget.spent.toFixed(2)} dollars`);
budget.grant({ usd: 2 });
budget.wake();
console.log(await count);
