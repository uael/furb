import { InputRenderable, InputRenderableEvents } from "@opentui/core";
import type { Action, Chosen, Field, Item } from "../parts.ts";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { bold } from "../ui.ts";
import { part } from "./part.ts";
import { type Choice, inputColors } from "./shared.ts";

/** A dot for each sign of a text. */
const dot = (text: string) => "•".repeat([...text].length);

export const dialogs = part({
  /** The dialog of a list that a part awaits, on the palette: Enter gives the selected item, the key of an action gives
   * the item and the key, and a close with no choice gives undefined. */
  partChoose<T>(
    title: string,
    items: readonly Item<T>[],
    {
      note = "",
      actions = [],
      selected = 0,
    }: { note?: string; actions?: readonly Action[]; selected?: number } = {},
  ): Promise<Chosen<T> | undefined> {
    return new Promise((resolve) => {
      const of = new Map<Choice, Item<T>>();
      const choices = items.map((item): Choice => {
        const choice: Choice = {
          label: String(item.label),
          detail: String(item.detail ?? ""),
          color: this.partTone(item.tone),
          lines: (item.lines ?? []).map((line) => this.partLine(line)),
          run: () => resolve({ value: item.value, action: "" }),
        };
        of.set(choice, item);
        return choice;
      });
      const keys = actions.map((one) => `${one.key} ${one.label}`).join(", ");
      this.openPalette(String(title), choices, {
        selected,
        note: [String(note), keys ? `Keys: ${keys}.` : ""].filter(Boolean),
        actions: actions.map((one) => ({
          key: String(one.key),
          label: String(one.label),
          run: (choice: Choice) => {
            const item = of.get(choice);
            resolve(item ? { value: item.value, action: String(one.key) } : undefined);
          },
        })),
      });
      this.overlayClosed = () => resolve(undefined);
    });
  },

  /** The dialog of a form that a part awaits: a field for each value, which hides a secret, and a close with no send
   * gives undefined. */
  partForm(
    title: string,
    fields: readonly Field[],
    { note = "" }: { note?: string } = {},
  ): Promise<Record<string, string> | undefined> {
    return new Promise((resolve) => {
      const moves =
        "Tab or Enter goes to the next field, ⇧Tab to the one before, and Enter on the last one sends it.";
      this.openPalette(String(title), [], { note: [String(note), moves].filter(Boolean) });
      // The filter of the palette has no use in a form, which takes the keys in its own fields.
      if (this.paletteInputRow) {
        this.paletteInputRow.visible = false;
        this.paletteInputRow.height = 0;
      }
      const overlay = this.overlay;
      const list = this.paletteList;
      if (!overlay || !list) return resolve(undefined);
      const inputs: InputRenderable[] = [];
      // The signs of each secret field, by its place, which its field shows as dots.
      const secrets = new Map<number, string>();
      const error = this.text("", c.danger);
      let at = 0;
      const go = (index: number) => {
        at = (index + inputs.length) % Math.max(1, inputs.length);
        inputs[at]?.focus();
      };
      const send = () => {
        const values = Object.fromEntries(
          fields.map((field, index) => [field.name, secrets.get(index) ?? inputs[index]?.value ?? ""]),
        );
        const missing = fields.findIndex((field) => field.required && !values[field.name]?.trim());
        if (missing >= 0) {
          error.content = `${fields[missing]?.label} is needed.`;
          go(missing);
          return;
        }
        this.overlayClosed = undefined;
        this.closeOverlay();
        resolve(values);
      };
      for (const [index, field] of fields.entries()) {
        const box = this.box({ marginBottom: space.section });
        box.add(
          this.inset(
            space.between,
            this.line([
              [String(field.label), c.text, bold],
              [field.required ? "  needed" : "", c.faint],
            ]),
          ),
        );
        if (field.detail) box.add(this.inset(space.between, this.text(String(field.detail), c.muted)));
        const row = this.row({ paddingLeft: space.between, height: space.bar });
        row.add(this.text(`${glyph.prompt} `, c.accent));
        const holder = this.box({ flexGrow: 1, height: space.bar });
        const start = String(field.value ?? "");
        const input = new InputRenderable(this.renderer, {
          id: `part-form-${index}`,
          width: "100%",
          value: field.secret ? dot(start) : start,
          ...inputColors(c.raised),
        });
        holder.add(input);
        if (field.secret) {
          // A secret field shows a dot for each sign and keeps its signs aside: what changed between the dots and the
          // text of the field is the change of the secret.
          secrets.set(index, start);
          let mending = false;
          input.on(InputRenderableEvents.INPUT, (value: string) => {
            if (mending) return;
            const was = [...(secrets.get(index) ?? "")];
            const now = [...value];
            let head = 0;
            while (head < Math.min(was.length, now.length) && now[head] === "•") head++;
            let tail = 0;
            while (tail < Math.min(was.length, now.length) - head && now[now.length - 1 - tail] === "•")
              tail++;
            const kept = [
              ...was.slice(0, head),
              ...now.slice(head, now.length - tail),
              ...was.slice(was.length - tail),
            ];
            secrets.set(index, kept.join(""));
            mending = true;
            input.value = dot(kept.join(""));
            mending = false;
          });
        }
        input.on(InputRenderableEvents.ENTER, () => (index === fields.length - 1 ? send() : go(index + 1)));
        input.on(InputRenderableEvents.INPUT, () => {
          error.content = "";
        });
        row.add(holder);
        box.add(row);
        list.add(box);
        inputs.push(input);
      }
      list.add(this.inset(space.between, error));
      this.paletteKeysOff = this.keymap.registerLayer({
        priority: 100,
        enabled: () => this.overlay === overlay,
        commands: [
          {
            name: "part-form-next",
            run: () => {
              go(at + 1);
              return true;
            },
          },
          {
            name: "part-form-back",
            run: () => {
              go(at - 1);
              return true;
            },
          },
        ],
        bindings: [
          { key: "tab", cmd: "part-form-next" },
          { key: "shift+tab", cmd: "part-form-back" },
        ],
      });
      this.overlayClosed = () => resolve(undefined);
      if (inputs.length) go(0);
      else {
        list.add(this.inset(space.between, this.text("Enter sends this form, which has no field.", c.muted)));
        this.paletteInput?.on(InputRenderableEvents.ENTER, send);
      }
    });
  },

  /** The dialog of a question of yes or no that a part awaits, with its text: a close with no choice gives false. */
  partConfirm(
    title: string,
    text: string,
    { yes = "Yes", no = "No" }: { yes?: string; no?: string } = {},
  ): Promise<boolean> {
    return new Promise((resolve) => {
      this.openPalette(String(title), [
        { label: String(yes), detail: "", run: () => resolve(true) },
        { label: String(no), detail: "", run: () => resolve(false) },
      ]);
      this.showQuestionText(String(text));
      this.overlayClosed = () => resolve(false);
    });
  },
});

export type DialogsPart = typeof dialogs;
