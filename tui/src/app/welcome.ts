import type { BoxRenderable, Renderable, RGBA, TextRenderable } from "@opentui/core";
import { imageReferences } from "furb";
import { demoSession, tour } from "../demo.ts";
import { shortenHome } from "../files.ts";
import { clip, dollars, modelName } from "../format.ts";
import type { View } from "../session.ts";
import { answering, type Check, checks } from "../setup.ts";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { bold, logo, mix, type Part } from "../ui.ts";
import { modelsRead, watchBar } from "../watching.ts";
import { part } from "./part.ts";
import { noBorder, starters, viewLabels, watching } from "./shared.ts";

export const welcome = part({
  /** A panel of the feed that says a state of the view: its heading, what the state holds, and a link that acts on it
   * with what the link does. */
  banner(
    box: BoxRenderable,
    color: RGBA,
    heading: Part[],
    detail: Renderable | undefined,
    ...actions: [label: string, does: string, run: () => void][]
  ): void {
    const panel = this.panel(box, color);
    panel.add(this.text(heading));
    if (detail) panel.add(this.inset(space.between, detail));
    for (const [index, [label, does, run]] of actions.entries()) {
      const row = this.box({
        flexDirection: "row",
        marginTop: index ? 0 : space.section,
        paddingLeft: space.between,
      });
      row.add(this.link([[label, c.accent, bold]], run));
      row.add(this.text(`  ${does}`, c.faint));
      panel.add(row);
    }
  },

  /** A text that lights under the pointer, and runs an action at a click. */
  link(content: string | Part[], run: () => void): TextRenderable {
    return this.hoverable(this.text(content, c.accent, { run }));
  },

  /** A message of the operator: its text, and the name of each image it refers to in the color of an attachment. A
   * click on the message opens the actions of its first image. */
  message(text: string): TextRenderable {
    const images = imageReferences(text);
    const parts: Part[] = [];
    let rest = text;
    for (const image of images) {
      const at = rest.indexOf(image.text);
      parts.push([rest.slice(0, at), c.text], [`${glyph.dot} ${image.name || "image"}`, c.secondary]);
      rest = rest.slice(at + image.text.length);
    }
    parts.push([rest, c.text]);
    const first = images[0];
    return this.text(parts, c.text, { run: first && (() => this.imageActions(first.uri)) });
  },

  /** What a view says in the middle of the feed when it has nothing else to show. */
  centered(box: BoxRenderable, height: number, ...nodes: Renderable[]): void {
    const frame = this.box({
      minHeight: Math.max(0, height - space.section * 2),
      justifyContent: "center",
      alignItems: "center",
    });
    for (const node of nodes) frame.add(node);
    box.add(frame);
  },

  /** The screen of a feed that has no turn yet: the logo of furb, what it is, where it works, where to start, and the
   * keys to know. */
  welcome(box: BoxRenderable, height: number): void {
    const w = this.session;
    const width = Math.min(72, this.feedWidth);
    const column = this.box({ width, alignItems: "center" });
    // The logo shades from the accent to the color of Python, one column at a time. The mode of a screen reader says
    // the name, since a reader reads the blocks of the logo as glyphs.
    const columns = Math.max(...logo.map((line) => line.length));
    if (this.reader) column.add(this.text([["furb", c.accent, bold]]));
    else
      for (const line of logo)
        column.add(
          this.text(
            [...line].map(
              (cell, at): Part => [cell, mix(c.accent, c.secondary, at / Math.max(1, columns - 1))],
            ),
            c.accent,
            { width: columns },
          ),
        );
    column.add(
      this.text("The model answers in Python. Read and steer each step it runs.", c.muted, {
        marginTop: space.section,
      }),
    );
    // The check of the setup, once the worker said what this machine holds.
    const { found, none, shown } = this.setupState();
    // With no model that answers, the actor is the operator, which the line does not name.
    const model = none ? "" : modelName(w.actorChoice.model).id.replace(/^operator$/, "");
    column.add(
      this.whole(
        this.text(
          [
            [clip(shortenHome(w.workingDirectory), 40, "end"), c.faint],
            [model ? "   " : "", c.faint],
            [model, c.faint],
          ],
          c.faint,
          { truncate: true },
        ),
        () => `${shortenHome(w.workingDirectory)}${model ? `   ${model}` : ""}`,
      ),
    );
    // With no model, a prompt has nothing to answer it, so the welcome offers only the life to watch.
    if (shown) {
      column.add(this.setupCheck(found, width));
      if (!w.preferences.setupSeen) {
        w.preferences.setupSeen = true;
        try {
          w.preferences.save();
        } catch (error) {
          this.report(error);
        }
      }
    }
    // Each way to start is a card of two lines. The first plays a life to watch, and a click on each other one puts
    // its prompt in the composer. The cards are as wide as their longest line, and stand in the middle as one block.
    const cards: [label: string, line: string, run: () => void][] = [
      ...(w.watching ? [] : [[...watching, () => void this.watch()] as [string, string, () => void]]),
      ...(none ? [] : starters).map(([label, prompt]): [string, string, () => void] => [
        label,
        prompt,
        () => this.insert(prompt),
      ]),
    ];
    const widest = Math.max(
      ...cards.flatMap(([label, line]) => [label, line].map((one) => Bun.stringWidth(one))),
    );
    const starts = this.box({
      width: Math.min(width, widest + space.between * 2 + 1),
      marginTop: space.section * 2,
      gap: space.section,
    });
    // A card has a bar at its left, but in the mode of a screen reader.
    const framed = !this.reader;
    for (const [index, [label, prompt, run]] of cards.entries()) {
      const lead = index === 0 && !w.watching;
      // The pointer lifts a card onto a panel, and its bar takes the accent.
      const card = this.box({
        paddingX: space.between,
        backgroundColor: c.background,
        border: ["left"],
        borderColor: c.border,
        customBorderChars: { ...noBorder, vertical: glyph.bar },
        onMouseUp: this.click(run),
        onMouseOver() {
          this.backgroundColor = c.panel;
          if (framed) this.borderColor = c.accent;
        },
        onMouseOut() {
          this.backgroundColor = c.background;
          if (framed) this.borderColor = lead ? c.accent : c.border;
        },
      });
      // The card that plays a life leads, with its bar in the accent.
      if (lead && framed) card.borderColor = c.accent;
      card.add(this.text([[label, lead ? c.accent : c.text, bold]], c.text, { truncate: true }));
      card.add(this.text(prompt, c.muted, { truncate: true }));
      starts.add(card);
    }
    column.add(starts);
    // Each key under the cards is a button that does what it names.
    const keys = this.box({ flexDirection: "row", marginTop: space.section * 2, gap: space.between + 1 });
    for (const [chord, action, run] of [
      ["/", "commands", () => this.palette()],
      ["@", "files", () => void this.filesPicker().catch(this.report)],
      ["⌃R", "Python", () => this.toggleMode()],
      ["F1", "help", () => this.help()],
    ] as const)
      keys.add(
        this.text(
          [
            [chord, c.muted],
            [` ${action}`, c.faint],
          ],
          c.faint,
          { run },
        ),
      );
    column.add(keys);
    this.centered(box, height, column);
  },

  /** The check of the setup, once the worker said what this machine holds: its checks, whether no model answers, and
   * whether the welcome shows it, which it does on the first run, and on each run while no model answers. The demo
   * needs no model and checks nothing. */
  setupState(): { found: Check[]; none: boolean; shown: boolean } {
    const w = this.session;
    const found = w.setup && w.catalogRead ? checks(w.setup, answering(w.setup, w.catalog)) : [];
    const none = found.at(-1)?.state === "missing";
    return { found, none, shown: Boolean(found.length) && (this.firstRun || none) };
  },

  /** What the welcome depends on, which says when its card is made again. */
  welcomeKey(): string {
    const w = this.session;
    return JSON.stringify([
      this.reader,
      w.catalogRead,
      w.catalog.length,
      w.setup,
      w.actorChoice.model,
      w.workingDirectory,
      w.watching,
    ]);
  },

  /** The check of the setup again, after the operator fixed a part of it. */
  checkAgain(): void {
    this.session.notice = "Checking the setup again.";
    void this.session
      .checkSetup()
      .then(() => {
        this.session.notice = this.setupState().none ? "Still no model answers." : "A model answers now.";
        this.renderContent();
      })
      .catch(this.report);
  },

  /** The name of a view of a chain: the watch names the transcript by what it is. */
  viewLabel(view: View): string {
    return view === "transcript" && this.session.watching ? modelsRead : viewLabels[view];
  },

  /** The bar of the watch, pinned over the feed: drawn again only when what it says changed. */
  renderWatch(): void {
    const w = this.session;
    const visible = w.watching && !this.tree;
    if (this.watchBar.visible !== visible) this.watchBar.visible = visible;
    const key = visible ? `${w.moment === undefined}:${w.cost}:${this.theme}` : "";
    if (key === this.watchKey) return;
    this.watchKey = key;
    this.clear(this.watchBar);
    if (visible) this.watchBanner(this.watchBar);
  },

  /** The check of the setup: each way to a model that this machine holds or lacks, with a fix for each one that is
   * missing, and a clear word when no model answers. */
  setupCheck(found: Check[], width: number): BoxRenderable {
    const none = found.at(-1)?.state === "missing";
    const panel = this.box({
      width,
      marginTop: space.section * 2,
      paddingX: space.between,
      gap: space.section,
    });
    panel.add(
      this.text(
        none
          ? [
              [`${glyph.failed} `, c.danger],
              ["No model answers yet", c.text, bold],
            ]
          : [
              [`${glyph.done} `, c.success],
              ["Setup", c.text, bold],
            ],
        c.text,
      ),
    );
    const marks: Record<Check["state"], Part> = {
      ready: [glyph.done, c.success],
      missing: [glyph.failed, c.danger],
      optional: [glyph.ring, c.faint],
    };
    // A check and its fix stand together, and a line of space stands between two checks.
    for (const check of found) {
      const [mark, color] = marks[check.state];
      const one = this.box({});
      one.add(
        this.text(
          [
            [`${mark} `, color],
            [check.label, c.text],
            [`  ${check.found}`, c.muted],
          ],
          c.text,
        ),
      );
      if (check.fix && check.state !== "ready") one.add(this.text(check.fix, c.faint, { paddingLeft: 2 }));
      panel.add(one);
    }
    return panel;
  },

  /** The bar at the top of the feed of a life that the operator watches: what it costs, which is nothing, what it
   * shows, in common words, and where the moment stands when the operator moved back in time. */
  watchBanner(box: BoxRenderable): void {
    const w = this.session;
    const panel = this.panel(box, c.accent);
    // The demo asks no model: its script answers each turn, so no call goes out and nothing is spent.
    const earlier = w.moment !== undefined;
    const [title, count, shows, keys] = watchBar(dollars(w.cost), earlier);
    panel.add(
      this.text([
        [`${glyph.running} `, c.accent],
        [title, c.text, bold],
        [`   ${count}`, c.muted],
      ]),
    );
    panel.add(this.text(shows, c.muted, { paddingLeft: space.between }));
    panel.add(this.text(keys, earlier ? c.warning : c.faint, { paddingLeft: space.between }));
  },

  /** Watch a life: a new demo that plays by itself, asks no model, and shows in a workspace of its own. */
  async watch(): Promise<void> {
    try {
      const session = await demoSession({ watch: true, preferences: this.session.preferences });
      await this.options.workspaces.show(session);
      void tour(session).catch(session.fail);
    } catch (error) {
      this.report(error);
    }
  },
});

export type WelcomePart = typeof welcome;

export const welcomeBound = ["watch", "checkAgain"] as const;
