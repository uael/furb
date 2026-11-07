// The tail of a thread row never joins the state word and the age: one cell parts them, for each state.
import { expect, test } from "bun:test";
import { clock } from "../src/clock.ts";
import { plain } from "../src/ui.ts";
import { composing } from "./composing.ts";

test("the state and the age of a thread are parted by one cell, for a cancelled, a paused and a running thread", () =>
  composing(async ({ app, session }) => {
    const act = { id: "prompt1" };
    session.changedAt.prompt1 = clock() - 5 * 60000;
    const thread = (state: string) => ({ act, messages: [], state, over: state !== "runs" }) as never;
    expect(plain(app.threadMeta(thread("cancelled"), "cancelled"))).toBe("cancelled 5m");
    expect(plain(app.threadMeta(thread("paused"), "paused"))).toBe("paused 5m");
    expect(plain(app.threadMeta(thread("runs"), "runs"))).toBe("5m");
  }));
