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

/** A line of code of a rung, as a program gives it. */
const codeLine = (text: string) =>
  ({
    kind: "code",
    step: { id: "rung1", kind: "rung" },
    number: 1,
    display: 1,
    text,
    acts: [],
    hint: "",
  }) as never;

test("a long string in a folded word has the one fold row of the feed under its line, which a click and the Details open", () =>
  composing(async ({ app, session }) => {
    const long = `${"word ".repeat(48)}ENDMARK`;
    const view = app.programView(
      [codeLine(`brief = "${long}"`)],
      100,
      () => "",
      () => "",
      new Set(),
      1,
    );
    const text = String(Reflect.get(Reflect.get(view, "target") as object, "content") ?? "");
    const rows = text.split("\n");
    // What the cut line hides stands at its end, in its trailer, and in no row of its own under it.
    expect(rows[0]).toMatch(/…"\s+…\+\d+$/);
    expect(rows.some((one) => /more characters/.test(one))).toBe(false);
    const row = app.foldRows.get("string:rung1:1:0");
    expect(row?.label).toMatch(/^\d+ more characters$/);
    expect(rows[0]).toContain(`…+${row?.label.split(" ")[0]}`);
    // The Details entry opens it in place, as a click on the row does.
    app.openFold("string:rung1:1:0");
    expect(session.folds["string:rung1:1:0"]).toBe(false);
  }));

/** The lines that the details of a word draw for a line of code of 100 columns or less, and the labels of its folds. */
function drawn(app: Parameters<Parameters<typeof composing>[0]>[0]["app"], text: string, width: number) {
  const view = app.programView(
    [codeLine(text)],
    width,
    () => "",
    () => "",
    new Set(),
    1,
  );
  const rows = String(Reflect.get(Reflect.get(view, "target") as object, "content") ?? "").split("\n");
  const labels = [...app.foldRows.entries()]
    .filter(([key]) => key.startsWith("string:rung1:1:"))
    .map(([, fold]) => Number(fold.label.split(" ")[0]));
  return { rows, labels };
}

test("a line that cuts two strings ends with one trailer that sums what both hide", () =>
  composing(async ({ app }) => {
    const text = `pair = ("${"alpha ".repeat(30)}", "${"omega ".repeat(36)}")`;
    const { rows, labels } = drawn(app, text, 100);
    expect(labels).toHaveLength(2);
    const trailers = rows.flatMap((row) => [...row.matchAll(/…\+(\d+)$/g)].map((one) => Number(one[1])));
    expect(trailers).toHaveLength(1);
    expect(trailers.reduce((sum, one) => sum + one, 0)).toBe(labels.reduce((sum, one) => sum + one, 0));
    expect(rows.some((one) => /more characters/.test(one))).toBe(false);
  }));

test("a cut line whose width has no room for its trailer keeps the fold row of its own under it", () =>
  composing(async ({ app }) => {
    const { rows, labels } = drawn(app, `brief = "${"word ".repeat(30)}"`, 32);
    expect(labels.length).toBeGreaterThan(0);
    expect(rows.some((one) => /…\+\d+/.test(one))).toBe(false);
    // The line keeps its whole width, as the room for a trailer that cannot fit is no room lost.
    expect(Bun.stringWidth(rows[0] ?? "")).toBeGreaterThan(24);
    expect(rows.filter((one) => /^▸ \d+ more characters$/.test(one))).toHaveLength(labels.length);
  }));

test("the rolling window of a running word has the one fold row of the feed, with its count and plural, which the Details open", () =>
  composing(async ({ app, session }) => {
    const lines = Array.from(
      { length: 12 },
      (_, at) =>
        ({ ...(codeLine(`x${at + 1} = ${at + 1}`) as object), number: at + 1, display: at + 1 }) as never,
    );
    const { lines: shown } = app.rolling(lines, "rung1:0", false);
    const view = app.programView(
      shown,
      100,
      () => "",
      () => "",
      new Set(),
      2,
      { key: "window:rung1:0", more: () => app.openWindow("rung1:0") },
    );
    const text = String(Reflect.get(Reflect.get(view, "target") as object, "content") ?? "");
    expect(text.split("\n")[0]).toBe("▸ 6 earlier lines");
    const [entry] = [...app.foldRows].filter(([key]) => key.startsWith("window:rung1:0"));
    expect(entry?.[1].label).toBe("6 earlier lines");
    entry?.[1].open?.();
    expect(session.folds["window:rung1:0"]).toBe(false);
  }));
