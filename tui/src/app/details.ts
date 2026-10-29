import { isAbsolute, relative } from "node:path";
import {
  type BoxRenderable,
  type LineNumberOptions,
  LineNumberRenderable,
  type MouseEvent,
  parseColor,
  type Renderable,
  type RGBA,
  type ScrollBoxOptions,
  ScrollBoxRenderable,
  type SimpleHighlight,
  StyledText,
  type TextChunk,
  TextRenderable,
} from "@opentui/core";
import { actorParts, display, safeText, TIMEOUT, unmarked } from "furb";
import {
  cancelled,
  type Exit,
  FOLD,
  type Fold,
  failed,
  foldStrings,
  literals,
  type ProgramLine,
  tailOf,
  wrap,
  writing,
} from "furb/life";
import { clock } from "../clock.ts";
import { conversation } from "../conversation.ts";
import { shortenHomes } from "../files.ts";
import { clip, dollars, modelName, share } from "../format.ts";
import type { ActRow } from "../session.ts";
import { theme as c, frame, glyph, motion, spacing as space, spin } from "../theme.ts";
import { bold, italic, type Part, styled, underline } from "../ui.ts";
import { part } from "./part.ts";
import { gutterWidth, saidOf, seconds, stateOf } from "./shared.ts";

/** A reason of the gate or of a raise as one line says it: without the line that the row already shows, and
 * without the place that the parser names, `(<string>, line 2), in 'x = = 1'`, which repeats the line. */
const plainReason = (why: string) =>
  why.replace(/^line \d+: /, "").replace(/\s*\(<string>, line \d+\)(?:, in '.*'$)?/, "");

