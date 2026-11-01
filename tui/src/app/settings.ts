import { getTreeSitterClient, type Renderable, type RGBA } from "@opentui/core";
import { gist } from "furb/life";
import { openFile } from "../editor.ts";
import { shortenHomes } from "../files.ts";
import { clip, modelName } from "../format.ts";
import {
  theme as c,
  colorless,
  paletteOf,
  palettes,
  setTheme,
  syntax,
  type ThemeChoice,
  type ThemeName,
  themeLabels,
} from "../theme.ts";
import { part } from "./part.ts";
import { inputColors, terminalDetail } from "./shared.ts";

export const settings = part({
  /** The models of the roster and of the catalog under their providers, with the one the chain uses marked. */
  models(): void {
    this.pick("model", "Model", "The chain sends its next prompt to this model.");
  },

  codexModels(): void {
    this.pickModels("codex");
  },

  pickModels(provider?: string): void {
    void this.session
      .reloadCatalog()
      .then(() => {
        if (!this.closed)
          this.pick(
            "model",
            provider ? "Codex models" : "Model",
            provider
              ? "These models use your ChatGPT subscription."
              : "The chain sends its next prompt to this model.",
            provider ? (value) => value.startsWith(`${provider}:`) : undefined,
          );
      })
      .catch(this.report);
  },

  authentication(status: import("furb").CodexAuthInfo): void {
    if (status.state !== "waiting" || !status.url) {
      if (this.authenticationOpen) this.closeOverlay();
      return;
    }
    const url = status.url;
    this.openPalette(
      "Sign in to Codex",
      [
        {
          label: "Open browser",
          detail: "Finish sign-in with your ChatGPT subscription",
          run: () => openFile(url),
        },
        {
          label: "Copy sign-in link",
          detail: "Use this if the browser did not open",
          run: () => {
            this.renderer.copyToClipboardOSC52(url);
          },
        },
        {
          label: "Cancel sign-in",
          detail: "Keep using the current model",
          run: async () => {
            await this.session.host.codexAuth("cancel");
          },
        },
      ],
      { note: "Furb will connect when sign-in finishes. No API key is needed." },
    );
    this.authenticationOpen = true;
    if (this.authenticationUrl !== url) {
      this.authenticationUrl = url;
      void openFile(url).catch(() => {
        this.session.notice = "Open the sign-in link to connect Codex.";
        this.render();
      });
    }
  },

  providers(): void {
    void this.session.host
      .providers()
      .then((choices) => {
        const order = ["codex", "claude-cli", "openai", "local"];
        const rank = (name: string) => (order.includes(name) ? order.indexOf(name) : order.length);
        choices.sort((one, other) => rank(one.name) - rank(other.name) || one.name.localeCompare(other.name));
        this.openPalette(
          "Providers",
          [
            ...choices.map((one) => ({
              label: one.name,
              detail:
                one.name === "codex"
                  ? one.enabled && one.models > 0
                    ? `Choose from ${one.models} models on your ChatGPT subscription`
                    : "Connect your ChatGPT subscription"
                  : one.name === "openai"
                    ? `OpenAI API, ${one.models} models available`
                    : `${one.enabled ? "Remove" : "Add"} provider, ${one.models} models available`,
              mark: this.current(one.enabled),
              run: () =>
                one.name === "codex" && one.enabled && one.models > 0
                  ? this.codexModels()
                  : this.action(
                      `/provider ${one.enabled && !(one.name === "codex" && one.models === 0) ? "remove" : "add"} ${one.name}`,
                    ),
            })),
            ...choices
              .filter((one) => one.name === "codex" && one.enabled)
              .map(() => ({
                label: "Disable codex",
                detail: "Stop offering ChatGPT subscription models in this project",
                run: () => this.action("/provider remove codex"),
              })),
            {
              label: "Add a custom provider",
              detail: "Enter its name, URL, model and context window",
              run: () => this.insert("/provider add "),
            },
            {
              label: "Reload settings",
              detail: "Read the JSON config files again",
              run: () => this.action("/reload"),
            },
          ],
          { note: "Changes apply to this project and are saved in .furb/config.json." },
        );
      })
      .catch(this.report);
  },

  effortPicker(): void {
    this.pick(
      "effort",
      `Effort of ${modelName(this.session.actorChoice.model).id}`,
      "More effort thinks longer and costs more. ⇧Tab moves to the next.",
    );
  },

  details(): void {
    const w = this.session;
    // Each thread of the chain folds to its header and opens to its program, as a click on its header does.
    const threads = w.view === "feed" ? this.chainThreadList(w.selected) : [];
    this.openPalette("Details", [
      ...threads.map((one) => {
        const folded = this.threadFolded(one.act.id);
        return {
          label: clip(shortenHomes(gist(this.threadTitle(one))) || one.act.id, 72),
          detail: folded ? "Expand" : "Collapse",
          run: () => {
            this.foldThread(one.act.id, !folded);
            this.scrollAfterLayout({ card: one.act.id });
          },
        };
      }),
      ...[...this.cards]
        .filter(([, card]) => card.compact || card.collapsible)
        .map(([id, card]) => ({
          label: card.heading.plainText.replace(/^[▸▾] /, ""),
          detail: card.closed ? "Expand" : "Collapse",
          run: () => {
            w.folds[card.state] = !card.closed;
            this.renderContent();
            this.scrollAfterLayout({ card: id });
          },
        })),
    ]);
  },

  themes(): void {
    // The theme of the terminal comes first, then the dark themes, then the light ones.
    const names = (Object.keys(palettes) as (keyof typeof palettes)[]).sort(
      (one, other) => Number(themeLabels[one][1] === "Light") - Number(themeLabels[other][1] === "Light"),
    );
    const choices: ThemeChoice[] = ["terminal", ...names];
    const chosen = this.session.preferences.theme;
    this.openPalette(
      "Color theme",
      choices.map((name) => {
        const palette = paletteOf(name === "terminal" ? this.theme : name);
        return {
          label: name === "terminal" ? "Terminal" : themeLabels[name][0],
          detail: name === "terminal" ? terminalDetail : themeLabels[name][1],
          mark: this.current(name === chosen),
          swatch: [palette.accent, palette.secondary, palette.success, palette.warning, palette.danger],
          run: () => {
            this.session.theme = name;
            this.render();
            this.session.save();
          },
        };
      }),
      {
        selected: choices.indexOf(chosen),
        note: colorless.on
          ? "NO_COLOR is set, so every theme shows the colors of the terminal alone."
          : "Every session shares this choice.",
      },
    );
  },

  shapes(): void {
    this.pick("shape", "Response shape", "The engine validates the result against this Python type.");
  },

  toggleMode(): void {
    this.session.mode = this.session.mode === "python" ? "prompt" : "python";
    this.render();
    void this.highlightEditor();
  },

  applyTheme(name: ThemeName): void {
    const old = { ...c };
    setTheme(name);
    this.theme = name;
    const replacements = new Map(
      Object.keys(old).map((key) => [old[key as keyof typeof old], c[key as keyof typeof c]]),
    );
    const recolor = (node: Renderable) => {
      for (const property of [
        "fg",
        "bg",
        "backgroundColor",
        "borderColor",
        "titleColor",
        "textColor",
        "placeholderColor",
      ]) {
        const color: unknown = Reflect.get(node, property);
        if (replacements.has(color as RGBA)) Reflect.set(node, property, replacements.get(color as RGBA));
      }
      for (const child of node.getChildren()) recolor(child);
    };
    recolor(this.root);
    // An input keeps the colors that it shows focused, which the walk above cannot read.
    for (const input of [this.composer, this.search]) Object.assign(input, inputColors(c.panel));
    this.clear(this.scroll);
    this.cards.clear();
    const previous = this.style;
    this.style = syntax();
    this.composer.syntaxStyle = this.style;
    previous.destroy();
  },

  async highlightEditor(): Promise<void> {
    if (this.closed) return;
    const content = this.composer.plainText;
    this.keepDraft();
    const version = ++this.editorVersion;
    this.composer.clearAllHighlights();
    if (this.session.mode !== "python" && !this.session.editing && !content.startsWith("/run ")) return;
    const result = await getTreeSitterClient()
      .highlightOnce(content, "python")
      .catch((error) => {
        if (!this.closed) this.session.fail(error);
        return undefined;
      });
    if (version !== this.editorVersion || this.closed) return;
    // The composer counts the characters of its text without the line ends, so each offset of the text loses the
    // line ends before it.
    const ends: number[] = [];
    for (let at = content.indexOf("\n"); at >= 0; at = content.indexOf("\n", at + 1)) ends.push(at);
    const flat = (offset: number) => offset - ends.filter((end) => end < offset).length;
    const mark = (start: number, end: number, styleId: number) =>
      this.composer.addHighlightByCharRange({ start: flat(start), end: flat(end), styleId });
    for (const [start, end, group] of result?.highlights ?? []) {
      const styleId =
        this.style.resolveStyleId(group) ?? this.style.resolveStyleId(group.split(".")[0] ?? "default");
      if (styleId !== null) mark(start, end, styleId);
    }
    if (content === this.session.rejectedWord || content === `/run ${this.session.rejectedWord}`) {
      const styleId = this.style.resolveStyleId("diagnostic");
      const lines = content.split("\n");
      for (const finding of this.session.findings) {
        const line = Number(finding.match(/line (\d+)/)?.[1] ?? 0) - 1;
        if (styleId !== null && line >= 0) {
          const start = lines.slice(0, line).reduce((size, line) => size + line.length + 1, 0);
          mark(start, start + (lines[line]?.length ?? 0), styleId);
        }
      }
    }
    const cursor = this.beforeCursor().length;
    const at = "()[]{}".includes(content[cursor] ?? " ") ? cursor : cursor - 1;
    const bracket = content[at] ?? "";
    const pair = "()[]{}".indexOf(bracket);
    if (bracket && pair >= 0) {
      const direction = pair % 2 === 0 ? 1 : -1;
      const other = "()[]{}"[pair + direction];
      let depth = 0;
      for (let index = at; index >= 0 && index < content.length; index += direction) {
        if (
          (result?.highlights ?? []).some(
            ([start, end, group]) => /^(string|comment)/.test(group) && index >= start && index < end,
          )
        )
          continue;
        if (content[index] === bracket) depth++;
        else if (content[index] === other && --depth === 0) {
          const styleId = this.style.resolveStyleId("matching");
          if (styleId !== null) for (const start of [at, index]) mark(start, start + 1, styleId);
          break;
        }
      }
    }
  },
});

export type SettingsPart = typeof settings;

export const settingsBound = [
  "models",
  "codexModels",
  "authentication",
  "providers",
  "effortPicker",
  "details",
  "shapes",
] as const;
