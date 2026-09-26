import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { boot } from "../src/index.ts";

const cwd = await mkdtemp(join(tmpdir(), "furb-ts-smoke-"));
const session = boot({ cwd, model: "claude-cli:sonnet", effort: "low" });
const engine = session.engine;
session.on("fault", (error) => {
  console.error(error);
  engine.cancel(engine.root);
});
try {
  const answer = await engine.prompt("str", {
    message: "Return exactly the string: native TypeScript ready",
    on: engine.root,
  });
  if (answer !== "native TypeScript ready") throw new Error(`Unexpected answer: ${JSON.stringify(answer)}`);
  const turns = engine.turns({ on: engine.root });
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
