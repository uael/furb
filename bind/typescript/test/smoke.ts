import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { boot } from "../src/index.ts";

const cwd = await mkdtemp(join(tmpdir(), "furb-ts-smoke-"));
const session = boot({ cwd, model: "claude-cli:sonnet", effort: "low" });
session.world?.on("fault", (error) => {
  console.error(error);
  session.life.cancel(session.life.root);
});
try {
  const answer = await session.life.prompt<string>(
    "str",
    "Return exactly the string: native TypeScript ready",
  );
  if (answer !== "native TypeScript ready") throw new Error(`Unexpected answer: ${JSON.stringify(answer)}`);
  const turns = session.life.turns();
  console.log(
    JSON.stringify({
      answer,
      turns: turns.length,
      usage: turns.filter((turn) => turn[2]).map((turn) => turn[2]),
    }),
  );
} finally {
  await session.dispose();
  await rm(cwd, { recursive: true });
}
