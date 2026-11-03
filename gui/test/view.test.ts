import { expect, test } from "bun:test";
import type { Node } from "furb/protocol";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Button, blocks, changed, follows, inlines, View, ViewSlot } from "../src/components/View.tsx";

const draw = (tree: Node, inert = false) => renderToStaticMarkup(createElement(View, { tree, inert }));

const text = (id: string, ...runs: [string, string?][]): Node => ({
  id,
  node: "text",
  runs: runs.map(([run, tone]) => (tone ? { text: run, tone: tone as never } : { text: run })),
});

test("a box lays out its children on its axis with its gap, and a border when it has one", () => {
  const tree: Node = {
    id: "b",
    node: "box",
    axis: "row",
    gap: 2,
    border: true,
    children: [text("t1", ["one"]), text("t2", ["two"])],
  };
  const markup = draw(tree);
  expect(markup).toContain("flex-direction:row;gap:0.5rem");
  expect(markup).toContain("border border-hair");
  expect(markup.indexOf("one")).toBeLessThan(markup.indexOf("two"));
});

test("a text draws each run in the color of its tone, and a run with no tone reads as text", () => {
  const markup = draw(text("t", ["ok", "success"], ["bad", "danger"], ["plain"]));
  expect(markup).toContain('class="text-done">ok');
  expect(markup).toContain('class="text-failed">bad');
  expect(markup).toContain('class="text-text">plain');
});

test("a list draws one item for each child, and a table draws its header and its cells", () => {
  const list: Node = { id: "l", node: "list", items: [text("a", ["x"]), text("b", ["y"])] };
  expect(draw(list).match(/<li>/g)).toHaveLength(2);
  const table: Node = {
    id: "tb",
    node: "table",
    header: ["Name", "State"],
    rows: [[text("c1", ["one"]), text("c2", ["done"])]],
  };
  const markup = draw(table);
  expect(markup).toContain('scope="col"');
  expect(markup).toContain(">Name<");
  expect(markup).toContain("one");
});

test("an input draws its value and its hint, and a progress draws its share or waits with none", () => {
  const input: Node = { id: "i", node: "input", value: "abc", hint: "type" };
  const markup = draw(input);
  expect(markup).toContain('value="abc"');
  expect(markup).toContain('placeholder="type"');
  expect(draw({ id: "p", node: "progress", done: 0.5 })).toContain('value="0.5"');
  expect(draw({ id: "p", node: "progress", done: 2 })).toContain('value="1"');
  expect(draw({ id: "p", node: "progress", done: null })).not.toContain("value=");
});

test("code keeps its language and its text, and a diff marks each line as it changes", () => {
  const code = draw({ id: "c", node: "code", text: "x = 1", language: "python" });
  expect(code).toContain('data-language="python"');
  expect(code).toContain("x = 1");
  const diff = draw({ id: "d", node: "diff", text: "--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n same" });
  expect(diff).toContain('data-change="del"');
  expect(diff).toContain('data-change="add"');
  expect(diff).toContain('data-change="hunk"');
  expect(changed("+++ b")).toBe("head");
  expect(changed(" same")).toBe("same");
});

test("a markdown draws its headings, its code, its lists and its paragraphs as elements", () => {
  expect(blocks("# Title\n\nSome **bold** text\nin two lines.\n\n- one\n- two\n\n```py\nx = 1\n```")).toEqual(
    [
      { kind: "heading", level: 1, text: "Title" },
      { kind: "paragraph", text: "Some **bold** text in two lines." },
      { kind: "list", items: ["one", "two"] },
      { kind: "code", language: "py", text: "x = 1" },
    ],
  );
  const markup = draw({ id: "m", node: "markdown", text: "# T\n\nUse `a` and **b**." });
  expect(markup).toContain("<h1");
  expect(markup).toContain("<code");
  expect(markup).toContain("<strong>b</strong>");
});

test("a markdown makes a link only of an address of the web, and never writes HTML of its text", () => {
  expect(inlines("[a](https://x.dev) [b](javascript:alert(1))")).toEqual([
    { kind: "link", text: "a", href: "https://x.dev" },
    { kind: "text", text: " [b](javascript:alert(1))" },
  ]);
  const markup = draw({ id: "m", node: "markdown", text: "<script>alert(1)</script>" });
  expect(markup).not.toContain("<script>");
  expect(markup).toContain("&lt;script&gt;");
});

test("a button sends its press to the skill, and an inert view draws it disabled and sends nothing", () => {
  const told: unknown[][] = [];
  const press = (inert: boolean) =>
    Button({ id: "go", label: "Go", inert, onEvent: (...one) => told.push(one) }) as unknown as {
      props: { onClick: () => void; disabled: boolean };
    };
  press(false).props.onClick();
  expect(told).toEqual([["go", "press"]]);
  const inert = press(true);
  expect(inert.props.disabled).toBe(true);
  inert.props.onClick();
  expect(told).toHaveLength(1);
  expect(draw({ id: "go", node: "button", label: "Go" }, true)).toContain("disabled");
  expect(draw({ id: "i", node: "input", value: "v" }, true)).toContain("readOnly");
});

test("a slot names where its view stands and what the view follows", () => {
  expect(follows({ slot: "dock", on: "chain2" })).toEqual({ slot: "dock", chain: "chain2" });
  expect(follows({ slot: "mark", act: "bash3" })).toEqual({ slot: "mark", act: "bash3" });
  expect(follows({ slot: "footer" })).toEqual({ slot: "footer" });
  const tree = text("t", ["hi"]);
  const markup = renderToStaticMarkup(
    createElement(ViewSlot, { slot: { slot: "row", act: "bash3" }, tree, label: "Row" }),
  );
  expect(markup).toContain('data-slot="row"');
  expect(markup).toContain('data-act="bash3"');
  expect(markup).toContain('aria-label="Row"');
});
