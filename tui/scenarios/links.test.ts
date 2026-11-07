// A link of Markdown in the feed opens at a click, as the fold of '… X more lines' does.
import { expect } from "bun:test";
import { scenario, test, until } from "./harness.ts";

test("a click on a link of Markdown in an answer opens it", () =>
  scenario(
    [{ cue: "Link: show.", word: 'close("See [the docs](https://example.com/docs) for more.")' }],
    async (window) => {
      const opened: string[] = [];
      window.app.openLink = async (url: string) => {
        opened.push(url);
      };
      await window.type("Link: show.");
      await window.shows((frame) => frame.includes("the docs (https"), "the answer");
      // The word of the model shows its own text first, so the click goes to the words of the answer.
      await window.click("the docs (https");
      await until(() => opened.length > 0, "the link opened", 10);
      expect(opened).toEqual(["https://example.com/docs"]);
    },
  ));