export const details = part({
  /** The rung that took the place of a rung of a model that failed: a later rung for the same prompt, which the model
   * wrote once it read the failure, so the failure no longer stands. */
  retry(act: ActRow): ActRow | undefined {
    if (act.kind !== "rung" || act.by === "operator" || !failed(act)) return undefined;
    return this.session.madeAfter(act, (other) => other.kind === "rung");
  },

  toldWord(id: string): string | undefined {
    const w = this.session;
    const { turns } = w;
    const held = this.told;
    if (held.turns !== turns) {
      // The turns before the last are whole, so each is read once while the turns begin with them. The last turn grows
      // as paragraphs are told on the chain, and it is read again.
      if (!(held.at > 0 && turns.length >= held.at && turns[held.at - 1] === held.turns[held.at - 1]))
        Object.assign(held, { at: 0, rung: undefined, words: new Map() });
      const whole = Math.max(held.at, turns.length - 1);
      const read = (from: { at: number; rung?: ActRow }, to?: number) => {
        for (const item of conversation(turns.slice(from.at, to), (one) => w.actOf(one), from))
          if (item.type === "python" && item.rung) held.words.set(item.rung.id, item.code);
        return from;
      };
      held.rung = read({ at: held.at, rung: held.rung }, whole).rung;
      held.at = whole;
      read({ at: whole, rung: held.rung });
      held.turns = turns;
    }
    return held.words.get(id);
  },

  /** What an act is about, whole: the message of a prompt, the program of a rung, the time of a wait, the ceilings of
   * a grant, and the first word of any other act. */
  subject(act: ActRow): string {
    const label = this.partRow(act)?.label;
    if (label !== undefined) return label;
    if (act.kind === "prompt") return String(act.words[1] ?? "");
    if (act.kind === "rung")
      return String(this.session.program[act.id] || act.word || this.toldWord(act.id) || act.words[0] || "");
    if (act.kind === "wait") return seconds(Number(unmarked(act.words[0])));
    if (act.kind === "write") {
      // A write names the path of the Text that it writes, which a read gave whole, so the path drops the directory of
      // the chain.
      const path = String((act.words[0] as { path?: unknown } | null)?.path ?? "");
      const inside = isAbsolute(path) ? relative(this.session.directory, path) : path;
      return inside && !inside.startsWith("..") && !isAbsolute(inside) ? inside : shortenHomes(path);
    }
    if (act.kind === "grant")
      return [
        act.words[0] === null ? "" : `${dollars(Number(unmarked(act.words[0])))} ceiling`,
        act.words[1] === null ? "" : share(Number(unmarked(act.words[1]))),
      ]
        .filter(Boolean)
        .join("  ");
    return String(act.words[0] || "");
  },

  /** Where a message of the operator stands, which its panel says at its right: the type of the answer that it asks
   * for, and a mark and a word only when a cancel or a failure ended it, or a pause holds it. */
  promptState(act: ActRow): Part[] {
    const shape: Part = [String(act.words[0] ?? ""), c.faint];
    if (act.unfinished && !act.done)
      return [shape, [`   ${glyph.ring} not finished when the record ends`, c.faint]];
    const state = act.done
      ? failed(act)
        ? stateOf("failed")
        : cancelled(act)
          ? stateOf("cancelled")
          : undefined
      : act.paused || this.session.host.pending.has(act.id)
        ? stateOf("held")
        : undefined;
    return state ? [shape, [`   ${state.mark} ${state.word}`, state.color]] : [shape];
  },

  /** The model of an actor by its name alone, with no provider, and its effort. */
  model(actor: string): { name: string; effort: string } {
    const { model, effort } = actorParts(
      actor || this.session.actor,
      this.session.roster.map(([name]) => name),
    );
    return { name: modelName(model).id, effort };
  },

  /** Who sent a prompt that is not a message of the operator, and to which model: a chain tells its model that an
   * act it waits on is done, and a rung asks a model a question. */
  sender(act: ActRow): string {
    const model = this.model(String(act.words[2] ?? "")).name;
    const maker = this.session.actOf(act.by);
    return maker?.kind === "chain" ? `the chain told ${model}` : `${act.by} asked ${model}`;
  },

  /** A word of Python in a card: its numbered lines, the reason it failed, and what to do after a refusal of the
   * gate. */
  word(box: BoxRenderable, word: string, reason?: string, hint?: string): void {
    // The code starts under the name of its rung, past the fold and the glyph of the heading.
    const inner = this.box({ gap: space.stack, paddingLeft: space.inset });
    // A rung whose word has not come yet shows no code.
    if (word) inner.add(this.numbered(word));
    // The parser names the word as <string> and says its line twice, which the reason leaves out.
    if (reason)
      inner.add(this.branched(shortenHomes(reason.replace(/\s*\(<string>, line \d+\)$/, "")), c.danger));
    // The line of what to do stands under the reason, at the column of its text.
    if (hint) inner.add(this.inset(space.between, this.text(hint, c.muted)));
    box.add(inner);
  },

  /** The one line that says what to do after the gate refused the word of a rung, and nothing for any other rung:
   * the operator fixes his own word, and the chain asks a model for its word again with the findings. */
  gateHint(act: ActRow): string | undefined {
    if (act.kind !== "rung" || !act.done || !act.run?.refused) return undefined;
    return act.by === "operator"
      ? "Press ↑ in the input to bring the word back, fix the line that the gate names, and send it again."
      : "The chain tells the model these findings and asks it for its word again.";
  },

  actDetails(box: BoxRenderable, act: ActRow): void {
    if (act.kind === "rung") {
      this.word(
        box,
        String(this.session.program[act.id] || act.words[0] || ""),
        act.run?.reason,
        this.gateHint(act),
      );
      return;
    }
    const details = this.box({ paddingLeft: space.between * 2, gap: space.stack });
    box.add(details);
    if (act.kind === "bash") {
      this.commandDetails(details, act);
      return;
    }
    if ((act.kind === "read" || act.kind === "write") && !failed(act)) {
      this.textDetails(details, act);
      return;
    }
    const row = this.partRow(act);
    if (row?.details) {
      for (const line of row.details) details.add(this.line(this.partLine(line), c.text));
      return;
    }
    const fields: Record<string, string[]> = {
      prompt: ["shape", "message", "actor"],
      rung: ["word", "retells", "actor", "returns"],
      wait: ["seconds"],
      grant: ["dollar ceiling", "context ceiling"],
    };
    details.add(this.reference(act.id, act.id));
    for (const [index, value] of act.words.entries()) {
      if (value === null || value === "") continue;
      details.add(
        this.text([
          [`${fields[act.kind]?.[index] ?? `argument ${index + 1}`}: `, c.faint],
          [display(value), c.muted],
        ]),
      );
    }
    if (act.done && act.value !== null)
      details.add(
        this.text(display(act.value), failed(act) ? c.danger : c.text, { marginTop: space.section }),
      );
  },

  /** The text that a read gave or that a write wrote, and under it its name. The row leaves the text out, so the card
   * reads the whole act. */
  textDetails(details: BoxRenderable, act: ActRow): void {
    const node = this.text("", c.text);
    details.add(node);
    details.add(this.reference(act.id, act.id));
    void this.session.host.act(act.id).then((whole) => {
      const text = (whole?.kind === "write" ? whole.words[0] : whole?.value) as
        | { content?: unknown }
        | undefined;
      if (!node.isDestroyed && typeof text?.content === "string")
        node.content = safeText(text.content.replace(/\n$/, ""));
    }, this.report);
  },

  /** What a command printed, and under it its name and how it was run. The command says its line, its exit and its time
   * in its heading. What it
   * printed to stderr takes the color of a failure, and each stream is named only when the command printed to both. */
  commandDetails(details: BoxRenderable, act: ActRow): void {
    const exit = (act.value && typeof act.value === "object" ? act.value : {}) as Exit;
    const both = Boolean(exit.stdout?.content && exit.stderr?.content);
    const shown: TextRenderable[] = [];
    for (const [name, color] of [
      ["stdout", c.text],
      ["stderr", c.danger],
    ] as const) {
      const content = exit[name]?.content;
      if (!content) continue;
      if (both)
        details.add(
          this.text(name, name === "stdout" ? c.faint : c.danger, {
            marginTop: shown.length ? space.section : 0,
          }),
        );
      // The line end that closes what a command printed opens no empty row.
      const node = this.text(content.replace(/\n$/, ""), color);
      details.add(node);
      shown.push(node);
    }
    const meta = this.box({ flexDirection: "row", marginTop: shown.length ? space.section : 0 });
    meta.add(this.reference(act.id, act.id));
    const [, input, timeout] = act.words;
    // The heading says the exit of a command and its time, so its card says only how it was run.
    const notes: Part[] = [
      [input === null ? "   input open" : "", c.faint],
      [typeof timeout === "number" && timeout !== TIMEOUT ? `   times out after ${timeout}s` : "", c.faint],
    ];
    meta.add(this.text(notes, c.faint));
    details.add(meta);
    // The row holds the tail of what the command printed, and the card reads the whole of it once it opens.
    if (act.output !== undefined)
      void this.session.host.act(act.id).then((whole) => {
        const streams = whole?.value as Exit | undefined;
        const contents = [streams?.stdout?.content, streams?.stderr?.content].filter(Boolean) as string[];
        for (const [index, node] of shown.entries())
          if (!node.isDestroyed && contents[index] !== undefined)
            node.content = safeText(contents[index].replace(/\n$/, ""));
      }, this.report);
  },

  /** What the last row of a thread shows while its model works on a step: the line that it still writes, at its
   * number after the lines it finished, else the last line that it thinks, as a comment, else `# thinking`, whose
   * letters a light crosses while the view moves. */
  thinkingParts(step: string, display: number, room: number): { number: number; parts: Part[] } {
    const stream = this.session.host.streams.get(step);
    // The lines that the model finished stand above as python, so the row takes the number after them.
    const { lines, tail } = writing(stream);
    const done = lines ? lines.split("\n").length : 0;
    if (stream?.text.trim()) return { number: display + done, parts: [[clip(tail || "…", room), c.muted]] };
    const thought = (stream?.thinking ?? "").trimEnd().split("\n").at(-1)?.trim();
    if (thought)
      return { number: display, parts: [[`# ${clip(thought, room - 2)}`, c.syntaxComment, italic]] };
    const word = "thinking";
    if (motion.still) return { number: display, parts: [[`# ${word}`, c.syntaxComment]] };
    // The light rests for a few frames after it crosses the word, so the row breathes and does not flicker.
    const lit = Math.floor(clock() / frame) % (word.length + 8);
    return {
      number: display,
      parts: [
        ["# ", c.syntaxComment],
        ...[...word].map(
          (letter, at): Part => [
            letter,
            at === lit ? c.text : Math.abs(at - lit) === 1 ? c.muted : c.syntaxComment,
          ],
        ),
      ],
    };
  },

  /** The last row of a thread while its model works on a step: a loader, the number of the line that comes, then what
   * the model thinks or writes, which the tick draws again. */
  thinkingRow(step: string, display: number, digits: number, room: number): LineNumberRenderable {
    const { number, parts } = this.thinkingParts(step, display, room);
    const text = this.text(parts, c.muted, { wrapMode: "none" });
    const view = new LineNumberRenderable(this.renderer, {
      target: text,
      paddingRight: space.inset,
      fg: c.faint,
      minWidth: gutterWidth(digits),
      lineNumbers: new Map([[0, number]]),
      // A sign of the gutter widens the gutter by its width, which moves the number and the code of this row off the
      // columns of the rows above it: the loader takes the first column of the gutter, which no number reaches.
      renderAfter(buffer) {
        buffer.drawText(spin(), this.x, this.y, parseColor(c.accent));
      },
    });
    // The loader is drawn by each frame, the first one too, so no tick is needed before it shows.
    this.thinkingRows.set(step, {
      view,
      text,
      display,
      room,
      said: saidOf(number, parts),
      wrote: number - display,
    });
    return view;
  },

  /** The rows that models write move with the tick: the loader turns, and the text follows the stream. */
  tickThinking(): void {
    for (const [step, row] of this.thinkingRows) {
      if (row.view.isDestroyed) {
        this.thinkingRows.delete(step);
        continue;
      }
      row.view.requestRender();
      this.followThinking(step);
    }
  },

  /** The row of a model follows its stream. A line that the model finished stands in the card above the row, which the
   * next frame of the feed makes: until the card shows it, the row keeps its text and its number, so that no frame
   * shows the line twice, or the next number under a program that lacks its line. */
  followThinking(step: string, wrote?: number): void {
    const row = this.thinkingRows.get(step);
    if (!row || row.view.isDestroyed) return;
    if (wrote !== undefined) row.wrote = wrote;
    const { number, parts } = this.thinkingParts(step, row.display, row.room);
    if (number - row.display !== row.wrote) {
      this.schedule();
      return;
    }
    const said = saidOf(number, parts);
    if (said === row.said) return;
    row.said = said;
    row.view.setLineNumbers(new Map([[0, number]]));
    row.text.content = styled(parts);
  },

  /** A program of the feed: its lines as one block of python, each numbered as a line of its word, with what the
   * acts of a line came to as a quiet comment beside it, the line that runs lit with its time, and the line that failed
   * marked, with its reason under it as a comment. */
  programView(
    lines: readonly ProgramLine[],
    room: number,
    took: (step: ActRow) => string,
    since: (act?: ActRow) => string,
    heads: ReadonlySet<ProgramLine>,
    digits = 1,
    options: { more?: (line: ProgramLine) => void } = {},
  ): Renderable {
    const texts: string[] = [];
    const ellipses: number[] = [];
    const numbers = new Map<number, number>();
    const hidden = new Set<number>();
    const colors = new Map<number, { content: RGBA; gutter: RGBA }>();
    // The mark of a line stands at the right end of its row: it runs, it is awaited, or it failed.
    const markGroups = {
      [glyph.running]: "mark.running",
      [glyph.awaited]: "mark.awaits",
      [glyph.failed]: "mark.failed",
    };
    const marked = (body: string, mark: string) => `${body}  ${mark}`;
    const flag = (at: number, from: number) =>
      fixed.push([
        at,
        from,
        from + 1,
        markGroups[(texts[at] ?? "")[from] as keyof typeof markGroups] ?? "default",
      ]);
    const markOf = (line: Extract<ProgramLine, { kind: "code" }>) =>
      line.fails ? glyph.failed : line.awaited !== undefined ? glyph.awaited : line.runs ? glyph.running : "";
    const widest = Math.max(
      0,
      ...lines.map((line) => (line.kind === "code" ? Bun.stringWidth(line.text) : 0)),
    );
    const noteOf = (line: Extract<ProgramLine, { kind: "code" }>) =>
      [
        this.lineHint(line),
        line.runs ? since(line.awaits?.act ?? line.acts.at(-1)) : "",
        heads.has(line) ? took(line.step) : "",
      ]
        .filter(Boolean)
        .join("  ");
    // The code takes the room that its hints leave, and the hints take 30 columns at most: a longer hint is cut.
    const notes = Math.max(
      0,
      ...lines.map((line) => (line.kind === "code" ? Bun.stringWidth(noteOf(line)) : 0)),
    );
    const column = Math.max(24, Math.min(100, widest, room - (notes ? Math.min(notes, 26) + 4 : 0)));
    const fit = (text: string, most: number) => clip(text, Math.max(4, most));
    const lane = lines.some((line) => line.kind === "code" && markOf(line)) ? 2 : 0;
    // The line of the program that each row shows, which a click opens or folds.
    const sources: (ProgramLine | undefined)[] = [];
    // The strings that each row folds, and the rows that show the strings of a line whole, each with where it starts
    // in the text of its line.
    const folded = new Map<number, { line: Extract<ProgramLine, { kind: "code" }>; folds: Fold[] }>();
    const whole = new Map<number, number>();
    // The colors come from each statement whole, so a line cut to its room or a folded string changes no color: the
    // program as it ran, each part of a row with where it starts in that program, and the rows that only comment.
    let program = "";
    const pieces: [row: number, from: number, to: number, at: number][] = [];
    const fixed: [row: number, from: number, to: number, group: string][] = [];
    const comment = (row: number, from = 0) => fixed.push([row, from, texts[row]?.length ?? 0, "comment"]);
    const push = (text: string, number?: number, source?: ProgramLine) => {
      const at = texts.length;
      texts.push(text);
      sources.push(source);
      if (number === undefined) hidden.add(at);
      else numbers.set(at, number);
      return at;
    };
    for (const line of lines) {
      if (line.kind === "code") {
        const note = noteOf(line);
        // An open line, and a line whose strings the operator opened, show the whole text, cut at the column into
        // rows of their own. Another line folds its longest strings until it fits.
        const shown =
          this.lineOpen(line) || this.openStrings(line).size ? undefined : foldStrings(line.text, column);
        const cuts = shown ? [] : wrap(line.text, column);
        const parts = shown
          ? [fit(shown.text, column)]
          : cuts.map((cut) => " ".repeat(cut.indent) + line.text.slice(cut.from, cut.to));
        const code = parts[0] ?? "";
        const mark = markOf(line);
        // The mark stands in a lane of its own after the column of the code, before the comment, so that no line is
        // longer for it.
        const hint = note ? `# ${fit(note, room - column - 4 - lane)}` : "";
        const left = `${code}${" ".repeat(Math.max(0, column - Bun.stringWidth(code)))}`;
        const body = hint || mark ? `${left}  ${lane ? (mark ? `${mark} ` : "  ") : ""}${hint}` : code;
        const at = push(body, line.display, line);
        if (hint) fixed.push([at, body.length - hint.length, body.length, "comment"]);
        if (mark) flag(at, left.length + 2);
        for (const more of parts.slice(1)) push(more, undefined, line);
        const folds = shown?.folds.filter((fold) => fold.at + FOLD.length <= Bun.stringWidth(code)) ?? [];
        if (folds.length) folded.set(at, { line, folds });
        if (!shown) for (const [k, cut] of cuts.entries()) whole.set(at + k, cut.from - cut.indent);
        const base = program.length;
        program += `${line.text}\n`;
        if (shown) {
          const found = shown.folds.length ? literals(line.text) : [];
          let from = 0;
          let source = base;
          for (const fold of shown.folds) {
            if (fold.at >= code.length) break;
            pieces.push([at, from, fold.at, source]);
            fixed.push([at, fold.at, Math.min(fold.at + FOLD.length, code.length), "string"]);
            from = fold.at + FOLD.length;
            source = base + (found[fold.literal]?.end ?? 0);
          }
          if (from < code.length) pieces.push([at, from, code.length, source]);
        } else
          for (const [k, cut] of cuts.entries())
            pieces.push([at + k, cut.indent, cut.indent + cut.to - cut.from, base + cut.from]);
        if (line.runs) {
          colors.set(at, { content: c.selected, gutter: c.selected });
        }
        // The line that made the act that a running line awaits is lit too, with a link to the line that awaits it.
        if (line.awaited !== undefined) {
          colors.set(at, { content: c.selected, gutter: c.selected });
        }
        if (line.fails) {
          for (const why of line.fails
            .split("\n")
            .filter((one) => one.trim())
            .slice(0, 3))
            comment(push(`# ${fit(plainReason(why), room - 4)}`));
        }
      } else if (line.kind === "output") comment(push(`#   ${fit(line.text, room - 6)}`));
      else if (line.kind === "writes") {
        const body = `# ${fit(line.text, room - 7)}`;
        const at = push(marked(body, glyph.running));
        fixed.push([at, 0, body.length, "comment"]);
        flag(at, body.length + 2);
      } else if (line.kind === "earlier") ellipses.push(push(line.text, undefined, line));
      else comment(push(fit(line.text, room - 2)));
    }
    // Each fold is underlined, at its place in the text of the program.
    const starts: number[] = [];
    let offset = 0;
    for (const text of texts) {
      starts.push(offset);
      offset += text.length + 1;
    }
    // A row that stands for lines that the window leaves out is an underlined ..., which a click opens.
    const ellipsis = (): SimpleHighlight[] =>
      ellipses.map(
        (row): SimpleHighlight => [starts[row] ?? 0, (starts[row] ?? 0) + (texts[row]?.length ?? 0), "fold"],
      );
    const marks: SimpleHighlight[] = [...folded].flatMap(([row, { folds }]) =>
      folds.map(
        (fold): SimpleHighlight => [
          (starts[row] ?? 0) + fold.at,
          (starts[row] ?? 0) + fold.at + FOLD.length,
          "fold",
        ],
      ),
    );
    // The fold under the pointer: a hover shows its string whole, and a click opens it in place.
    const foldAt = (x: number, y: number) => {
      const row = y - view.y;
      const one = folded.get(row);
      const at = x - target.x;
      const fold = one?.folds.find((fold) => {
        const from = Bun.stringWidth((texts[row] ?? "").slice(0, fold.at));
        return at >= from && at < from + FOLD.length;
      });
      return one && fold ? { line: one.line, fold } : undefined;
    };
    const hoverFold = (event: MouseEvent) => {
      const hit = foldAt(event.x, event.y);
      if (!hit) return false;
      clearTimeout(this.hoverTimer);
      this.unhover();
      this.hoverTimer = setTimeout(() => {
        const raw = literals(hit.line.text)[hit.fold.literal]?.raw;
        const text = raw
          ? hit.fold.whole
          : hit.fold.whole.replace(/\\([nt"'\\])/g, (_, one: string) =>
              one === "n" ? "\n" : one === "t" ? "  " : one,
            );
        this.hoverCard(
          event.x,
          event.y,
          14,
          () => this.openString(hit.line, hit.fold.literal),
          this.text(text.slice(0, 2000), c.text),
        );
      }, 220);
      return true;
    };
    // A line under the pointer shows what it holds in a card once the pointer rests on it: the whole reason of a line
    // that failed, what each act of the line came to, the act that it awaits, or a message whole.
    let pointed: ProgramLine | undefined;
    const widestRow = Math.max(0, ...texts.map((text) => Bun.stringWidth(text)));
    const hoverLine = (event: MouseEvent) => {
      const line = sources[event.y - view.y];
      const telling =
        line?.kind === "code"
          ? Boolean(line.fails || line.acts.length || line.awaits)
          : line?.kind === "note" && line.step.kind === "prompt";
      if (!line || !telling) {
        pointed = undefined;
        return false;
      }
      if (pointed === line && this.hover) return true;
      clearTimeout(this.hoverTimer);
      this.unhover();
      pointed = line;
      // The card stands right of the program, on the height of the row, so it hides no line of the program.
      const right = target.x + widestRow + space.between;
      this.hoverTimer = setTimeout(
        () => void this.lineCard(line, Math.max(event.x, right), event.y - 1),
        220,
      );
      return true;
    };
    const hover = (event: MouseEvent) => hoverFold(event) || hoverLine(event);
    // Each run of characters of one group in a part of a row takes the group that it has in the program.
    const highlights = async (): Promise<SimpleHighlight[]> => {
      const groups = await this.groupsOf(program);
      const out: SimpleHighlight[] = [];
      for (const [row, from, to, at] of pieces) {
        const start = starts[row] ?? 0;
        let run = from;
        for (let k = from + 1; k <= to; k++) {
          const group = groups[at + run - from];
          if (k < to && groups[at + k - from] === group) continue;
          if (group) out.push([start + run, start + k, group]);
          run = k;
        }
      }
      for (const [row, from, to, group] of fixed)
        out.push([(starts[row] ?? 0) + from, (starts[row] ?? 0) + to, group]);
      return [...out, ...marks, ...ellipsis()];
    };
    const target = this.code(texts.join("\n"), marks, hover, highlights);
    // Each row is cut to its room already, so none wraps: a row that is too long loses its end, and no mark falls under it.
    target.wrapMode = "none";
    const view = new LineNumberRenderable(this.renderer, {
      target,
      paddingRight: space.inset,
      fg: c.faint,
      minWidth: gutterWidth(digits),
      lineNumbers: numbers,
      hideLineNumbers: hidden,
      lineColors: colors,
    });
    // The gutter of the numbers takes the events of the pointer over the program, so it shows the string of a fold.
    view.onMouseMove = (event) => {
      if (hover(event)) return;
      clearTimeout(this.hoverTimer);
      this.unhover();
    };
    // A press that ends a drag selects text and opens nothing. A press on a fold opens its string, and a press on a
    // string that shows whole folds it again.
    view.onMouseUp = (event) => {
      const row = event.y - view.y;
      const line = sources[row];
      if (line?.kind === "earlier" && options.more && !this.renderer.getSelection()?.getSelectedText())
        return options.more(line);
      if (line?.kind !== "code" || this.renderer.getSelection()?.getSelectedText()) return;
      const hit = foldAt(event.x, event.y);
      if (hit) return this.openString(hit.line, hit.fold.literal);
      const open = this.openStrings(line);
      const index = (whole.get(row) ?? -1) + event.x - target.x;
      const literal = literals(line.text).findIndex(
        (one, k) => open.has(k) && whole.has(row) && index >= one.start - 1 && index <= one.end,
      );
      if (literal >= 0) return this.openString(line, literal);
      if (line.acts.length || Bun.stringWidth(line.text) > column) this.openLine(line);
    };
    return view;
  },

  /** The card of a line of a program under the pointer: the reason of a line that failed whole, in the color of a
   * failure, the act that a running line awaits, and what each act of the line came to, read whole from the host, or
   * the message of a note whole. A press opens the act, or the line. */
  async lineCard(line: ProgramLine, x: number, y: number): Promise<void> {
    const nodes: Renderable[] = [];
    let open = () => this.openLine(line);
    if (line.kind === "note") {
      nodes.push(this.text(clip(String(line.step.words[1] ?? line.text), 1200), c.text));
      open = () => this.go("feed", line.step.id);
    } else if (line.kind === "code") {
      if (line.fails) {
        const refused = Boolean((line.step as ActRow).run?.refused);
        nodes.push(
          this.text(
            `${glyph.failed} ${refused ? "The gate refused" : `${line.step.id} raised at`} line ${line.display}`,
            c.danger,
            { attributes: bold },
          ),
          this.text(
            shortenHomes(line.fails.split("\n").map(plainReason).join("\n")).slice(0, 1200),
            c.danger,
          ),
        );
      }
      const acts = [...(line.awaits ? [line.awaits.act] : []), ...line.acts.slice(-3)] as ActRow[];
      for (const act of acts) {
        const held = await this.actPreview(act);
        const state = act.done
          ? stateOf(failed(act) ? "failed" : "done")
          : { word: "runs", mark: glyph.running, color: c.accent };
        const head: Part[] = [
          [act.id, c.link, bold],
          [`  ${act.kind}  `, c.secondary],
          [clip(this.subject(act).split("\n")[0] ?? "", 30), c.muted],
          [
            `  ${state.mark} ${line.awaits?.act === act ? `made on line ${line.awaits.display}` : state.word}`,
            state.color,
          ],
        ];
        nodes.push(this.text(head), this.text(held.slice(0, 1200) || " ", c.muted, { maxHeight: 10 }));
      }
      const first = acts[0];
      if (first && !line.fails) open = () => void this.follow(first.id).catch(this.report);
    }
    if (!nodes.length || this.closed || this.overlay) return;
    this.hoverCard(x, y, 16, open, ...nodes);
  },

  /** What an act came to, as its card under the pointer shows it: the text of a read or of a write, what a command
   * printed, and the value of any other act, read whole from the host. */
  async actPreview(act: ActRow): Promise<string> {
    if (!act.done) return act.kind === "bash" ? tailOf(act, 10) || "runs" : "pending";
    const whole = (await this.session.host.act(act.id).catch(() => undefined)) ?? act;
    const text = (value: unknown) => (value as { content?: unknown } | null)?.content;
    if (act.kind === "read" && typeof text(whole.value) === "string")
      return String(text(whole.value)).trimEnd();
    if (act.kind === "write" && typeof text(whole.words[0]) === "string")
      return String(text(whole.words[0])).trimEnd();
    if (act.kind === "bash") {
      const exit = (whole.value ?? {}) as Exit;
      const printed = [exit.stdout?.content, exit.stderr?.content].filter(Boolean).join("\n").trimEnd();
      return printed || (exit.code === null || exit.code === undefined ? "timed out" : `exit ${exit.code}`);
    }
    return display(whole.value);
  },

  /** The strings of a line of a program that the operator opened, which show whole. */
  openStrings(line: Extract<ProgramLine, { kind: "code" }>): Set<number> {
    const folds = this.session.folds;
    const key = `string:${line.step.id}:${line.number}:`;
    // Each frame asks of each line of each program, so the lines with an open string are read once from the folds.
    if (this.stringLines?.folds !== folds)
      this.stringLines = {
        folds,
        lines: new Set(
          Object.keys(folds)
            .filter((one) => one.startsWith("string:") && folds[one] === false)
            .map((one) => one.slice(0, one.lastIndexOf(":") + 1)),
        ),
      };
    if (!this.stringLines.lines.has(key)) return new Set();
    return new Set([...literals(line.text).keys()].filter((k) => folds[key + k] === false));
  },

  /** A string of a line of a program opens in place, or folds again. */
  openString(line: Extract<ProgramLine, { kind: "code" }>, literal: number): void {
    const w = this.session;
    w.folds[`string:${line.step.id}:${line.number}:${literal}`] = this.openStrings(line).has(literal);
    this.stringLines = undefined;
    w.save();
    this.unhover();
    // A feed that follows its end would push the line up as it grows, so the view keeps its offset.
    const top = this.scroll.scrollTop;
    this.renderContent();
    this.scrollNow(top);
    this.scrollAfterLayout(top);
  },

  /** Whether a line of a program stands open: its whole text, and what its acts gave under it. */
  lineOpen(line: ProgramLine): boolean {
    return line.kind === "code" && this.session.folds[`line:${line.step.id}:${line.number}`] === false;
  },

  /** A line of a program opens, or folds again. */
  openLine(line: ProgramLine): void {
    if (line.kind !== "code") return;
    const w = this.session;
    w.folds[`line:${line.step.id}:${line.number}`] = this.lineOpen(line);
    w.save();
    this.renderContent();
  },

  /** The quiet hint of a line of a program: what a part of an extension draws as the row of each act of the line, and
   * else what its acts came to. */
  lineHint(line: Extract<ProgramLine, { kind: "code" }>): string {
    if (!this.partRows) return line.hint;
    const drawn = line.acts.map((act) => this.partRow(act as ActRow)?.label).filter(Boolean);
    return drawn.length ? drawn.join("  ") : line.hint;
  },

  /** Python with the number of each line. */
  numbered(word: string, options: LineNumberOptions = { fg: c.faint, minWidth: 3 }): LineNumberRenderable {
    return new LineNumberRenderable(this.renderer, {
      target: this.code(word),
      paddingRight: space.inset,
      ...options,
    });
  },

  /** A text of a dialog, which scrolls past the rows it takes. */
  document(node: Renderable, options: ScrollBoxOptions): ScrollBoxRenderable {
    const document = new ScrollBoxRenderable(this.renderer, { scrollX: false, scrollY: true, ...options });
    document.add(node);
    return document;
  },

  /** A card under the pointer, which a press opens. */
  hoverCard(x: number, y: number, rows: number, open: () => void, ...nodes: Renderable[]): void {
    const area = this.region(x);
    const width = Math.min(58, area.width - 4);
    this.popup(
      {
        left: Math.max(area.left + 1, Math.min(x, area.left + area.width - width - 1)),
        top: Math.max(1, Math.min(y + 1, this.renderer.height - rows)),
        width,
        maxHeight: rows,
        paddingX: space.between,
        paddingY: space.inset,
        onMouseDown: open,
      },
      ...nodes,
    );
  },

  reference(label: string, value: string): TextRenderable {
    const node = this.text(label, c.link, { attributes: underline });
    node.onMouseDown = () => {
      void this.follow(value).catch(this.report);
    };
    node.onMouseOver = (event) => {
      void this.referenceHover(value, event.x, event.y);
    };
    node.onMouseOut = () => {
      this.unhover();
    };
    return node;
  },

  async follow(value: string): Promise<void> {
    this.unhover();
    const act = this.session.actOf(value);
    if (value.startsWith("furb-image://")) this.imageActions(value);
    else if (act?.kind === "chain") await this.session.select(act.id);
    else if (act && act.id === value && act.on === this.session.selected) this.go("feed", act.id);
    else this.showValue(value, await this.referenced(value));
  },

  /** What a reference holds, as a view reads it: the text of a door, which the outcome of its act holds under the
   * path of that door as an Exit holds its streams; the act the door opens on, whole, while its outcome holds no
   * such text; or the text that the host reads at the path. This read is no act of the operator, so it makes none
   * and the journal keeps nothing of it. */
  async referenced(value: string): Promise<unknown> {
    const act = this.session.actOf(value);
    if (!act) return (await this.session.host.look(value, this.session.selected)).content;
    const whole = (await this.session.host.act(act.id)) ?? act;
    const held = whole.value && typeof whole.value === "object" ? Object.values(whole.value) : [];
    const door = held.find(
      (one): one is { path: string; content: string } =>
        Boolean(one) && typeof one === "object" && (one as { path?: unknown }).path === value,
    );
    return door ? door.content : whole;
  },

  async referenceHover(value: string, x: number, y: number): Promise<void> {
    try {
      const act = this.session.acts.find((act) => act.id === value);
      const detail = value.startsWith("furb-image://")
        ? "Image attachment. Click to open its actions."
        : act
          ? `${act.kind}  ${act.done ? display(act.value) : "pending"}`
          : display(await this.referenced(value));
      if (this.closed || this.overlay) return;
      this.hoverCard(
        x,
        y,
        8,
        () => void this.follow(value).catch(this.report),
        this.text(value, c.link, { attributes: bold }),
        this.text(detail.slice(0, 400), c.muted, { maxHeight: 4 }),
      );
    } catch {
      /* A path can have disappeared since the turn was written. */
    }
  },

  /** The python of a user turn: a header is `#` and the name of an act or the kind of a query, a comment is `#` and
   * a space, and a header that names an act and an image attachment are references. */
  transcriptText(source: string): TextRenderable {
    const text = safeText(source);
    const chunks: TextChunk[] = [];
    let at = 0;
    for (const match of text.matchAll(/^#(?! |$)\S+|^#(?: .*)?$|furb-image:\/\/[\w.]+/gm)) {
      if (match.index > at) chunks.push({ __isChunk: true, text: text.slice(at, match.index), fg: c.text });
      chunks.push({
        __isChunk: true,
        text: match[0],
        fg: match[0].startsWith("furb-image://")
          ? c.syntaxString
          : /^#(?! |$)/.test(match[0])
            ? c.syntaxKeyword
            : c.syntaxComment,
      });
      at = match.index + match[0].length;
    }
    chunks.push({ __isChunk: true, text: text.slice(at), fg: c.text });
    return this.pointable(
      new TextRenderable(this.renderer, { content: new StyledText(chunks), wrapMode: "word", flexShrink: 0 }),
      text,
    );
  },
});

export type DetailsPart = typeof details;
