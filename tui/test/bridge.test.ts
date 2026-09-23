import { expect, test } from "bun:test";
import type { Fact } from "@furb/engine";
import { HostView } from "../src/bridge.ts";

test("the view of the host takes more facts than one call takes as arguments", () => {
  const view = new HostView(async () => null);
  const count = 700_000;
  view.update({
    completed: 0,
    cost: 0,
    directory: "/tmp",
    imageDirectory: "/tmp/images",
    actor: "operator",
    effort: "low",
    roster: [],
    facts: new Array<Fact>(count).fill(["done", "x", "world", null]),
    prompts: [],
    streams: [],
    held: [],
    changes: 0,
    models: [],
  });
  expect(view.facts).toHaveLength(count);
});
