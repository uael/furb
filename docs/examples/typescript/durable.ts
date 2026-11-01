import { open } from "furb";

await using life = await open("release.jsonl");
console.log(await life.prompt("str", "Plan the release of version 2."));
