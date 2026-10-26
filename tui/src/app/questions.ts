import { InputRenderableEvents, type KeyEvent } from "@opentui/core";
import { type Ask, beside } from "furb/life";
import { elapsed } from "../format.ts";
import { theme as c, glyph, spacing as space } from "../theme.ts";
import { bold } from "../ui.ts";
import { part } from "./part.ts";

export const questions = part({
  /** The question whose values the input offers as a list in place of the box of the prompt: the question aimed, when
   * its shape is a choice, a Literal or a bool, and no program is under edit. */
  picking(): Ask | undefined {
    const ask = this.session.operatorPrompt;
    if (!ask?.choices.length || this.session.editing) return undefined;
    if (this.choice.id !== ask.id) this.choice = { id: ask.id, at: 0, other: false };
    return ask;
  },

  /** The values of a question of a choice, each after its number, and a last row, Other, whose words go in place of
   * every value. The row of the pointer is lit. While Other is chosen, the input under the list takes the words. */
  renderPick(ask: Ask | undefined): void {
    if (this.pickBox.visible !== Boolean(ask)) this.pickBox.visible = Boolean(ask);
    const typing = !ask || this.choice.other;
    const changed = this.composer.visible !== typing;
    if (changed) this.composer.visible = typing;
    // A record open to read has no input. Other inputs, such as a row being renamed, keep their focus.
    if (this.session.host.reading) this.composer.blur();
    else if (changed) {
      if (typing && !this.overlay) this.composer.focus();
      else this.composer.blur();
    }
    if (!ask) {
      this.pickKey = "";
      return;
    }
    const key = JSON.stringify([ask.id, ask.choices, this.choice, this.theme]);
    if (key === this.pickKey) return;
    this.pickKey = key;
    this.clear(this.pickBox);
    for (const [index, label] of [...ask.choices, "Other"].entries()) {
      const here = index === this.choice.at;
      this.pickBox.add(
        this.line(
          [
            [here ? glyph.pointer : " ", c.warning],
            [` ${index + 1}  `, here ? c.muted : c.faint],
            [label, here ? c.text : c.muted, here ? bold : 0],
            [index === ask.choices.length ? "  in your own words" : "", c.faint],
          ],
          c.text,
          { truncate: true, run: () => void this.takePick(index) },
        ),
      );
    }
  },

  /** The pointer of the list moves a step, round from the last row to the first. */
  stepPick(by: number): void {
    const ask = this.picking();
    if (!ask) return;
    const rows = ask.choices.length + 1;
    this.choice = { ...this.choice, at: (this.choice.at + by + rows) % rows };
    this.render();
  },

  /** The row at a place, the pointer by default, answers: a value answers the question with it, and Other gives the
   * input to the words of the operator. */
  async takePick(place?: number): Promise<void> {
    const at = place ?? this.choice.at;
    const ask = this.picking();
    if (!ask) return;
    const value = ask.choices[at];
    this.choice = { id: ask.id, at, other: value === undefined };
    if (value !== undefined) await this.session.answer(ask, value).catch(this.report);
    this.render();
  },

  /** The time since an act started. */
  progress(id: string): string {
    return elapsed(Date.now() - (this.session.started[id] ?? Date.now()));
  },

  resume(): void {
    const pending = this.session.host.pending;
    this.openPalette("Saved work is paused", [
      {
        label: "Resume saved work",
        detail: `${pending.size} unfinished ${pending.size === 1 ? "act starts" : "acts start"} again.`,
        run: async () => {
          await this.session.host.resume();
          await this.session.refresh();
        },
      },
      {
        label: "Keep it paused",
        detail: "Inspect the session first. /wake brings this choice back.",
        run: () => {},
      },
    ]);
  },

  /** The dialog of an ask: its whole message, and the field of its answer or the choices of its shape. Its title says
   * which ask of the desk it is, and from which chain. Tab and ⇧Tab go to the next and the previous ask, and an
   * answer goes on to the next ask that waits, until none waits. */
  question(chosen?: Ask): void {
    const ask = chosen ?? this.session.aimed ?? this.session.desk.asks[0];
    const w = this.session;
    if (!ask) return;
    if (w.pointed !== ask.id || ask.chain !== w.selected) void w.point(ask.id).catch(this.report);
    const { asks } = w.desk;
    const title = `Question ${asks.findIndex((one) => one.id === ask.id) + 1} of ${asks.length} from ${ask.label}`;
    const moves = asks.length > 1 ? " Tab goes to the next one." : "";
    // An answer goes on to the ask after this one, or closes the dialog when no other ask waits.
    const next = () => {
      const rest = w.desk.asks;
      const after = rest.find((one) => one.began > ask.began) ?? rest[0];
      if (after) this.question(after);
      else this.closeOverlay();
    };
    if (ask.choices.length) {
      this.openPalette(
        title,
        ask.choices.map((choice) => ({
          label: choice,
          detail: "",
          run: () => w.answer(ask, choice).then(next),
        })),
      );
      this.showQuestionText(ask.message);
      this.answering = ask.id;
      return;
    }
    this.openPalette(title, []);
    // The field of the dialog takes the answer, and shows its prompt from the start.
    const prompt = this.paletteInputRow?.getChildren()[0];
    if (prompt) prompt.visible = true;
    if (this.paletteInputRow) this.paletteInputRow.height = space.bar;
    this.showQuestionText(ask.message);
    if (this.paletteInput) this.paletteInput.placeholder = `Type the answer to ${ask.id}, as ${ask.shape}`;
    const error = this.text(`Enter sends it. Esc leaves the question open.${moves}`, c.muted);
    this.paletteList?.add(error);
    this.answering = ask.id;
    const input = this.paletteInput;
    if (!input) return;
    input.removeAllListeners(InputRenderableEvents.INPUT);
    input.removeAllListeners(InputRenderableEvents.ENTER);
    input.on(InputRenderableEvents.ENTER, () => {
      void w
        .answer(ask, input.value)
        .then(next)
        .catch((failure) => {
          error.fg = c.danger;
          error.content = String(failure);
        });
    });
  },

  /** The dialog of the ask a step after or before the one that it shows. */
  moveAsk(key?: KeyEvent) {
    const next = beside(this.session.desk.asks, this.answering, key?.shift ? -1 : 1);
    if (next && next.id !== this.answering) this.question(next);
  },

  showQuestionText(message: string): void {
    if (!this.overlay || !this.paletteInput) return;
    // The text of the question takes the rows it wraps to, up to what the screen leaves, and scrolls past them. A
    // long question moves the dialog up to the top of the screen, and a short one leaves it where every dialog stands.
    const inner = this.paletteWidth - space.inset * 2 - space.between * 2;
    const rows = message
      .split("\n")
      .reduce((sum, line) => sum + Math.max(1, Math.ceil(Bun.stringWidth(line) / inner)), 0);
    if (rows > 3) this.overlay.top = 1;
    this.questionDocument = this.document(this.markdown(message), {
      height: Math.max(1, Math.min(rows, 8, this.renderer.height - 18)),
      // A field that shows stands a row under the text, and a field that takes no row leaves its own space under it.
      marginBottom: this.paletteInputRow?.height ? space.section : 0,
      contentOptions: { paddingLeft: space.between, paddingRight: space.between },
    });
    this.overlay.insertBefore(this.questionDocument, this.paletteInputRow);
    this.overlay.maxHeight = this.paletteHeight;
  },
});

export type QuestionsPart = typeof questions;

export const questionsBound = ["resume", "moveAsk"] as const;
