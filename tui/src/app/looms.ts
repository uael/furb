import type { MouseEvent } from "@opentui/core";
import { type Loom, weave } from "furb/life";
import { type ChainEntry, type ChainsDrawn, drawChains, quietFolds, weaveWidth } from "../chains.ts";
import { theme as c } from "../theme.ts";
import { plain } from "../ui.ts";
import { part } from "./part.ts";

/** What the panel of the chains holds while it shows over the feed. */
export interface ChainsPanel {
  /** The chain that the pointer stands on, or `rest:` and the chain whose finished chains it stands on. */
  pointer: string;
  folds: Set<string>;
  rested: Set<string>;
  top: number;
  /** The entries, the chain of each line and the column of its fold arrow, as the last draw left them. */
  entries: ChainEntry[];
  targets: (string | undefined)[];
  arrows: (number | undefined)[];
  loom: Loom;
  /** Whether a chain works, which makes the panel draw again with time. */
  live: boolean;
  woven?: { key: string; loom: Loom };
}

export const looms = part({
  /** The panel of the chains over the feed: the tree of the chains drawn as a loom, which the keys and the mouse move
   * on, and which only picks a chain. A second press of the key closes it. */
  chains(): void {
    if (this.tree?.panel) {
      this.closeTree();
      return;
    }
    const pointer = this.session.selected;
    const panel: ChainsPanel = {
      pointer,
      folds: new Set(),
      rested: new Set(),
      top: 0,
      entries: [],
      targets: [],
      arrows: [],
      loom: { rows: [], owed: [], folded: 0 },
      live: false,
    };
    panel.folds = quietFolds(this.chainsLoom(panel, 12), pointer);
    this.closeOverlay();
    this.closeSearch();
    this.tree = { rows: [], selected: pointer, title: "Chains", panel };
    this.composer.blur();
    this.render();
  },

  /** The loom of the chains at a count of cells of time, woven again only when the life changed. */
  chainsLoom(panel: ChainsPanel, cells: number): Loom {
    const w = this.session;
    const key = [w.version, cells, w.host.prompts.size, w.host.pending.size].join(":");
    if (panel.woven?.key !== key)
      panel.woven = {
        key,
        loom: weave({
          acts: w.acts,
          prompts: [...w.host.prompts.values()],
          measures: w.host.measures,
          folds: new Set(),
          width: cells,
          pending: w.host.pending,
          label: (id) => id,
        }),
      };
    const loom = panel.woven.loom;
    return {
      ...loom,
      rows: loom.rows.map((row) => (row.id === "operator" ? row : { ...row, label: w.labelOf(row.id) })),
    };
  },

  chainsDraw(panel: ChainsPanel, width: number, height: number): ChainsDrawn {
    const loom = this.chainsLoom(panel, weaveWidth(width));
    panel.loom = loom;
    panel.live = loom.rows.some((row) => row.state === "working");
    const drawn = drawChains({
      loom,
      width,
      height,
      pointer: panel.pointer,
      folds: panel.folds,
      rested: panel.rested,
      top: panel.top,
      actor: (id) => this.actorOf(id),
    });
    panel.top = drawn.top;
    panel.entries = drawn.entries;
    panel.targets = drawn.targets;
    panel.arrows = drawn.arrows;
    return drawn;
  },

  renderChains(): void {
    const panel = this.tree?.panel;
    if (!panel) return;
    const width = this.feedWidth;
    const height = Math.max(10, this.scroll.height || this.renderer.height - 12);
    let drawn = this.chainsDraw(panel, width, height);
    if (!drawn.entries.some((entry) => entry.id === panel.pointer)) {
      panel.pointer = drawn.entries[0]?.id ?? panel.pointer;
      drawn = this.chainsDraw(panel, width, height);
    }
    if (this.paneChanged(this.treeBar, ["chains", this.theme, width, plain(drawn.header)])) {
      this.clear(this.treeBar);
      this.treeBar.add(this.line(drawn.header, c.muted, { flexGrow: 1, flexShrink: 1 }));
      this.treeBar.add(this.text("Esc", c.faint, { run: () => this.closeTree() }));
    }
    if (!this.paneChanged(this.scroll, [drawn.lines.map(plain).join("|"), width, this.theme])) return;
    this.clear(this.scroll);
    for (const [at, line] of drawn.lines.entries())
      this.scroll.add(
        this.line(line, c.text, {
          onMouseUp: this.click((event) => void this.clickChains(at, event.x).catch(this.report)),
          onMouseScroll: (event: MouseEvent) => {
            const direction = event.scroll?.direction;
            if (direction === "up" || direction === "down") this.moveChains(direction === "up" ? -3 : 3);
          },
        }),
      );
  },

  chainsEntry(): ChainEntry | undefined {
    const panel = this.tree?.panel;
    return panel?.entries.find((entry) => entry.id === panel.pointer);
  },

  pointChains(id: string): void {
    const panel = this.tree?.panel;
    if (!panel) return;
    panel.pointer = id;
    this.renderContent();
    this.renderStatus();
  },

  moveChains(step: number): void {
    const panel = this.tree?.panel;
    if (!panel?.entries.length) return;
    const at = panel.entries.findIndex((entry) => entry.id === panel.pointer);
    const next = panel.entries[Math.max(0, Math.min(panel.entries.length - 1, at + step))];
    if (next) this.pointChains(next.id);
  },

  /** A branch of the tree folded or shown, or for the finished chains of a chain: folded into one row or each shown. */
  toggleChains(entry: ChainEntry): void {
    const panel = this.tree?.panel;
    if (!panel) return;
    const rest = entry.id.startsWith("rest:");
    const set = rest ? panel.rested : panel.folds;
    const id = rest ? entry.id.slice(5) : entry.id;
    if (set.has(id)) set.delete(id);
    else set.add(id);
    this.renderContent();
    this.renderStatus();
  },

  /** Right opens a branch, or goes into it; Left folds a branch, or goes to the chain above. */
  foldChains(open: boolean): void {
    const panel = this.tree?.panel;
    const entry = this.chainsEntry();
    if (!panel || !entry) return;
    const rest = entry.id.startsWith("rest:");
    if (open) {
      if (entry.foldable && entry.folded) this.toggleChains(entry);
      else {
        const child = panel.entries.find((one) => one.parent === entry.id);
        if (child) this.pointChains(child.id);
      }
    } else if (entry.foldable && !entry.folded && !rest) this.toggleChains(entry);
    else if (entry.parent) this.pointChains(entry.parent);
  },

  /** Enter opens the chain under the pointer in the feed and closes the panel: the panel only picks. */
  async enterChains(): Promise<void> {
    const entry = this.chainsEntry();
    if (!entry) return;
    if (entry.id.startsWith("rest:")) {
      this.toggleChains(entry);
      return;
    }
    this.closeTree();
    if (entry.id !== this.session.selected) await this.session.select(entry.id);
  },

  async clickChains(at: number, x: number): Promise<void> {
    const panel = this.tree?.panel;
    const id = panel?.targets[at];
    if (!panel || !id) return;
    const arrow = panel.arrows[at];
    const entry = panel.entries.find((one) => one.id === id);
    const column = x - this.scroll.x;
    if (entry?.foldable && arrow !== undefined && column >= arrow && column < arrow + 2)
      this.toggleChains(entry);
    else if (panel.pointer === id) await this.enterChains();
    else this.pointChains(id);
  },

  /** The pointer goes to the next chain that waits on a question of the operator. */
  askChains(step: number): void {
    const panel = this.tree?.panel;
    if (!panel) return;
    const asking = [...new Set(panel.loom.owed.map((prompt) => prompt.chain))];
    if (!asking.length) {
      this.session.notice = "No chain waits on a question of yours.";
      this.render();
      return;
    }
    const at = asking.indexOf(panel.pointer);
    const next =
      asking[(at < 0 ? (step > 0 ? 0 : asking.length - 1) : at + step + asking.length) % asking.length];
    if (next) this.pointChains(next);
  },

  /** The chain under the pointer paused, or woken when a pause holds it. */
  holdChains(): void {
    const panel = this.tree?.panel;
    const id = panel?.pointer ?? "";
    if (!panel || !id || id.startsWith("rest:")) return;
    const row = panel.loom.rows.find((one) => one.id === id);
    this.action(`/${row?.state === "paused" ? "wake" : "pause"} ${id}`);
  },

  /** What the chain under the pointer waits on, cancelled. */
  cancelChains(): void {
    const panel = this.tree?.panel;
    const row = panel?.loom.rows.find((one) => one.id === panel.pointer);
    if (row?.waits) this.action(`/cancel ${row.waits}`);
    else this.session.notice = `${row?.label ?? "This chain"} waits on nothing to cancel.`;
  },
});

export type LoomsPart = typeof looms;
