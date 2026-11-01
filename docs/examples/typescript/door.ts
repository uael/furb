import { open } from "furb";

await using life = await open("door.jsonl");
const release = life.prompt("str", "Release version 2 in one hour.");
await release.step();
console.log(release.plan);
release.edit(release.plan.replace("await wait(3600)\n", ""));
console.log(await release);
