import {
  CodeRenderable,
  DiffRenderable,
  InputRenderable,
  InputRenderableEvents,
  type Renderable,
  type RGBA,
} from "@opentui/core";
import type { Gesture, Node, Run, Tone, Tree } from "furb/protocol";
import { theme as c, markOf, spacing as space } from "../theme.ts";
import { bold, type Part, styled } from "../ui.ts";
import { part } from "./part.ts";
import { inputColors } from "./shared.ts";

/** The cells of a bar of progress. */
const bar = 20;

/** The color of a tone in the theme on screen. */
export function toneColor(tone: Tone | undefined): RGBA {
  return c[tone ?? "text"];
}

/** Whether a node or a node under it is a progress with no share done, which a spinner draws. */
export function spins(node: Node): boolean {
  switch (node.node) {
    case "progress":
      return node.done === undefined || node.done === null;
    case "box":
      return node.children.some(spins);
    case "list":
      return node.items.some(spins);
    case "table":
      return node.rows.some((row) => row.some(spins));
    default:
      return false;
  }
}

export const view = part({
  /** Takes the tree of a view in place of each tree of a lower version of its viewer, and draws the slots again. A
   * tree of a version that the window holds already, or of a lower one, changes nothing, since frames can cross. */
  showTree(tree: Tree): void {
    const held = this.views.get(tree.viewer);
    if (held && held.version >= tree.version) return;
    this.views.set(tree.viewer, tree);
    this.renderDock();
  },

  /** Ends a view in this window, and draws the slots again. */
  disposeView(viewer: string): void {
    if (this.views.delete(viewer)) this.renderDock();
  },

  /** The trees of the views of the dock of the chain on screen, in the order that they came. */
  dockTrees(): Tree[] {
    const on = this.session.selected;
    return [...this.views.values()].filter(({ slot }) => slot.slot === "dock" && slot.on === on);
  },

  /** Says what a person did to a node of a view to the process that serves it. A window that reads only says
   * nothing, since it draws each acting node inert. */
  viewGesture(viewer: string, node: string, kind: Gesture, value?: string): void {
    if (this.session.host.role === "read") return;
    this.viewEvents({ viewer, node, kind, ...(value === undefined ? {} : { value }) });
  },

  /** The node of OpenTUI that draws a node of a view. An acting node, a button or an input, is inert in a window
   * that reads only: it shows, faint, and takes no key and no click. */
  viewNode(viewer: string, node: Node): Renderable {
    const inert = this.session.host.role === "read";
    const id = `view:${viewer}:${node.id}`;
    switch (node.node) {
      case "box": {
        const box = this.box({
          id,
          flexDirection: node.axis,
          gap: node.gap,
          border: node.border,
          borderColor: c.faint,
          paddingX: node.border ? space.inset : 0,
          ...(node.axis === "row" ? { flexWrap: "wrap" } : {}),
        });
        for (const child of node.children) box.add(this.viewNode(viewer, child));
        return box;
      }
      case "text":
        return this.text(this.viewRuns(node.runs), c.text, { id });
      case "list": {
        const list = this.box({ id });
        for (const item of node.items) {
          const row = this.box({ flexDirection: "row" });
          row.add(this.text("• ", c.faint));
          const holder = this.box({ flexGrow: 1, flexShrink: 1 });
          holder.add(this.viewNode(viewer, item));
          row.add(holder);
          list.add(row);
        }
        return list;
      }
      case "table": {
        const table = this.box({ id });
        const cell = () => this.box({ flexGrow: 1, flexBasis: 0, flexShrink: 1, paddingRight: space.inset });
        const head = this.box({ flexDirection: "row" });
        for (const name of node.header) {
          const one = cell();
          one.add(this.line([[name, c.muted, bold]], c.muted, { truncate: true }));
          head.add(one);
        }
        table.add(head);
        for (const cells of node.rows) {
          const row = this.box({ flexDirection: "row" });
          for (const child of cells) {
            const one = cell();
            one.add(this.viewNode(viewer, child));
            row.add(one);
          }
          table.add(row);
        }
        return table;
      }
      case "input": {
        if (inert)
          return this.line(node.value || node.hint || "", node.value ? c.muted : c.faint, {
            id,
            truncate: true,
          });
        const holder = this.box({ id: `${id}:field`, height: space.bar, flexGrow: 1 });
        const input = new InputRenderable(this.renderer, {
          id,
          width: "100%",
          value: node.value,
          placeholder: node.hint ?? "",
          ...inputColors(c.raised),
          onMouseDown: () => input.focus(),
        });
        input.on(InputRenderableEvents.INPUT, (value: string) =>
          this.viewGesture(viewer, node.id, "change", value),
        );
        input.on(InputRenderableEvents.ENTER, () => {
          this.viewGesture(viewer, node.id, "submit", input.value);
          this.composer.focus();
        });
        holder.add(input);
        return holder;
      }
      case "button":
        return inert
          ? this.line(`[ ${node.label} ]`, c.faint, { id })
          : this.hoverable(
              this.line([[`[ ${node.label} ]`, c.accent, bold]], c.accent, {
                id,
                run: () => this.viewGesture(viewer, node.id, "press"),
              }),
            );
      case "progress": {
        if (node.done === undefined || node.done === null) {
          // Work of no known size turns the loader of work that runs, from the one table of marks.
          const runs = () => markOf("runs")[0];
          const loader = this.line(runs(), markOf("runs")[1], { id });
          this.moving.set(loader, runs);
          return loader;
        }
        const share = Math.min(1, Math.max(0, node.done));
        const full = Math.round(share * bar);
        return this.line(
          [
            ["━".repeat(full), c.accent],
            ["━".repeat(bar - full), c.faint],
            [` ${Math.round(share * 100)}%`, c.muted],
          ],
          c.text,
          { id },
        );
      }
      case "code": {
        const block = this.box({ id: `${id}:block`, backgroundColor: c.raised, paddingX: space.inset });
        block.add(
          new CodeRenderable(this.renderer, {
            id,
            content: node.text,
            ...(node.language ? { filetype: node.language } : {}),
            syntaxStyle: this.style,
            wrapMode: "none",
            drawUnstyledText: true,
          }),
        );
        return block;
      }
      case "diff":
        return new DiffRenderable(this.renderer, {
          id,
          diff: node.text,
          view: "unified",
          syntaxStyle: this.style,
          fg: c.text,
          showLineNumbers: true,
          lineNumberFg: c.faint,
          lineNumberBg: c.background,
          contextBg: c.background,
          addedBg: c.added,
          removedBg: c.removed,
          addedSignColor: c.success,
          removedSignColor: c.danger,
          wrapMode: "word",
        });
      case "markdown": {
        const markdown = this.markdown(node.text);
        markdown.id = id;
        return markdown;
      }
    }
  },

  /** The runs of a text, each in the color of its tone. */
  viewRuns(runs: readonly Run[]): Part[] {
    return runs.map(({ text, tone }) => [text, toneColor(tone)] as const);
  },

  /** Draws each text whose mark moves again at the frame of the clock of the TUI, and nothing else. */
  tickMoving(): void {
    for (const [text, read] of this.moving) {
      if (text.isDestroyed) {
        this.moving.delete(text);
        continue;
      }
      const now = read();
      text.content = typeof now === "string" ? now : styled(now);
    }
  },
});

export type ViewPart = typeof view;
