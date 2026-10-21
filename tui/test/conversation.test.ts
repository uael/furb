import { expect, test } from "bun:test";
import type { Turn } from "@furb/engine";
import { conversation } from "../src/conversation.ts";
import type { ActRow } from "../src/session.ts";

test("the open of a rung that its caller wrote shows as its word, under a header that names its maker", () => {
  const act: ActRow = {
    id: "rung1",
    kind: "rung",
    by: "operator",
    on: "chain1",
    words: ["k = 1"],
    done: true,
    value: null,
    paused: false,
  };
  const turns = [["user", "#rung1 by operator\nrung1_word = 'k = 1'", null, null]] as unknown as Turn[];
  expect(conversation(turns, [act]).map((item) => item.type)).toEqual(["python"]);
});
