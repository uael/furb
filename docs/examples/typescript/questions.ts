import { createInterface } from "node:readline";
import { open } from "@furb/engine";

await using life = await open("deploy.jsonl");
const terminal = createInterface({ input: process.stdin });
const lines = terminal[Symbol.asyncIterator]();
const deploy = life.prompt("str", "Deploy version 2. Ask me before you deploy.");
for await (const question of life.questions({ until: deploy })) {
  process.stdout.write(`${question.message} `);
  question.answer((await lines.next()).value);
}
terminal.close();
console.log(await deploy);
