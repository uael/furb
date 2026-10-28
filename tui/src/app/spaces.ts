import { join } from "node:path";
import {
  type BoxOptions,
  type BoxRenderable,
  InputRenderable,
  InputRenderableEvents,
  type Renderable,
} from "@opentui/core";
import { shortenHome } from "../files.ts";
import type { Cancel } from "../session.ts";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { type Part, plain } from "../ui.ts";
import type { SessionEntry, Workspace } from "../workspaces.ts";
import { part } from "./part.ts";
import { type Choice, inputColors, sessionDetail } from "./shared.ts";

export const spaces = part({
  /** A row of a session or a workspace, in a list of the sidebar. While the pointer is on the row, it lights, and at its
   * end a button renames it and a button removes it, which asks first. The buttons are drawn in the color of the row
   * until then, so the row keeps its layout. The mark of the state tells the state in a tip, and a right click opens
   * the menu of the row. A row under a new name holds an input in place of its name. */
  itemRow(
    into: Renderable,
    item: SessionEntry | Workspace,
    parts: {
      lead: Part[];
      name: Part;
      state?: Part;
      after?: boolean;
      /** What stands at the right of the name, before the buttons: the badges of what needs a look. */
      tail?: Part[];
      tailTip?: () => Part[];
      /** What a click on the lead does, which is the fold of a workspace to peek at its sessions. */
      peek?: () => void;
    },
    tip: () => Part[],
    run: () => void,
    options: BoxOptions = {},
    selected = false,
  ): BoxRenderable {
    const noun = "sessions" in item ? "workspace" : "session";
    const lit = selected ? c.selected : c.raised;
    const ground = selected ? c.selected : this.menuItem === item ? lit : c.panel;
    // A click in the input of a new name only moves its cursor.
    const renaming = this.renaming?.item === item ? this.renaming : undefined;
    const row = this.row({
      paddingLeft: space.inset,
      paddingRight: space.inset,
      backgroundColor: renaming ? lit : ground,
      ...(renaming ? {} : { onMouseUp: this.click(run) }),
      onMouseDown: (event) => {
        if (event.button !== 2) return;
        event.stopPropagation();
        this.itemMenu(item, event.x, event.y);
      },
      ...options,
    });
    const peek = parts.peek;
    row.add(
      this.line(
        parts.lead,
        c.muted,
        peek
          ? {
              onMouseUp: this.click((event) => {
                event.stopPropagation();
                peek();
              }),
            }
          : {},
      ),
    );
    const state = parts.state && this.line([parts.state], c.muted);
    if (state && !parts.after) row.add(state);
    if (renaming) {
      const input = new InputRenderable(this.renderer, {
        flexGrow: 1,
        flexShrink: 1,
        value: renaming.value,
        ...inputColors(c.background),
      });
      input.on(InputRenderableEvents.INPUT, (value: string) => {
        renaming.value = value;
      });
      // A row that the sidebar draws again keeps the cursor where it was.
      if (renaming.cursor !== undefined) input.cursorOffset = renaming.cursor;
      input.onKeyDown = () => {
        queueMicrotask(() => {
          if (!input.isDestroyed) renaming.cursor = input.cursorOffset;
        });
      };
      input.on(InputRenderableEvents.ENTER, () => this.endRename(true));
      // The name is kept when the input loses its focus to another part of the screen, and not when the sidebar
      // draws the row again, which focuses its new input at once.
      input.on("blurred", () =>
        queueMicrotask(() => {
          if (this.renaming === renaming && !renaming.input?.focused) this.endRename(true);
        }),
      );
      renaming.input = input;
      row.add(input);
      into.add(row);
      input.focus();
      return row;
    }
    const name = this.line([parts.name], c.muted, {
      truncate: true,
      flexShrink: 1,
      flexGrow: parts.after ? 0 : 1,
    });
    row.add(name);
    if (state && parts.after) {
      state.marginLeft = space.between;
      row.add(state);
    }
    if (parts.after) row.add(this.box({ flexGrow: 1 }));
    const tail = parts.tail?.length
      ? this.line(parts.tail, c.muted, { marginLeft: space.between })
      : undefined;
    if (tail) row.add(tail);
    // The buttons stand over the right end of the row while the pointer is on it, so that they take no column from
    // the marks and the cost, which end where the marks of every other row of the sidebar end.
    const buttons = this.row({
      position: "absolute",
      right: space.inset,
      top: 0,
      backgroundColor: lit,
      visible: false,
    });
    const button = (mark: string, act: () => void) =>
      this.line(mark, c.muted, {
        marginLeft: space.inset,
        onMouseUp: this.click((event) => {
          event.stopPropagation();
          act();
        }),
      });
    const rename = button(glyph.rename, () => this.startRename(item));
    const remove = button(glyph.remove, () => this.removeItem(item));
    buttons.add(rename);
    buttons.add(remove);
    row.add(buttons);
    row.onMouseOver = (event) => {
      row.backgroundColor = lit;
      buttons.visible = true;
      rename.fg = event.target === rename ? c.accent : c.muted;
      remove.fg = event.target === remove ? c.danger : c.muted;
      // A name that its room cuts shows whole in a tip of its own.
      if (event.target === name) return;
      this.unhover();
      if (event.isDragging) return;
      const told: Part[] | undefined =
        event.target === rename
          ? [[`Rename this ${noun}`, c.text]]
          : event.target === remove
            ? [
                [
                  "sessions" in item
                    ? "Remove this workspace from the list"
                    : "Archive or remove this session",
                  c.text,
                ],
              ]
            : event.target === state
              ? tip()
              : tail && event.target === tail
                ? parts.tailTip?.()
                : undefined;
      if (told) this.tip(told, event.x, event.y);
    };
    row.onMouseOut = () => {
      row.backgroundColor = ground;
      buttons.visible = false;
      this.unhover();
    };
    into.add(row);
    return row;
  },

  /** A tip that stands under the pointer and ends at its left, or above it where the rows under it cannot hold it,
   * inside the screen. It never covers the row of the pointer, or the pointer would leave the text that shows it. */
  tip(parts: Part[], x: number, y: number): void {
    const size = Math.min(
      this.renderer.width - 2,
      Math.max(25, Bun.stringWidth(plain(parts)) + space.between),
    );
    const rows = Math.max(1, Math.ceil(Bun.stringWidth(plain(parts)) / Math.max(1, size - space.inset * 2)));
    this.popup(
      {
        left: Math.max(1, Math.min(x - size, this.renderer.width - size - 1)),
        top: y + 1 + rows <= this.renderer.height ? y + 1 : Math.max(0, y - rows),
        width: size,
        paddingX: space.inset,
      },
      this.text(parts),
    );
  },

  /** A popup over the screen, in place of the one before it, which holds nodes until the pointer leaves what shows
   * it. */
  popup(options: BoxOptions, ...nodes: Renderable[]): void {
    this.unhover();
    this.hover = this.box({ position: "absolute", backgroundColor: c.raised, zIndex: 30, ...options });
    for (const node of nodes) this.hover.add(node);
    this.root.add(this.hover);
  },

  /** The popup gone, where one shows. */
  unhover(): void {
    this.hover?.destroyRecursively();
    this.hover = undefined;
  },

  /** Ask how to remove a session: archive it, which keeps its record under a row of its own, or move it to the trash.
   * An archived session offers to come back instead. */
  removeSession(entry: SessionEntry): void {
    const library = this.options.workspaces;
    const group = library.groupOf(entry);
    if (!group) return;
    this.openPalette(
      `Remove the session “${entry.name}”?`,
      [
        entry.archived
          ? {
              label: "Restore",
              detail: "Bring it back to the list of its workspace",
              run: () => library.restore(entry),
            }
          : {
              label: "Archive",
              detail: "Fold it under Archived. Its record stays, and a click opens it again.",
              run: () => void library.archive(entry).catch(this.report),
            },
        {
          label: "Move to trash",
          color: c.danger,
          detail: "Stop this session and move its saved record to the trash",
          run: () => library.delete(entry).then(() => {}),
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      {
        note: `The trash is ${shortenHome(join(group.directory, ".furb", "trash"))}, where you can get it back.`,
      },
    );
  },

  /** What names a session or a workspace for as long as it lives: the path of its record, or its folder. */
  itemKey(item: SessionEntry | Workspace): string {
    return "sessions" in item ? item.directory : item.path;
  },

  /** Rename a session or a workspace in its row: an input takes the place of its name, Enter keeps what it holds,
   * and Escape leaves the name as it was. */
  startRename(item: SessionEntry | Workspace): void {
    this.closeOverlay();
    this.renaming = { item, value: item.name };
    this.session.notice = "Enter keeps the new name, and Esc leaves it as it was.";
    this.render();
  },

  endRename(keep: boolean): void {
    const renaming = this.renaming;
    if (!renaming) return;
    this.renaming = undefined;
    const name = renaming.value.trim();
    if (keep && name && name !== renaming.item.name) this.options.workspaces.rename(renaming.item, name);
    if (this.session.notice.startsWith("Enter keeps the new name")) this.session.notice = "";
    // The input gives its focus back to the composer, and a part that took the focus from it keeps it.
    const focused = this.renderer.currentFocusedRenderable;
    if (!focused || focused === renaming.input || focused.isDestroyed) this.composer.focus();
    this.render();
  },

  /** A cancel that ends running work asks first: it says how many acts it ends, and on which chains. */
  confirm(cancel: Cancel): void {
    const { act, reach, back } = cancel;
    const ended = reach.acts.length;
    const acts = `${ended} running ${ended === 1 ? "act" : "acts"}`;
    const where = reach.chains.length === 1 ? "one chain" : `${reach.chains.length} chains`;
    this.openPalette(
      `${back ? "Take back" : "Cancel"} ${act.id}, and end ${acts} on ${where}?`,
      [
        {
          label: back ? "Take it back" : "Cancel it",
          color: c.danger,
          detail: `End ${act.id} and the ${acts} under it`,
          run: () => this.session.end(cancel),
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      { note: `It ends work on ${this.session.chainsOf(reach)}.` },
    );
  },

  /** Ask how to remove a session, or whether a workspace leaves the list. */
  removeItem(item: SessionEntry | Workspace): void {
    if (!("sessions" in item)) {
      this.removeSession(item);
      return;
    }
    const library = this.options.workspaces;
    this.openPalette(
      `Remove the workspace “${item.name}” from the list?`,
      [
        {
          label: "Remove from the list",
          detail: "Its folder and its sessions stay on disk, and /workspace adds it back",
          run: () => library.remove(item),
        },
        { label: "Keep", detail: "Return without changes", run() {} },
      ],
      { note: `The folder is ${shortenHome(item.directory)}.` },
    );
  },

  /** The menu that a right click on the row of a session or a workspace opens, at the pointer. */
  itemMenu(item: SessionEntry | Workspace, x: number, y: number): void {
    const library = this.options.workspaces;
    const rename = { label: "Rename", detail: "", run: () => this.startRename(item) };
    const choices: Choice[] =
      "sessions" in item
        ? [
            { label: "New session", detail: "", run: () => library.create(item).then(() => {}) },
            rename,
            ...(library.groupOf() === item
              ? []
              : [
                  {
                    label: this.unfolded.has(`ws:${item.directory}`) ? "Fold" : "Unfold",
                    detail: "",
                    run: () => this.peekWorkspace(item),
                  },
                ]),
            { label: "Remove from the list", detail: "", run: () => this.removeItem(item) },
          ]
        : [
            ...(library.current === item
              ? []
              : [{ label: "Open", detail: "", run: () => library.select(item) }]),
            rename,
            item.archived
              ? { label: "Restore", detail: "", run: () => library.restore(item) }
              : { label: "Archive", detail: "", run: () => library.archive(item) },
            { label: "Move to trash", detail: "", color: c.danger, run: () => this.removeItem(item) },
          ];
    this.openPalette(item.name, choices, { at: { x, y } });
    this.menuItem = item;
    this.render();
  },

  /** The workspaces and their sessions in a palette, which opens once the workspaces are read again. */
  workspacePicker(): Promise<void> {
    const library = this.options.workspaces;
    return library
      .refresh()
      .then(() =>
        this.openPalette(
          "Workspaces and sessions",
          [
            {
              label: "Add a workspace",
              detail: "Open a project folder",
              mark: ["+ ", c.accent],
              run: () => this.insert("/workspace "),
            },
            // Each workspace is a part of the list with its folder in its title, and its sessions under it.
            ...library.groups.flatMap((group) => [
              ...group.sessions.map((entry) => ({
                label: entry.name,
                detail: sessionDetail(entry, library.current === entry),
                status: entry.status,
                heading: `${group.name}   ${shortenHome(group.directory)}`,
                run: () => library.select(entry),
              })),
              {
                label: "New session",
                detail: `Start a fresh life in ${group.name}`,
                mark: ["+ ", c.faint] as Part,
                heading: `${group.name}   ${shortenHome(group.directory)}`,
                run: async () => {
                  await library.create(group);
                },
              },
            ]),
          ],
          { note: "Open sessions keep running while you work in another one." },
        ),
      )
      .catch(this.report);
  },
});

export type SpacesPart = typeof spaces;

export const spacesBound = ["confirm", "workspacePicker"] as const;
