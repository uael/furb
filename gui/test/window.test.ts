import { expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Composer } from "../src/components/Composer.tsx";
import { Picker } from "../src/components/Picker.tsx";
import { empty, type Held } from "../src/held.ts";
import type { Life } from "../src/wire.ts";

/** A life that the host sends, with what a test changes in it. */
function lifeOf(change: Partial<Life> = {}): Life {
  return {
    record: "/work/.furb/sessions/one.jsonl",
    root: "chain1",
    directory: "/work",
    place: "work",
    name: "Session 1",
    selected: "chain1",
    actor: "claude-cli:sonnet/high",
    status: "idle",
    loading: false,
    notice: "",
    error: "",
    recorded: false,
    pending: 0,
    cost: 0,
    measures: {},
    asks: [],
    ...change,
  };
}

/** The window as it holds a life, with what a test changes in it. */
const heldOf = (life: Life | null, change: Partial<Held> = {}): Held => ({
  ...empty,
  connected: true,
  life,
  ...change,
});

/** The markup of the composer under a chain. */
const composer = (held: Held, chain = "chain1", mine = false) =>
  renderToStaticMarkup(
    createElement(Composer, {
      held,
      chain,
      label: "Main",
      mine,
      onSent: () => undefined,
      onChain: () => undefined,
    }),
  );

test("the composer sends a message to the chain shown", () => {
  const markup = composer(heldOf(lifeOf()));
  expect(markup).toContain('placeholder="Message Main"');
  expect(markup).toContain('aria-label="A message to Main"');
});

test("the composer answers the question that waits on the chain shown, and not one of another chain", () => {
  const ask = {
    id: "prompt3",
    chain: "chain1",
    label: "Main",
    by: "rung2",
    shape: "str",
    message: "Which effort should the next step take?",
    notice: false,
    choices: [],
    began: 9,
  };
  const held = heldOf(lifeOf({ asks: [ask] }));
  expect(composer(held)).toContain('placeholder="Answer: Which effort should the next step take?"');
  expect(composer(held, "chain2")).toContain('placeholder="Message Main"');
});

test("a question of a choice gives a button for each of its values, and a notice gives none", () => {
  const ask = {
    id: "prompt4",
    chain: "chain1",
    label: "Main",
    by: "rung3",
    shape: "Literal['low', 'high']",
    message: "Which effort?",
    notice: false,
    choices: ["low", "high"],
    began: 9,
  };
  const markup = composer(heldOf(lifeOf({ asks: [ask] })));
  expect(markup).toContain(">low</button>");
  expect(markup).toContain(">high</button>");
  expect(composer(heldOf(lifeOf()))).not.toContain(">low</button>");
});

test("the composer takes back your last message only on a chain that holds one", () => {
  expect(composer(heldOf(lifeOf()), "chain1", true)).toContain('aria-label="Take back your last message"');
  expect(composer(heldOf(lifeOf()), "chain1", false)).not.toContain("Take back");
});

test("a record that opens to read takes no message, and says why", () => {
  const reading = { why: "Drift: bash1 drifts", made: "abc", engine: "def", logged: true };
  const markup = composer(heldOf(lifeOf({ reading })));
  expect(markup).not.toContain("<textarea");
  expect(markup).toContain("This record opens to read: it was made by abc");
});

test("the picker offers a new life in each project, and opens each of its records", () => {
  const projects = [
    {
      directory: "/work",
      name: "work",
      records: [{ path: "/work/.furb/sessions/one.jsonl", name: "Session 1", status: "idle", size: 2048 }],
    },
  ];
  const markup = renderToStaticMarkup(createElement(Picker, { held: heldOf(null, { projects }) }));
  expect(markup).toContain("New life");
  expect(markup).toContain("Session 1");
  expect(markup).toContain("Ready · 2 KB");
  expect(markup).toContain(">Open<");
  expect(markup).not.toContain("Waiting for the host of the life");
  expect(renderToStaticMarkup(createElement(Picker, { held: empty }))).toContain(
    "Waiting for the host of the life",
  );
});
