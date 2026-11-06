import { expect, test } from "bun:test";
import type { BoxRenderable, Renderable } from "@opentui/core";
import { foldOf, hidden } from "../src/app/shared.ts";
import { composing } from "./composing.ts";

/** The plain text of a node and of each node in it, row by row. */
const plainOf = (node: Renderable): string =>
  "plainText" in node && typeof node.plainText === "string"
    ? node.plainText
    : node
        .getChildren()
        .map((one) => plainOf(one))
        .join("");

/** The lines of a text, numbered from one. */
const numbered = (count: number) => Array.from({ length: count }, (_, at) => `line ${at + 1}`).join("\n");

test("every fold says the count of what it hides in one wording, with the plural of its noun", () => {
  expect([
    hidden(1, "line"),
    hidden(2, "line"),
    hidden(1, "step", "earlier"),
    hidden(6, "message", "earlier"),
  ]).toEqual(["1 more line", "2 more lines", "1 earlier step", "6 earlier messages"]);
  // A fold of a line or two hides nothing, so such a text shows whole.
  expect([foldOf(12), foldOf(14), foldOf(15), foldOf(40)]).toEqual([0, 0, 3, 28]);
});

test("the tail of a command shows whole up to the bound, then one fold of its earlier lines that a click opens in place", () =>
  composing(async ({ app, session }) => {
    const short: BoxRenderable = app.box();
    app.excerpt(short, numbered(13), "tail:short");
    expect(short.getChildren()).toHaveLength(1);
    expect(plainOf(short.getChildren()[0] as Renderable)).toEndWith(numbered(13));
    const long: BoxRenderable = app.box();
    app.excerpt(long, numbered(20), "tail:long");
    const [fold, rest] = long.getChildren();
    expect(plainOf(fold as Renderable)).toBe("▸ 8 earlier lines");
    expect(plainOf(rest as Renderable)).toEndWith(numbered(20).split("\n").slice(8).join("\n"));
    app.openFold("tail:long");
    expect(session.folds["tail:long"]).toBe(false);
    const open: BoxRenderable = app.box();
    app.excerpt(open, numbered(20), "tail:long");
    expect(open.getChildren()).toHaveLength(1);
    expect(plainOf(open.getChildren()[0] as Renderable)).toEndWith(numbered(20));
  }));

test("a reply shows whole up to the bound, then its first rows and one fold of the rest that a click opens in place", () =>
  composing(async ({ app }) => {
    const short: BoxRenderable = app.box();
    app.bounded(short, numbered(14), "reply:short");
    expect(short.getChildren()).toHaveLength(1);
    const long: BoxRenderable = app.box();
    app.bounded(long, numbered(30), "reply:long");
    expect(long.getChildren().map(plainOf).at(-1)).toBe("▸ 18 more lines");
    app.openFold("reply:long");
    const open: BoxRenderable = app.box();
    app.bounded(open, numbered(30), "reply:long");
    expect(open.getChildren()).toHaveLength(1);
  }));

test("each fold of the feed opens from the Details of ⌥D, as a click on its row does", () =>
  composing(async ({ app, session, screen, frame }) => {
    app.excerpt(app.box(), numbered(40), "tail:keys");
    app.details();
    await screen.mockInput.typeText("28 earlier lines");
    screen.mockInput.pressEnter();
    for (let step = 0; step < 40 && session.folds["tail:keys"] !== false; step++) await frame();
    expect(session.folds["tail:keys"]).toBe(false);
  }));
