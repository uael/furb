import { imageContent, imagePath, WORK } from "furb";
import { asksOperator } from "furb/life";
import { clipboardImage } from "../clipboard.ts";
import { externalEditor, openFile } from "../editor.ts";
import { shortenHome } from "../files.ts";
import { kibibytes } from "../format.ts";
import { type Action, bindings, shown } from "../keys.ts";
import { isLifeView, lifeViewDetails, lifeViewLabels, lifeViews } from "../overview.ts";
import type { ActRow } from "../session.ts";
import { publishShare } from "../share.ts";
import { theme as c, glyph } from "../theme.ts";
import type { Part } from "../ui.ts";
import { part } from "./part.ts";
import { title } from "./shared.ts";

export const actions = part({
  /** The commands that the view answers itself, by their text. */
  async globalCommand(text: string): Promise<boolean> {
    const [name, ...words] = text.startsWith("/") ? text.slice(1).split(" ") : [];
    const argument = words.join(" ").trim();
    const library = this.options.workspaces;
    // A command with no argument that opens a picker or acts at once, and /inspect, which opens the value it names.
    const bare: Record<string, () => unknown> = {
      details: this.details,
      rewind: this.rewind,
      tree: this.sessionTree,
      queue: this.queuePicker,
      model: this.models,
      providers: this.providers,
      effort: this.effortPicker,
      shape: this.shapes,
      theme: () => this.themes(),
      exit: () => this.options.quit(),
      end: () => this.options.quit(true),
      reopen: () => this.session.reopen(),
      stop: () => this.session.stopHost(),
      editor: () => this.editDraft(),
      files: this.filesPicker,
      new: () => library.create(),
      sidebar: () => library.toggle(),
      workspace: this.workspacePicker,
      watch: this.watch,
      repair: () => this.session.repairJournal(),
      renew: () => this.session.renewJournal(),
    };
    if (name && !argument && Object.hasOwn(bare, name)) {
      await bare[name]?.();
      return true;
    }
    if (name === "sidebar" && argument) {
      this.setRailStyle(argument);
      return true;
    }
    if (name === "serve") {
      const served = await this.session.host.serve(argument);
      const copy = (address: string, what: string) => () => {
        this.renderer.copyToClipboardOSC52(address);
        this.session.notice = `Copied the address ${what}.`;
      };
      this.openPalette(
        "Serve this life to other operators",
        [
          {
            label: "Copy the address to read and drive it",
            detail: served.write,
            run: copy(served.write, "to read and drive this life"),
          },
          {
            label: "Copy the address to read it only",
            detail: served.read,
            run: copy(served.read, "to read this life only"),
          },
        ],
        {
          note: "An operator attaches with furb-tui --attach <address>. Each address holds its key, so give it only to the operator that it is for.",
        },
      );
      return true;
    }
    if (name === "access") {
      if (!argument) this.accessPicker();
      else if (argument === "reader" || argument === "motion") this.toggleAccess(argument);
      else throw new Error("Choose reader or motion.");
      return true;
    }
    if (name === "view") {
      if (!argument) this.viewPicker();
      else if (isLifeView(argument)) this.switchView(argument);
      else throw new Error(`Choose ${lifeViews.join(", ")}.`);
      return true;
    }
    if (name === "inspect" && argument) {
      await this.inspect(argument);
      return true;
    }
    if (text === "/image" || text.startsWith("/image ")) {
      const path = text.slice(6).trim();
      if (path) await this.session.attachImage(path);
      else await clipboardImage((path) => this.session.attachImage(path));
      return true;
    }
    if (text === "/delete") {
      this.openPalette(
        "Delete a session",
        library.groups.flatMap((group) =>
          group.sessions.map((entry) => ({
            label: entry.name,
            detail: group.name,
            run: () => this.removeItem(entry),
          })),
        ),
      );
      return true;
    }
    if (text.startsWith("/workspace ")) {
      const group = await library.add(text.slice(11).trim());
      const entry = group.sessions[0];
      if (entry) await library.select(entry);
      else await library.create(group);
      return true;
    }
    if (name === "extensions" && !argument) {
      await this.extensionsDialog();
      return true;
    }
    // A command of a part of an extension, which no command of the TUI shadows.
    const part = name ? this.parts.command(name, this.seen()) : undefined;
    if (part) {
      this.partRun(() => part.run(argument, this.seen()));
      return true;
    }
    return false;
  },

  /** The views of the life to choose from, the one that shows marked. */
  viewPicker(): void {
    this.openPalette(
      "View of the life",
      lifeViews.map((view) => ({
        label: `${lifeViewLabels[view]}${view === this.lifeView ? "  (shows)" : ""}`,
        detail: lifeViewDetails[view],
        run: () => this.switchView(view),
      })),
    );
  },

  imageActions(uri: string): void {
    const content = imageContent(this.session.host.imageDirectory, uri);
    const file = imagePath(this.session.host.imageDirectory, uri).path;
    const pending = this.session.images[this.session.selected]?.find((image) => image.uri === uri);
    this.openPalette(pending?.name ?? "Image attachment", [
      {
        label: "Open image",
        detail: `${content.mimeType}  ${kibibytes(Buffer.byteLength(content.data, "base64"))}`,
        run: () => openFile(file),
      },
      ...(pending
        ? [
            {
              label: "Remove from this draft",
              detail: "Keep the saved image file",
              run: () => {
                this.session.images[this.session.selected] = (
                  this.session.images[this.session.selected] ?? []
                ).filter((image) => image.uri !== uri);
                this.session.save();
                this.render();
              },
            },
          ]
        : []),
    ]);
  },

  async editDraft(): Promise<void> {
    const value = await externalEditor(
      this.renderer,
      this.composer.plainText,
      this.writesPython,
      this.session.directory || this.session.host.directory,
    );
    // An editor ends a file with a line end, which the draft leaves out.
    if (!this.closed) {
      this.insert(value.replace(/\n$/, ""));
      this.session.notice = "The draft is back from the editor.";
      this.render();
    }
  },

  async filesPicker(): Promise<void> {
    const files = await this.session.projectFiles(true);
    if (this.closed) return;
    this.openPalette(
      "Attach a project file",
      files.map((path) => ({
        label: path,
        detail: "Read into this chain with the next message",
        run: () => {
          const token = this.beforeCursor().match(/(?:^|\s)(@[^\s]*)$/)?.[1] ?? "";
          this.replaceBefore(token, `@${/\s/.test(path) ? JSON.stringify(path) : path} `);
        },
      })),
    );
  },

  queuePicker(): void {
    const session = this.session;
    this.openPalette(
      "Queued follow-ups",
      [
        ...(session.queueHeld
          ? [
              {
                label: "Resume queue",
                detail: session.queueError || "Send when this chain's current work is complete",
                run: async () => {
                  session.queueHeld = false;
                  session.queueError = "";
                  await session.drainQueue();
                },
              },
            ]
          : []),
        ...session.queued.map((entry) => ({
          label: entry.text.split("\n")[0] ?? "Follow-up",
          // A follow-up asks the actor of its chain when it is sent, and the session reads that actor for the
          // selected chain alone.
          detail: `${entry.chain === session.selected ? `to ${this.model("").name} ` : ""}on ${session.labelOf(entry.chain)}`,
          mark: [`${glyph.ring} `, c.faint] as Part,
          run: () =>
            this.openPalette("Queued message", [
              {
                label: "Edit in composer",
                detail: "Remove from the queue and edit before sending again",
                run: async () => {
                  session.removeQueued(entry.id);
                  await session.select(entry.chain);
                  this.insert(entry.text);
                },
              },
              {
                label: "Remove",
                detail: "Remove this queued message",
                run: () => {
                  session.removeQueued(entry.id);
                },
              },
            ]),
        })),
      ],
      { note: "Sent in turn after the current work. Choose one to edit or remove it." },
    );
  },

  shared(path: string, markdown: string): void {
    this.openPalette(
      "Conversation ready to share",
      [
        {
          label: "Open the page",
          detail: "Show the conversation in the browser",
          run: () => openFile(path),
        },
        {
          label: "Copy the path",
          detail: "Put the path of the page on the clipboard",
          run: () => {
            this.renderer.copyToClipboardOSC52(path);
            this.session.notice = "Export path copied.";
          },
        },
        {
          label: "Upload a secret gist",
          detail: "Anyone with the link can read it. Needs the GitHub CLI, signed in.",
          run: async () => {
            const url = await publishShare(path, markdown, this.session.sessionName);
            this.session.notice = `Shared: ${url}`;
            if (!this.closed)
              this.openPalette("Share link", [
                {
                  label: "Copy link",
                  detail: url,
                  run: () => {
                    this.renderer.copyToClipboardOSC52(url);
                    this.session.notice = "Share link copied.";
                  },
                },
              ]);
          },
        },
      ],
      { note: `The page is ${shortenHome(path)}.` },
    );
  },

  action(command: string): void {
    this.closeOverlay();
    void this.globalCommand(command)
      .then((handled) => {
        if (!handled) return this.session.submit(command);
      })
      .catch(this.report);
  },

  report(error: unknown): void {
    const message = error instanceof Error ? error.message : String(error);
    if (this.closed) {
      const session = this.options.workspaces.current?.session;
      if (session) session.notice = message;
    } else this.showValue("Could not complete action", message);
  },

  /** What the operator can do with an act that a right click chose. */
  actActions(act: ActRow): void {
    const w = this.session;
    const message = w.isUserPrompt(act) && !asksOperator(act);
    this.openPalette(`${title(act.kind)} ${act.id}`, [
      {
        label: "Inspect",
        detail: "Its words, its state, and its value",
        run: () =>
          this.showValue(act.id, {
            kind: act.kind,
            by: act.by,
            words: act.words,
            done: act.done,
            value: act.value,
          }),
      },
      ...(!w.host.reading && w.editable(act)
        ? [
            {
              label: "Edit its program",
              detail: "Change the Python the model wrote for it, and replay it",
              run: () => this.action(`/edit ${act.id}`),
            },
          ]
        : []),
      ...(!w.host.reading && !act.done && !act.unfinished && WORK.includes(act.kind)
        ? [
            w.lastOwn?.id === act.id && message
              ? {
                  label: "Take back",
                  detail: "Cancel this message alone, and give it back to the input to write again",
                  run: () => this.action("/cancel"),
                }
              : {
                  label: "Cancel",
                  detail: "End this act and the work under it, and not the thread or the chain it stands on",
                  run: () => this.action(`/cancel ${act.id}`),
                },
          ]
        : []),
      ...(!w.host.reading && this.isPoint(act)
        ? [
            {
              label: message ? "Edit and send again" : "Branch after it",
              detail: message
                ? "A new branch that has not read this message, with the message in the input"
                : "A new branch that reads the chain up to this act",
              run: () => this.rewindTo(act),
            },
          ]
        : []),
      {
        label: "Copy its name",
        detail: act.id,
        run: () => {
          this.renderer.copyToClipboardOSC52(act.id);
          w.notice = `${act.id} copied.`;
        },
      },
    ]);
  },

  /** The chord of the key that runs an action, as this terminal sends it. */
  chordOf(action: Action): string {
    const found = bindings.find(({ binding }) => binding.run === action);
    return found ? shown(found.key, found.binding, this.kitty) : "";
  },
});

export type ActionsPart = typeof actions;

export const actionsBound = ["filesPicker", "queuePicker", "shared", "report"] as const;
