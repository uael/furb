import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { type Mount, type Node, PROTOCOL } from "@uael/furb/protocol";
import { generated, typesPath } from "../script/protocol.ts";

test("the types of the protocol are the ones that its schema makes", async () => {
  expect(readFileSync(typesPath, "utf8")).toBe(await generated());
});

test("the package gives the types of the protocol and its version", () => {
  const text = (id: string): Node => ({ id, node: "text", runs: [{ text: id, tone: "accent" }] });
  const tree: Node = {
    id: "root",
    node: "box",
    axis: "column",
    gap: 1,
    border: true,
    children: [
      text("t"),
      { id: "l", node: "list", items: [text("i")] },
      { id: "tb", node: "table", header: ["a"], rows: [[text("c")]] },
      { id: "in", node: "input", value: "", hint: "name" },
      { id: "b", node: "button", label: "go" },
      { id: "p", node: "progress", done: null },
      { id: "co", node: "code", text: "x = 1", language: "python" },
      { id: "d", node: "diff", text: "-a\n+b\n" },
      { id: "m", node: "markdown", text: "# hi" },
    ],
  };
  const mount: Mount = { viewer: "v1", life: "life.jsonl", slot: { slot: "dock", on: "chain1" }, skill: "k" };
  expect(PROTOCOL).toBe(7);
  expect(JSON.parse(JSON.stringify({ mount, tree }))).toEqual({ mount, tree });
});
