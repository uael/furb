import { display } from "furb";
import { shortenHome } from "../files.ts";
import { theme as c, glyph } from "../theme.ts";
import { bold } from "../ui.ts";
import { part } from "./part.ts";
import type { Choice } from "./shared.ts";

export const values = part({
  async showHover(name: string, x: number, y: number): Promise<void> {
    try {
      const inspected = await this.session.engine.inspect(name, this.session.selected);
      if (this.closed || this.overlay) return;
      this.hoverCard(
        x,
        y,
        9,
        () => this.inspect(name),
        this.text([
          [name, c.text, bold],
          [`: ${inspected.kind}`, c.secondary],
        ]),
        this.text(inspected.representation.slice(0, 280), c.muted, { maxHeight: 4 }),
        this.text([
          ["Click", c.muted],
          [" opens it   ", c.faint],
          ["⌃G", c.muted],
          [" inspects a name", c.faint],
        ]),
      );
    } catch {
      this.unhover();
    }
  },

  /** The value a name holds in the module of the chain, shown once the life has read it. */
  inspect(name: string): Promise<void> {
    this.unhover();
    return this.session.engine
      .inspect(name, this.session.selected)
      .then((value) => {
        this.showValue(`${name}: ${value.kind}`, value.value ?? value.representation, name);
      })
      .catch(this.report);
  },

  showValue(label: string, value: unknown, name?: string, back?: () => void): void {
    const choices: Choice[] = [];
    if (back) choices.push({ label: "← Back", detail: "Return to the parent value", run: back });
    if (name && /^[\p{L}_][\p{L}\p{N}_]*$/u.test(name)) {
      const definition = [...Object.entries(this.session.program)]
        .reverse()
        .find(([, source]) =>
          source
            .split("\n")
            .some((line) => new RegExp(`^(?:def |class )?${name}\\b(?:\\s*[:=(])`).test(line)),
        );
      if (definition)
        choices.push({
          label: "Go to definition",
          detail: definition[0],
          run: () => {
            this.go("feed", definition[0]);
          },
        });
      else
        choices.push({
          label: "Engine definition",
          detail: `Find ${name} in the engine source`,
          run: async () => {
            const lines = (await this.session.host.source()).split("\n");
            const at = lines.findIndex((line) =>
              new RegExp(`^(?:(?:async )?def |class )?${name}\\b`).test(line),
            );
            if (at < 0) {
              this.session.notice = "This name has no definition in the engine source.";
              return;
            }
            const next = lines.slice(at + 1).findIndex((line) => /^(?:def |class |[A-Z_]+\s*=)/.test(line));
            this.openPalette(`${name} in engine.py, line ${at + 1}`, [
              {
                label: "← Back to value",
                detail: label,
                run: () => this.showValue(label, value, name, back),
              },
            ]);
            this.paletteList?.add(
              this.document(
                this.numbered(lines.slice(at, next < 0 ? undefined : at + next + 1).join("\n"), {
                  fg: c.muted,
                  lineNumberOffset: at,
                }),
                { height: Math.min(24, this.renderer.height - 16) },
              ),
            );
          },
        });
    }
    if (value && typeof value === "object") {
      for (const [key, child] of Object.entries(value))
        choices.push({
          label: `${key}  ${typeof child === "object" && child !== null ? glyph.closed : ""}`,
          detail: shortenHome(display(child)).replaceAll("\n", " ").slice(0, 110),
          run: () =>
            this.showValue(`${label}.${key}`, child, undefined, () =>
              this.showValue(label, value, name, back),
            ),
        });
    } else
      choices.push({
        label:
          typeof value === "string" && (value.length > 100 || value.includes("\n"))
            ? "Copy value"
            : display(value),
        detail: "Copy this value",
        run: () => {
          this.renderer.copyToClipboardOSC52(display(value));
          this.session.notice = "Value copied.";
        },
      });
    const act = typeof value === "string" ? this.session.actOf(value) : undefined;
    if (typeof value === "string" && act)
      choices.push({
        label: "Follow this act or door",
        detail: value,
        run: () => {
          if (act.kind === "chain") return this.session.select(act.id);
          void this.referenced(value)
            .then((held) => this.showValue(value, held))
            .catch(this.report);
        },
      });
    this.openPalette(label, choices);
    if (typeof value === "string" && (value.length > 100 || value.includes("\n"))) {
      this.questionDocument = this.document(this.text(value), {
        height: Math.max(3, Math.min(18, this.renderer.height - 18)),
      });
      this.paletteList?.add(this.questionDocument);
    }
  },

  async names(): Promise<void> {
    const w = this.session;
    const names = (await w.engine.names(w.selected)).filter((name) => !name.startsWith("_"));
    // The names that the words of this chain bind come first, then the acts of the chain, then what the engine gives.
    const identifier = "[\\p{L}_][\\p{L}\\p{N}_]*";
    // A name is bound by an assignment, a def or a class, a for loop, an import, or an as.
    const binds = [
      `^\\s*(${identifier})\\s*(?::[^=\\n]+)?=(?!=)`,
      `^\\s*(?:async\\s+)?(?:def|class)\\s+(${identifier})`,
      `^\\s*(?:async\\s+)?for\\s+(${identifier})\\s+in\\b`,
      `^\\s*(?:from\\s+\\S+\\s+)?import\\s+(${identifier})`,
      `\\bas\\s+(${identifier})`,
    ];
    const program = Object.values(w.program).join("\n");
    const bound = new Set(
      binds.flatMap((pattern) =>
        [...program.matchAll(new RegExp(pattern, "gmu"))].map((match) => match[1] ?? ""),
      ),
    );
    const acts = new Set(w.acts.map((act) => act.id));
    const rank = (key: string) => (bound.has(key) ? 0 : acts.has(key) ? 1 : 2);
    const headings = ["Named in this chain", "Acts of this chain", "Given by the engine"];
    this.openPalette(
      "Inspect a name",
      names
        .map((key, index) => ({ key, index, group: rank(key) }))
        .sort((one, other) => one.group - other.group || one.index - other.index)
        .map(({ key, group }) => ({
          label: key,
          detail: "",
          heading: headings[group],
          run: () => this.inspect(key),
        })),
      { note: "Enter reads the live value of the name, its type, and its fields." },
    );
  },

  async completeNames(): Promise<void> {
    const names = await this.session.engine.names(this.session.selected);
    const prefix = this.beforeCursor().match(/[\p{L}_][\p{L}\p{N}_]*$/u)?.[0] ?? "";
    this.openPalette(
      "Complete Python name",
      names
        .filter((name) => name.startsWith(prefix) && !name.startsWith("_"))
        .map((name) => ({
          label: name,
          detail: "Insert this name at the cursor",
          run: () => {
            this.replaceBefore(prefix, name);
            this.composer.focus();
          },
        })),
    );
  },

  /** The prompts of the chain, whose program an edit changes and replays. */
  ladders(): void {
    const prompts = this.session.activity.filter((act) => this.session.editable(act));
    this.openPalette(
      "Edit a prompt program",
      prompts.map((act) => {
        const { mark, color } = this.actState(act);
        return {
          label: act.id,
          detail: String(act.words[1]),
          mark: [`${mark} `, color],
          run: () => this.action(`/edit ${act.id}`),
        };
      }),
      {
        selected: Math.max(0, prompts.length - 1),
        note: "The program is the Python the model wrote for the prompt. An edit replays it.",
      },
    );
  },
});

export type ValuesPart = typeof values;

export const valuesBound = ["inspect"] as const;
