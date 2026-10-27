import { basename, join } from "node:path";
import { type BoxOptions, InputRenderable, InputRenderableEvents, type Renderable } from "@opentui/core";
import { checkoutOf } from "../checkout.ts";
import { shortenHome } from "../files.ts";
import { clip } from "../format.ts";
import { type Cancel, statusLabels } from "../session.ts";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { bold, type Part, plain } from "../ui.ts";
import type { SessionEntry, Workspace } from "../workspaces.ts";
import { part } from "./part.ts";
import { type Choice, inputColors, sessionDetail, statusMark } from "./shared.ts";

export const spaces = part({
  /** The workspaces in the sidebar, under the session: each folder with its sessions, which scroll on their own. */
  renderWorkspaces(): void {
    const library = this.options.workspaces;
    const width = this.session.preferences.sidebarWidth;
    if (
      !this.paneChanged(this.railSpaces, [
        this.theme,
        library.current?.path,
        width,
        library.notice,
        library.groups.map((group) => [
          group.name,
          group.directory,
          group.collapsed,
          group.sessions.map((entry) => [entry.path, entry.name, entry.status, entry.error, entry.archived]),
        ]),
        [...this.unfolded],
        this.renaming && this.itemKey(this.renaming.item),
        this.menuItem && this.itemKey(this.menuItem),
      ])
    )
      return;
    this.clear(this.railHeading);
    this.clear(this.railSpaces);
    this.unhover();
    const inner = width - space.between * 2;
    this.railHeading.add(this.text("Workspaces", c.text, { attributes: bold, flexGrow: 1 }));
    this.railHeading.add(this.text("⌃W", c.faint, { run: () => void this.workspacePicker() }));
    const padding = { paddingLeft: space.inset, paddingRight: space.between };
    const line = (parts: Part[], run: () => void, options: BoxOptions = {}) =>
      this.railSpaces.add(this.railRow(parts, run, { ...padding, ...options }));
    if (library.notice) this.railSpaces.add(this.inset(space.between, this.text(library.notice, c.warning)));
    if (!library.groups.length)
      this.railSpaces.add(
        this.inset(space.between, this.text("No workspaces yet. Add a project folder below.", c.muted)),
      );
    // A session and a workspace take one shape of row. While the pointer is on the row, it lights, and at its end a
    // button renames it and a button removes it, which asks first. The buttons are drawn in the color of the row until
    // then, so the row keeps its layout. The mark of the state tells the state in a tip, and a right click opens the
    // menu of the row. A row under a new name holds an input in place of its name.
    const itemRow = (
      item: SessionEntry | Workspace,
      parts: { lead: Part[]; name: Part; state?: Part; after?: boolean },
      tip: () => Part[],
      run: () => void,
      options: BoxOptions = {},
      selected = false,
    ) => {
      const noun = "sessions" in item ? "workspace" : "session";
      const lit = selected ? c.selected : c.raised;
      const ground = selected ? c.selected : this.menuItem === item ? lit : c.panel;
      // A click in the input of a new name only moves its cursor.
      const renaming = this.renaming?.item === item ? this.renaming : undefined;
      const row = this.row({
        paddingLeft: space.inset,
        paddingRight: space.between,
        backgroundColor: renaming ? lit : ground,
        ...(renaming ? {} : { onMouseUp: this.click(run) }),
        onMouseDown: (event) => {
          if (event.button !== 2) return;
          event.stopPropagation();
          this.itemMenu(item, event.x, event.y);
        },
        ...options,
      });
      row.add(this.line(parts.lead, c.muted));
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
        this.railSpaces.add(row);
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
      const button = (mark: string, act: () => void) =>
        this.line(mark, ground, {
          marginLeft: space.inset,
          onMouseUp: this.click((event) => {
            event.stopPropagation();
            act();
          }),
        });
      const rename = button(glyph.rename, () => this.startRename(item));
      const remove = button(glyph.remove, () => this.removeItem(item));
      row.add(rename);
      row.add(remove);
      row.onMouseOver = (event) => {
        row.backgroundColor = lit;
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
                : undefined;
        if (told) this.tip(told, event.x, event.y);
      };
      row.onMouseOut = () => {
        row.backgroundColor = ground;
        rename.fg = ground;
        remove.fg = ground;
        this.unhover();
      };
      this.railSpaces.add(row);
      return row;
    };
    for (const [index, group] of library.groups.entries()) {
      const status = library.groupStatus(group);
      const current = group === library.groupOf();
      const quiet = status === "saved" || status === "idle";
      const count = group.sessions.filter((entry) => entry.status === status).length;
      itemRow(
        group,
        {
          lead: [[" "], [`${group.collapsed ? glyph.closed : glyph.open} `, c.faint]],
          name: [group.name, current ? c.text : c.muted, bold],
          state: quiet ? undefined : statusMark(status),
          after: true,
        },
        () => [
          statusMark(status),
          [statusLabels[status], c.text, bold],
          [
            `  ${count} of ${group.sessions.length} ${group.sessions.length === 1 ? "session" : "sessions"}`,
            c.muted,
          ],
        ],
        () => library.toggle(group),
        { marginTop: index ? space.section : space.stack },
      );
      // A folded workspace shows its name alone, and an open one its folder under its name.
      if (group.collapsed) continue;
      this.railSpaces.add(
        this.inset(
          space.inset + 3,
          this.whole(
            this.line(clip(shortenHome(group.directory), inner - space.inset - 1, "end"), c.faint, {
              truncate: true,
            }),
            () => shortenHome(group.directory),
          ),
        ),
      );
      const sessionRow = (entry: SessionEntry) => {
        const selected = library.current === entry;
        // A session that runs in a worktree of the workspace names the worktree after its own name.
        const checkout = checkoutOf(entry.path, group.directory);
        const label = checkout === group.directory ? entry.name : `${entry.name}  ${basename(checkout)}`;
        itemRow(
          entry,
          {
            lead: [[selected ? glyph.mark : " ", c.accent], ["  "]],
            state: statusMark(entry.status),
            name: [label, selected ? c.text : entry.archived ? c.faint : c.muted, selected ? bold : 0],
          },
          () => [
            statusMark(entry.status),
            [statusLabels[entry.status], c.text, bold],
            [entry.error ? `  ${entry.error}` : "", c.danger],
          ],
          () => {
            void library.select(entry).catch(this.report);
          },
          {},
          selected,
        );
      };
      // The sidebar shows the sessions that live: the one shown, and each that works, asks, waits for a wake or
      // failed, and the newest of the others up to three rows. The older sessions fold under a row of their own,
      // which a click opens.
      const kept = group.sessions.filter((entry) => !entry.archived);
      const live = (entry: SessionEntry) =>
        library.current === entry ||
        ["working", "opening", "blocked", "paused", "error"].includes(entry.status);
      const resting = kept.filter((entry) => !live(entry));
      const room = Math.max(0, 3 - (kept.length - resting.length));
      const shown = new Set([...kept.filter(live), ...resting.slice(0, room)]);
      for (const entry of kept.filter((entry) => shown.has(entry))) sessionRow(entry);
      const earlier = kept.filter((entry) => !shown.has(entry));
      if (earlier.length) {
        this.railSpaces.add(
          this.foldRow(`${group.directory}\nearlier`, "  Earlier", earlier.length, padding),
        );
        if (this.unfolded.has(`${group.directory}\nearlier`)) for (const entry of earlier) sessionRow(entry);
      }
      line([["   "], ["+ ", c.faint], ["New session", c.faint]], () => {
        void library.create(group).catch(this.report);
      });
      // The archived sessions fold under a row of their own, as the finished chains do, which a click opens.
      const archived = group.sessions.filter((entry) => entry.archived);
      if (archived.length) {
        this.railSpaces.add(this.foldRow(group.directory, "  Archived", archived.length, padding));
        if (this.unfolded.has(group.directory)) for (const entry of archived) sessionRow(entry);
      }
    }
    line([[" "], ["+ ", c.faint], ["Add a workspace", c.faint]], () => this.insert("/workspace "), {
      marginTop: space.section,
    });
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
            { label: item.collapsed ? "Unfold" : "Fold", detail: "", run: () => library.toggle(item) },
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
