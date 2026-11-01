import { isAbsolute, relative } from "node:path";
import { asksOperator } from "furb/life";
import { shortenHome } from "../files.ts";
import { clip } from "../format.ts";
import type { ActRow } from "../session.ts";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { bold, type Part, plain } from "../ui.ts";
import { part } from "./part.ts";
import { statusMark, type TreeRow } from "./shared.ts";

export const tree = part({
  /** The rewind tree in the feed, with the pointer on the last message of the chain: Enter then gives that message
   * back on a new branch that has not read it. A paused chain is woken first. */
  rewind(): void {
    if (this.session.paused) {
      this.openPalette("Wake this chain before rewinding", [
        {
          label: "Wake chain",
          detail: "Then choose the point that the new branch starts from.",
          run: () => (this.session.host.pending.size ? this.resume() : this.action("/wake")),
        },
      ]);
      return;
    }
    const points = this.session.activity.filter((act) => this.isPoint(act));
    const last = points.findLast((act) => this.session.isUserPrompt(act)) ?? points.at(-1);
    this.openTree(last?.id ?? this.session.selected, "Rewind");
  },

  /** A rewind to a point. When a write changed a file after it, it asks first what comes back: the conversation on a
   * new branch, the files on this chain, or both. A change that a command made is not restored. */
  async rewindTo(act: ActRow): Promise<void> {
    const w = this.session;
    const found = await w.filesAt(act.id);
    const { files, commands } = found;
    if (!files.length) {
      await w.rewind(act.id);
      return;
    }
    const message = w.isUserPrompt(act);
    const count = files.length === 1 ? "the file" : `the ${files.length} files`;
    const named = files.map(({ path }) => {
      const near = relative(w.workingDirectory, path);
      return near.startsWith("..") || isAbsolute(near) ? shortenHome(path) : near;
    });
    const stays = commands
      ? `${commands} ${commands === 1 ? "command" : "commands"} ran after it: a change that a command made is not restored.`
      : "A change that a command made is not restored.";
    this.openPalette(
      `Rewind to ${act.id}`,
      [
        {
          label: "Restore the conversation",
          detail: message
            ? "A new branch that has not read this message, with the message in the input. The files stay."
            : "A new branch that reads the chain up to this point. The files stay as they are.",
          run: () => w.rewind(act.id).then(() => {}),
        },
        {
          label: "Restore the files",
          detail: `Put back ${count} that a write changed after this point. The conversation stays.`,
          run: () => w.restoreAt(act.id, found),
        },
        {
          label: "Both",
          detail: `A new branch from this point, and ${count} put back`,
          run: () => w.rewind(act.id, undefined, found).then(() => {}),
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      { note: `${clip(named.join(", "), 200)}. ${stays}` },
    );
  },

  /** The tree of the session in the feed, with the pointer on the chain shown. */
  sessionTree(): void {
    this.openTree(this.session.selected, "Session tree");
  },

  /** Whether an act is a point that a branch can start from: an act that a turn of its chain tells, which is no
   * chain, no grant, and no rung that its chain wrote to retell its source. */
  isPoint(act: ActRow): boolean {
    return (
      !["chain", "grant"].includes(act.kind) &&
      (act.kind !== "rung" || this.session.actOf(act.by)?.kind !== "chain")
    );
  },

  openTree(selected: string, title: string): void {
    this.closeOverlay();
    this.closeSearch();
    this.treeFolds.clear();
    this.tree = { rows: [], selected, title };
    this.tree.rows = this.treeRows();
    if (!this.tree.rows.some((row) => row.id === selected)) this.tree.selected = this.tree.rows[0]?.id ?? "";
    this.composer.blur();
    this.render();
  },

  closeTree(): void {
    if (!this.tree) return;
    this.tree = undefined;
    this.composer.focus();
    this.render();
  },

  /** The row of the tree that the pointer stands on. */
  treeRow(): TreeRow | undefined {
    return this.tree?.rows.find((row) => row.id === this.tree?.selected);
  },

  /** The rows of the tree. A chain holds its acts, each act holds the acts it made, and a branch stands under the
   * point it starts from. The chain shown and the chains above it are open, and the others folded. */
  treeRows(): TreeRow[] {
    const w = this.session;
    const chains = w.chains;
    const order = new Map(w.acts.map((act, index) => [act.id, index]));
    const source = (chain: ActRow) => {
      const id = chain.words[1];
      return typeof id === "string" && chains.some((one) => one.id === id) ? id : undefined;
    };
    const path = new Set<string>();
    for (let id: string | undefined = w.selected; id && !path.has(id); ) {
      path.add(id);
      const chain = chains.find((one) => one.id === id);
      id = chain && source(chain);
    }
    // A branch starts after the last point of its source that it reads: the points made before it, but those that the
    // rung that made it took out, and that rung.
    const branches = new Map<string, ActRow[]>();
    for (const chain of chains) {
      const from = source(chain);
      if (!from) continue;
      const maker = w.acts.find((act) => act.id === chain.by);
      const taken = String(maker?.words[0] ?? "").match(/take\(([^)]*)\)/)?.[1] ?? "";
      const omitted = new Set([...taken.matchAll(/"([^"]+)"/g)].map((match) => match[1]));
      const made = order.get(chain.id) ?? 0;
      const point = w.acts.findLast(
        (act) =>
          act.on === from &&
          this.isPoint(act) &&
          (order.get(act.id) ?? 0) < made &&
          !omitted.has(act.id) &&
          act.id !== chain.by &&
          act.by !== chain.by,
      );
      const at = point?.id ?? from;
      branches.set(at, [...(branches.get(at) ?? []), chain]);
    }
    const rows: TreeRow[] = [];
    type Node = { act: ActRow; chain: boolean };
    const visit = (node: Node, lines: string, last: boolean, depth: number, up?: string) => {
      const { act } = node;
      const children: Node[] = [
        ...(node.chain
          ? w.acts.filter(
              (one) =>
                one.on === act.id &&
                this.isPoint(one) &&
                !w.acts.some((maker) => maker.id === one.by && maker.on === act.id && this.isPoint(maker)),
            )
          : w.acts.filter((one) => one.by === act.id && one.on === act.on && this.isPoint(one))
        ).map((one) => ({ act: one, chain: false })),
        ...(branches.get(act.id) ?? []).map((one) => ({ act: one, chain: true })),
      ];
      const folded = this.treeFolds.get(act.id) ?? (node.chain && !path.has(act.id));
      const branch = depth === 0 ? "" : `${lines}${last ? "└─ " : "├─ "}`;
      rows.push({
        ...this.treeLabel(node.act, node.chain),
        id: act.id,
        lines: branch,
        parent: children.length > 0,
        folded,
        up,
      });
      if (folded) return;
      const inner = depth === 0 ? "" : `${lines}${last ? "   " : "│  "}`;
      for (const [index, child] of children.entries())
        visit(child, inner, index === children.length - 1, depth + 1, act.id);
    };
    const roots = chains.filter((chain) => !source(chain));
    for (const root of roots) visit({ act: root, chain: true }, "", true, 0);
    return rows;
  },

  /** The label of a row of the tree, the tag at its right, and what choosing it does. */
  treeLabel(act: ActRow, chain: boolean): Pick<TreeRow, "parts" | "tag" | "hint" | "run"> {
    const w = this.session;
    if (chain) {
      const current = act.id === w.selected;
      const status = this.session.chainStatus(act.id);
      return {
        parts: [statusMark(status), [w.labelOf(act.id), current ? c.text : c.muted, bold]],
        tag: current ? "current" : act.id,
        hint: "open it",
        run: async () => {
          this.closeTree();
          await w.select(act.id);
        },
      };
    }
    const branch = async () => {
      this.closeTree();
      await this.rewindTo(act);
    };
    if (w.isUserPrompt(act) && !asksOperator(act))
      return {
        parts: [
          [`${glyph.prompt} `, c.accent],
          [String(act.words[1] ?? "").split("\n")[0] ?? "", c.text],
        ],
        tag: act.id,
        hint: "edit it on a new branch",
        run: branch,
      };
    const { mark, color } = this.actState(act);
    const subject = this.subject(act);
    const observation = act.kind === "prompt" && !asksOperator(act) && !w.isUserPrompt(act);
    const name = asksOperator(act) ? "question" : act.kind === "rung" ? act.id : act.kind;
    return {
      parts: observation
        ? [
            [`${mark} `, color],
            [subject.split("\n")[0] ?? "", c.muted],
            [`  ${this.sender(act)}`, c.faint],
          ]
        : [
            [`${mark} `, color],
            [name, c.text],
            // A rung shows the first line of its program, and any other act its words on one line, as the feed does.
            [
              `  ${act.kind === "rung" ? (subject.split("\n")[0] ?? "") : subject.replace(/\s+/g, " ").trim()}`,
              c.muted,
            ],
          ],
      tag: act.kind === "rung" ? "" : act.id,
      hint: "branch after it",
      run: branch,
    };
  },

  renderTree(): void {
    const tree = this.tree;
    if (!tree) return;
    if (tree.panel) {
      this.renderChains();
      return;
    }
    const selected = tree.selected;
    tree.rows = this.treeRows();
    if (!tree.rows.some((row) => row.id === selected)) tree.selected = tree.rows[0]?.id ?? "";
    const width = this.feedWidth;
    if (this.paneChanged(this.treeBar, [this.theme, width, tree.title])) {
      this.clear(this.treeBar);
      const note =
        tree.title === "Rewind"
          ? "Choose a point to go back to, with the files as they were or not. A change by a command stays."
          : "Open a chain, or go back to a point, with the files as they were or not. A change by a command stays.";
      // The note is cut at its end where the bar has no room for it beside the title and the key that leaves.
      const room =
        width - space.inset * 2 - Bun.stringWidth(tree.title) - space.between * 2 - Bun.stringWidth("Esc");
      this.treeBar.add(
        this.line(
          [
            [tree.title, c.text, bold],
            [`${" ".repeat(space.between)}${clip(note, Math.max(0, room))}`, c.muted],
          ],
          c.muted,
          { flexGrow: 1, flexShrink: 1 },
        ),
      );
      this.treeBar.add(this.text("Esc", c.faint, { run: () => this.closeTree() }));
    }
    if (
      !this.paneChanged(this.scroll, [
        tree.rows.map((row) => [row.id, row.lines, plain(row.parts), row.tag, row.folded, row.parent]),
        tree.selected,
        width,
        this.theme,
      ])
    )
      return;
    this.clear(this.scroll);
    for (const [index, row] of tree.rows.entries()) {
      const active = row.id === tree.selected;
      const line = this.row({
        id: `tree-${row.id}`,
        // Each tree of chains stands apart from the one above it.
        marginTop: index && !row.lines ? space.section : 0,
        paddingX: space.inset,
        backgroundColor: active ? c.selected : c.background,
        onMouseUp: this.click((event) => {
          // A click on the fold of a row folds it, a click on another row points at it, and a click on the row
          // that the pointer is on chooses it.
          const fold = event.x - (this.scroll.x + space.inset + Bun.stringWidth(row.lines)) <= 1;
          if (row.parent && fold) this.foldTree(row.id, !row.folded);
          else if (active) void this.chooseTreeRow();
          else this.pointTree(row.id);
        }),
      });
      if (!active) this.hoverable(line, c.panel);
      const fold = row.parent ? `${row.folded ? glyph.closed : glyph.open} ` : "  ";
      const tag = row.tag ? `  ${row.tag}` : "";
      const room = Math.max(8, width - space.inset * 2 - Bun.stringWidth(row.lines + fold + tag));
      const label = plain(row.parts);
      line.add(
        this.line(
          [
            [row.lines, c.faint],
            [fold, c.faint],
            ...(Bun.stringWidth(label) > room ? this.clipParts(row.parts, room) : row.parts),
          ],
          c.text,
          { flexGrow: 1, flexShrink: 1, truncate: true },
        ),
      );
      if (tag) line.add(this.line(tag, active ? c.muted : c.faint));
      this.scroll.add(line);
    }
    this.scrollAfterLayout({ card: `tree-${tree.selected}` });
  },

  /** Parts cut to a width at their end, with the mark of a cut. */
  clipParts(parts: readonly Part[], width: number): Part[] {
    const cut: Part[] = [];
    let left = width;
    for (const [text, fg, attributes, bg] of parts) {
      if (left <= 0) break;
      const size = Bun.stringWidth(text);
      if (size < left || (size === left && cut.length === parts.length - 1))
        cut.push([text, fg, attributes, bg]);
      else cut.push([clip(text, left), fg, attributes, bg]);
      left -= size;
    }
    return cut;
  },

  pointTree(id: string): void {
    if (!this.tree) return;
    this.tree.selected = id;
    this.renderContent();
    this.renderStatus();
  },

  moveTree(step: number): void {
    const tree = this.tree;
    if (!tree?.rows.length) return;
    const at = tree.rows.findIndex((row) => row.id === tree.selected);
    const next = tree.rows[Math.max(0, Math.min(tree.rows.length - 1, at + step))];
    if (next) this.pointTree(next.id);
  },

  foldTree(id: string, folded: boolean): void {
    this.treeFolds.set(id, folded);
    this.renderContent();
    this.renderStatus();
  },

  async chooseTreeRow(): Promise<void> {
    const row = this.treeRow();
    if (!row) return;
    if (row.id === this.session.selected && this.session.chains.some((chain) => chain.id === row.id)) {
      this.closeTree();
      return;
    }
    await Promise.resolve(row.run()).catch(this.report);
  },
});

export type TreePart = typeof tree;

export const treeBound = ["rewind", "sessionTree", "closeTree"] as const;
