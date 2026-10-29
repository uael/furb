import { InputRenderable, InputRenderableEvents, RGBA } from "@opentui/core";
import { commands, slashes } from "../commands.ts";
import { clip } from "../format.ts";
import { bindings, chords, keys, shown } from "../keys.ts";
import { type View, views } from "../session.ts";
import { theme as c, glyph, spacing as space, spin } from "../theme.ts";
import { bold, type Part } from "../ui.ts";
import { part } from "./part.ts";
import { type Choice, inputColors, statusMark, title, viewLabels, wrapped } from "./shared.ts";

export const palette = part({
  /** Every action in one list: each with its label, the slash command that does it, what it does, and its keys. The
   * table of keys gives the chord of each command, and each action that has keys and no command, which stands before
   * the command that it goes with. */
  palette(): void {
    const views_: Record<View, string> = {
      feed: "Messages, the Python each model wrote, and answers",
      transcript: "The exact text that the model reads",
      changes: "The diff of each file that the life wrote",
    };
    const keysOf = new Map<string, string>();
    const before = new Map<string, Choice>();
    for (const { key, binding } of bindings) {
      if (binding.command) keysOf.set(binding.command, shown(key, binding, this.kitty));
      const { choice, run } = binding;
      if (choice && run)
        before.set(choice.before, {
          label: choice.label,
          detail: choice.detail,
          command: "",
          keys: shown(key, binding, this.kitty),
          run: () => void this.actions[run](),
        });
    }
    const choices: Choice[] = [
      ...(this.session.host.reading
        ? [
            {
              label: "Read only",
              detail: "Why this record opens to read, and how to start a new life from it",
              command: "",
              run: () => this.readOrRenew(),
            },
          ]
        : []),
      { label: "New session", detail: commands.new[2], command: "/new", run: () => this.action("/new") },
    ];
    // One key names the three views, as 1-3.
    const toggle = bindings.find(({ binding }) => binding.run === "view");
    for (const [index, view] of views.entries())
      choices.push({
        label: `${viewLabels[view]} view`,
        detail: views_[view],
        command: "",
        keys: toggle && shown(toggle.key, toggle.binding, this.kitty).replace("1-3", String(index + 1)),
        run: () => this.showView(view),
      });
    for (const { name, label, argument, detail, usage } of slashes) {
      if (name === "new") continue;
      const aside = before.get(name);
      if (aside) choices.push(aside);
      choices.push({
        label,
        detail,
        command: usage,
        keys: keysOf.get(name),
        run: this.command(name, argument),
      });
    }
    for (const part of this.parts.loaded)
      for (const one of part.commands)
        choices.push({
          label: title(one.name),
          detail: `${one.detail} (${part.name})`,
          command: `/${one.name}`,
          run: () => this.action(`/${one.name}`),
        });
    this.openPalette("Commands", choices, { rich: true });
  },

  /** A dialog of choices, which a filter narrows: with the choice that it selects first, a note under its title, and
   * the rows of a rich choice. At a point, it is a menu as wide as its labels need, which stands under the point, or
   * over it where the rows under it cannot hold it, and leaves the screen behind it as it is. */
  openPalette(
    label: string,
    choices: Choice[],
    {
      selected = 0,
      note = "",
      rich = false,
      at,
    }: { selected?: number; note?: string | string[]; rich?: boolean; at?: { x: number; y: number } } = {},
  ): void {
    // A dialog or a menu takes the keys, so a name that a row takes ends first, and is kept.
    this.endRename(true);
    this.closeOverlay();
    this.rich = rich;
    this.paletteStart = 0;
    this.composer.blur();
    this.unhover();
    // The dialog is as wide as its longest label and detail need, from 80 columns to 110, and no wider than the feed.
    // It stands over the middle of the feed, so that every dialog takes one place and covers no part of the sidebar,
    // and over the middle of the screen only when the feed is narrower than 80 columns.
    const widest = (pick: (choice: Choice) => string) =>
      Math.max(0, ...choices.map((choice) => Bun.stringWidth(pick(choice))));
    const wanted = Math.max(
      80,
      Math.min(110, widest((choice) => choice.label) + widest((choice) => choice.detail) + 10),
    );
    const column = this.feedWidth + space.gutter * 2;
    const stage = column - 4 >= 80 ? column : this.renderer.width;
    // A menu holds its title with the key that closes it, and each label after the pointer of the list.
    const menu = Math.min(
      this.renderer.width - 2,
      space.inset * 2 +
        Math.max(
          Bun.stringWidth(label) + space.between * 2 + 3,
          widest((choice) => choice.label) + 2 + space.between,
        ),
    );
    this.paletteWidth = at ? menu : Math.min(stage - 4, wanted);
    const width = this.paletteWidth;
    // The rows of a menu: its inset, its title with the space under it, and its choices.
    const rows = space.inset * 2 + space.bar + space.section + choices.length;
    // A veil dims the screen behind the dialog, and a click on it closes the dialog. The veil of a menu is clear, and a
    // right click on it closes the menu too.
    this.backdrop = this.box({
      position: "absolute",
      left: 0,
      top: 0,
      width: "100%",
      height: "100%",
      backgroundColor: at ? "transparent" : c.backdrop,
      zIndex: 19,
      onMouseDown: (event) => {
        if (event.button === 2) this.closeOverlay();
      },
      onMouseUp: this.click(() => this.closeOverlay()),
    });
    this.root.add(this.backdrop);
    this.overlay = this.box({
      id: "palette",
      position: "absolute",
      left: at
        ? Math.max(1, Math.min(at.x, this.renderer.width - width - 1))
        : Math.floor((stage - width) / 2),
      top: at
        ? at.y + 1 + rows <= this.renderer.height
          ? at.y + 1
          : Math.max(0, at.y - rows)
        : Math.max(1, Math.min(4, Math.floor(this.renderer.height / 8))),
      width,
      maxHeight: this.paletteHeight,
      paddingX: space.inset,
      paddingY: space.inset,
      gap: space.stack,
      backgroundColor: c.raised,
      zIndex: 20,
    });
    this.root.add(this.overlay);
    // The title and the filter start where the labels of the list start, and the prompt of the filter stands over the
    // pointer of the list.
    const heading = this.row({ paddingLeft: space.between, paddingRight: space.inset });
    heading.add(this.text(label, c.text, { attributes: bold, truncate: true, flexGrow: 1, flexShrink: 1 }));
    heading.add(this.text("Esc", c.faint, { run: () => this.closeOverlay() }));
    this.overlay.add(heading);
    // A note in a list lays out on as many lines as its items need, and a single text takes one line.
    const notes = Array.isArray(note)
      ? note.flatMap((item) => wrapped(item, this.paletteWidth - 8, Number.MAX_SAFE_INTEGER))
      : note
        ? [clip(note, this.paletteWidth - 8)]
        : [];
    this.paletteNote = notes.length * space.bar;
    for (const line of notes)
      this.overlay.add(this.inset(space.between, this.line(clip(line, this.paletteWidth - 8), c.muted)));
    // A short list shows its filter only once the operator types in it, and until then the filter takes no row.
    const quiet = choices.length <= 5;
    this.paletteInputRow = this.row({
      height: quiet ? 0 : space.bar,
      paddingRight: space.inset,
      marginBottom: space.section,
    });
    const prompt = this.text(`${glyph.prompt} `, c.accent, { visible: !quiet });
    this.paletteInputRow.add(prompt);
    this.paletteInput = new InputRenderable(this.renderer, {
      id: "palette-search",
      flexGrow: 1,
      placeholder: quiet ? "" : "Type to filter",
      ...inputColors(c.raised),
    });
    this.paletteInputRow.add(this.paletteInput);
    this.overlay.add(this.paletteInputRow);
    this.paletteList = this.box({
      gap: space.stack,
      onMouseScroll: (event) => {
        const direction = event.scroll?.direction;
        if (direction !== "up" && direction !== "down") return;
        this.selection = Math.max(
          0,
          Math.min(this.filtered.length - 1, this.selection + (direction === "up" ? -1 : 1)),
        );
        this.renderChoices();
      },
    });
    this.overlay.add(this.paletteList);
    // A label that starts with the filter comes first, then a label that holds it, then a detail that holds it.
    const filter = (value: string) => {
      const wanted = value.toLowerCase();
      const rank = (choice: Choice) => {
        const label = choice.label.trim().toLowerCase();
        const command = choice.command?.toLowerCase() ?? "";
        return label.startsWith(wanted) || command.startsWith(wanted) || command.startsWith(`/${wanted}`)
          ? 0
          : label.includes(wanted)
            ? 1
            : 2;
      };
      return choices
        .filter((choice) =>
          `${choice.label} ${choice.detail} ${choice.command ?? ""} ${choice.keys ?? ""}`
            .toLowerCase()
            .includes(wanted),
        )
        .map((choice, index) => ({ choice, index, rank: wanted ? rank(choice) : 0 }))
        .sort((one, other) => one.rank - other.rank || one.index - other.index)
        .map(({ choice }) => choice);
    };
    this.filtered = filter("");
    this.selection = Math.max(0, selected);
    this.paletteInput.on(InputRenderableEvents.INPUT, (value: string) => {
      prompt.visible = !quiet || value !== "";
      if (this.paletteInputRow) this.paletteInputRow.height = prompt.visible ? space.bar : 0;
      this.filtered = filter(value);
      this.selection = 0;
      this.paletteStart = 0;
      this.renderChoices();
    });
    this.paletteInput.on(InputRenderableEvents.ENTER, () => this.choose());
    this.renderChoices();
    this.paletteInput.focus();
  },

  /** The rows the palette may take: all but a row above and below for a question, whose text stands above its
   * input, and all but eight otherwise. */
  get paletteHeight(): number {
    return this.overlay && this.questionDocument?.parent === this.overlay
      ? this.renderer.height - 2
      : Math.max(12, Math.min(34, this.renderer.height - 8));
  },

  renderChoices(): void {
    if (!this.paletteList) return;
    this.clear(this.paletteList);
    // The list has the rows of the palette but its inset, its heading, its note, its input with the space under it,
    // and the text it asks.
    const asked = this.questionDocument?.parent === this.overlay ? (this.questionDocument?.height ?? 0) : 0;
    const room = Math.max(2, this.paletteHeight - space.inset * 2 - 3 * space.bar - this.paletteNote - asked);
    // A rich choice takes a row for its label, a row for its command when it has one, a row for its detail, and a
    // row of space before the next one.
    // A choice that opens a part of the list takes a row more for its title, and a row of space above it.
    const titled = (index: number) =>
      Boolean(this.filtered[index]?.heading) &&
      this.filtered[index]?.heading !== this.filtered[index - 1]?.heading;
    // The first choice that the list shows carries the title of its part, with no space above it.
    const cost = (index: number, first: boolean) => {
      const choice = this.filtered[index];
      if (!choice) return 0;
      const above = first ? (choice.heading ? 1 : 0) : titled(index) ? 2 : 0;
      return (this.rich ? (choice.command ? 3 : 2) : 1) + above;
    };
    const gap = this.rich ? space.section : 0;
    const span = (from: number, to: number) => {
      let sum = (to - from) * gap;
      for (let index = from; index <= to; index++) sum += cost(index, index === from);
      return sum;
    };
    // A list that the palette cannot hold whole keeps a row of space and a row for its count under it, and scrolls as
    // little as it takes to show the selected choice.
    const fits = span(0, this.filtered.length - 1) <= room ? room : room - space.section - space.bar;
    let start = Math.min(this.paletteStart, this.selection);
    while (start < this.selection && span(start, this.selection) > fits) start++;
    this.paletteStart = start;
    let end = start;
    while (end + 1 < this.filtered.length && span(start, end + 1) <= fits) end++;
    const shown = this.filtered.slice(start, end + 1);
    const inner = this.paletteWidth - space.inset * 2;
    // The details stand in one column after the labels, which take at most half of the row, or what short details
    // leave.
    const widest = (width: (choice: Choice) => number) => Math.max(0, ...this.filtered.map(width));
    const column = Math.min(
      Math.max(
        Math.floor(inner / 2),
        inner -
          space.between -
          widest(
            (choice) =>
              Bun.stringWidth(choice.detail.replace(/\s*\n\s*/g, " ")) +
              (choice.swatch ? choice.swatch.length * 2 + space.between : 0),
          ),
      ),
      widest((choice) => Bun.stringWidth(choice.label) + (choice.mark || choice.status ? 2 : 0)) +
        space.between,
    );
    for (const [offset, choice] of shown.entries()) {
      const index = start + offset;
      const selected = index === this.selection;
      if (titled(index) || (offset === 0 && choice.heading))
        this.paletteList.add(
          this.inset(
            space.between,
            this.line([[choice.heading ?? "", c.muted, bold]], c.muted, {
              marginTop: offset ? space.section : 0,
            }),
          ),
        );
      const block = this.box({
        backgroundColor: selected ? c.selected : c.raised,
        marginTop: offset && this.rich ? gap : 0,
        onMouseUp: this.click(() => {
          this.selection = index;
          this.choose();
        }),
        onMouseOver: () => {
          if (this.selection === index) return;
          this.selection = index;
          this.renderChoices();
        },
      });
      if (this.rich) {
        // The bar of the selected choice runs down all its rows.
        const bar: Part = [`${selected ? glyph.mark : " "} `, c.accent];
        const width = inner - 2;
        const keys = choice.keys ?? "";
        const top = this.row();
        top.add(
          this.text([bar, [clip(choice.label, width - Bun.stringWidth(keys) - 2), c.text, bold]], c.text, {
            flexGrow: 1,
            flexShrink: 1,
            truncate: true,
          }),
        );
        if (keys) top.add(this.text(keys, selected ? c.muted : c.faint, { paddingRight: space.inset }));
        block.add(top);
        if (choice.command)
          block.add(this.line([bar, [clip(choice.command, width), selected ? c.accent : c.secondary]]));
        block.add(this.line([bar, [clip(choice.detail.replace(/\s*\n\s*/g, " "), width), c.muted]], c.muted));
        this.paletteList.add(block);
        continue;
      }
      block.flexDirection = "row";
      block.height = space.bar;
      const fg = choice.color ?? c.text;
      block.add(this.text(`${selected ? glyph.pointer : " "} `, c.accent, { attributes: bold }));
      const mark: Part | undefined = choice.mark ?? (choice.status ? statusMark(choice.status) : undefined);
      // A label or a detail that the row cut shows whole in a tip while the pointer is over it.
      const markText = mark ? mark[0] : "";
      block.add(
        this.whole(
          this.text(
            [
              ...(mark ? [mark] : []),
              // A label is cut at its end where the column of the details starts, a gap before it.
              [
                choice.detail
                  ? clip(
                      choice.label,
                      Math.max(4, column - space.between - (mark ? Bun.stringWidth(mark[0]) : 0)),
                    )
                  : choice.label,
                fg,
                selected ? bold : 0,
              ],
            ],
            fg,
            {
              width: choice.detail ? column : undefined,
              flexShrink: choice.detail ? 0 : 1,
              truncate: true,
            },
          ),
          () => `${markText}${choice.label}`,
        ),
      );
      if (choice.swatch)
        block.add(
          this.text(
            choice.swatch.map((hex): Part => [`${glyph.chip} `, RGBA.fromHex(hex)]),
            c.text,
            { width: choice.swatch.length * 2 + space.between },
          ),
        );
      if (choice.detail)
        block.add(
          this.whole(
            this.text(
              clip(
                choice.detail.replace(/\s*\n\s*/g, " "),
                Math.max(
                  8,
                  inner -
                    space.between -
                    column -
                    (choice.swatch ? choice.swatch.length * 2 + space.between : 0),
                ),
              ),
              selected ? c.text : c.muted,
              { truncate: true, flexShrink: 1 },
            ),
            () => choice.detail.replace(/\s*\n\s*/g, " "),
          ),
        );
      this.paletteList.add(block);
    }
    if (!this.filtered.length && this.paletteInput?.value)
      this.paletteList.add(this.inset(space.between, this.text("Nothing matches this filter.", c.muted)));
    else if (shown.length < this.filtered.length)
      this.paletteList.add(
        this.inset(
          space.between,
          this.text(`${this.selection + 1} of ${this.filtered.length}`, c.faint, {
            marginTop: space.section,
          }),
        ),
      );
  },

  choose(): void {
    const choice = this.filtered[this.selection];
    if (!choice) return;
    this.closeOverlay();
    void Promise.resolve()
      .then(() => choice.run())
      .catch(this.report);
  },

  closeOverlay(): void {
    this.authenticationOpen = false;
    if (this.menuItem) {
      this.menuItem = undefined;
      this.schedule();
    }
    this.overlay?.destroyRecursively();
    this.backdrop?.destroyRecursively();
    this.overlay = undefined;
    this.backdrop = undefined;
    this.paletteInput = undefined;
    this.paletteInputRow = undefined;
    this.paletteList = undefined;
    this.questionDocument = undefined;
    this.answering = undefined;
    this.composer?.focus();
  },

  /** The keys that this terminal sends, what each mark means, and the commands. */
  help(): void {
    const legend: [string, RGBA, string][] = [
      [`${spin(0)} Working`, c.accent, "A model or a command runs"],
      [`${glyph.running} Running`, c.accent, "A chain or a session has work in progress"],
      [`${glyph.asks} Input needed`, c.warning, "A question waits for your answer"],
      [`${glyph.notice} Notice`, c.faint, "A notice tells you something and asks nothing"],
      [`${glyph.held} Paused`, c.warning, "Work waits until you wake it"],
      [`${glyph.done} Done`, c.success, "The act ended and gave its value"],
      [
        `${glyph.dot} Finished, unread`,
        c.success,
        "A chain you started, or a session, ended while you were away",
      ],
      [`${glyph.failed} Failed`, c.danger, "The act raised, or the gate refused its Python"],
      [`${glyph.ring} Ready`, c.faint, "Nothing runs, or the work is saved or pending"],
    ];
    this.openPalette(
      "Keys and commands",
      keys
        .map(
          (key): Choice => ({
            label: chords(key, this.kitty),
            detail: key.action,
            heading: "Keys",
            run: () => {},
          }),
        )
        .concat(
          legend.map(([mark, color, meaning]) => ({
            label: mark,
            detail: meaning,
            color,
            heading: "Marks",
            run: () => {},
          })),
        )
        .concat(
          slashes.map(({ usage, detail }) => ({
            label: usage,
            detail,
            heading: "Slash commands",
            run: () => {},
          })),
        ),
      { note: "The chords that this terminal sends, what each mark means, and every command." },
    );
  },
});

export type PalettePart = typeof palette;
