import { afterAll, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { removeDemoDirectories } from "@furb/tui/src/demo.ts";
import type { Restore } from "../src/wire.ts";
import { until, withLife } from "./hosting.ts";

afterAll(removeDemoDirectories);

test("a pause and a wake hold the act that the operator picked, and a cancel that ends running work waits for him to confirm it", () =>
  withLife(async (host, window) => {
    const root = window.held().life?.root;
    await host.call("send", [root, "Hire two workers"]);
    const message = () =>
      [...window.held().acts.values()].find((act) => act.kind === "prompt" && act.by === "operator");
    const workers = () =>
      [...window.held().acts.values()].filter(
        (act) => act.kind === "prompt" && act.words[1] === "Think until I cancel.",
      );
    await until("the two workers", () => workers().length === 2);
    const id = message()?.id ?? "";
    await host.call("pause", [id]);
    await until("the pause", () => message()?.paused === true);
    // The pause holds the act and what stands under it, and not the chain or the other acts of the life.
    expect(window.held().acts.get(root ?? "")?.paused).toBe(false);
    await host.call("wake", [id]);
    await until("the wake", () => message()?.paused === false);
    await host.call("cancel", [id]);
    await until("the question to confirm", () => window.said.some((one) => "confirm" in one));
    const asked = window.said.find((one) => "confirm" in one);
    expect(asked && "confirm" in asked ? asked.confirm.act : "").toBe(id);
    expect(asked && "confirm" in asked ? asked.confirm.acts.length : 0).toBeGreaterThan(1);
    expect(message()?.done).toBe(false);
    await host.call("confirm", [true]);
    await until("the cancel", () => message()?.done === true && workers().every((act) => act.done));
  }));

test("a button of a choice answers the question that it names, as a line of the composer does", () =>
  withLife(async (host, window) => {
    const root = window.held().life?.root ?? "";
    // The model of the demo asks only on a later turn, so a first message goes before.
    await host.call("send", [root, "Say what this project does."]);
    await until("the first answer", () =>
      [...window.held().acts.values()].some(
        (act) => act.kind === "prompt" && act.by === "operator" && act.done,
      ),
    );
    await host.call("send", [root, "Ask me first: which effort?"]);
    await until("the question", () => (window.held().life?.asks.length ?? 0) > 0);
    const ask = window.held().life?.asks[0];
    expect(ask?.message).toBe("Which effort should the next step take?");
    await host.call("answer", [ask?.id, "high"]);
    await until("the answer", () => window.held().acts.get(ask?.id ?? "")?.done === true);
    expect(window.held().acts.get(ask?.id ?? "")?.value).toBe("high");
    await expect(host.call("answer", [ask?.id, "low"])).rejects.toThrow("waits for no answer");
  }));

test("a message taken back while it runs ends, and comes back to the composer of its chain", () =>
  withLife(async (host, window) => {
    const root = window.held().life?.root ?? "";
    await host.call("send", [root, "Think until I cancel."]);
    const message = () =>
      [...window.held().acts.values()].find((act) => act.kind === "prompt" && act.by === "operator");
    await until("the message", () => Boolean(message()));
    expect(await host.call("takeBack", [root])).toBe(root);
    await until("the message back", () => window.said.some((one) => "compose" in one));
    expect(window.said.find((one) => "compose" in one)).toEqual({
      compose: { chain: root, text: "Think until I cancel." },
    });
    await until("the end of the message", () => message()?.done === true);
  }));

test("a rewind gives back the files and the conversation on a new branch, as the operator chose", () =>
  withLife(async (host, window, directory) => {
    const root = window.held().life?.root ?? "";
    const readme = join(directory, "README.md");
    const before = await readFile(readme, "utf8");
    const done = (count: number) => () =>
      [...window.held().acts.values()].filter(
        (act) => act.kind === "prompt" && act.by === "operator" && act.done,
      ).length === count;
    await host.call("send", [root, "Read this project and say what it does."]);
    await until("the first answer", done(1));
    await host.call("send", [root, "Add the search shortcut, Ctrl+K."]);
    await until("the second answer", done(2));
    expect(await readFile(readme, "utf8")).not.toBe(before);
    const second = [...window.held().acts.values()].findLast(
      (act) => act.kind === "prompt" && act.by === "operator",
    );
    const files = (await host.call("files", [second?.id])) as Restore;
    expect(files.files).toEqual([{ path: readme, made: false }]);
    const branch = await host.call("rewind", [second?.id, "both"]);
    expect(branch).not.toBe(root);
    expect(await readFile(readme, "utf8")).toBe(before);
    await until("the branch", () => [...window.held().acts.values()].some((act) => act.id === branch));
  }));
