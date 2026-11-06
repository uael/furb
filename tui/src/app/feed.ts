import { type BoxRenderable, DiffRenderable, type RGBA } from "@opentui/core";
import { display, imageReferences } from "furb";
import {
  actLines,
  asksOperator,
  cancelled,
  earProgram,
  gist,
  isDone,
  type NoteRun,
  operated,
  outcomeOf,
  type ProgramLine,
  pauseSaid,
  stepLines,
  thread,
  threadProgram,
  type wants,
  writing,
} from "furb/life";
import { clock } from "../clock.ts";
import { fromDirectory, shortenHomes } from "../files.ts";
import { clip, dollars, modelLabel, plainFailure } from "../format.ts";
import type { Mark as PartMark } from "../parts.ts";
import { hunkPatch, type Scope } from "../review.ts";
import type { ActRow, View } from "../session.ts";
import { theme as c, glyph, markOf, spacing as space, spin, threadTone } from "../theme.ts";
import { bold, italic, type Part, plain } from "../ui.ts";
import { branchMessages, branchStart, branchTaken, branchTakes } from "../watching.ts";
import { part } from "./part.ts";
import { type BlockOptions, hidden, MESSAGE_ROWS, PROGRAM_STEPS, spell, viewLabels } from "./shared.ts";
import { SCROLL } from "./thread.ts";

/** A note of the round, which the feed draws alone or in a run with the notes that follow it. */
interface Note {
  act: ActRow;
  indent: number;
  tone: ReturnType<typeof threadTone>;
  /** The text that a search reads. */
  shown: string;
  /** What its card is made of, which changes when the row changes. */
  key: string;
  /** The row that it draws in the panel of its block. */
  row: () => BoxRenderable;
}

export const feed = part({
  /** What a question that the input answers says under it: the list of its values takes the arrows and the numbers,
   * and a field takes the words of the shape. */
  answeredBelow(ask: Parameters<typeof wants>[0]): Part[] {
    if (ask.choices.length) return [["The list below answers this question", c.muted]];
    return [
      [`The input below answers this question, as ${ask.shape}, or `, c.muted],
      ["⌃A", c.text],
      [" opens each in turn", c.muted],
    ];
  },

  renderContent(): void {
    const w = this.session;
    const view = this.tree ? "tree" : `${w.selected}:${w.view}`;
    this.treeBar.visible = Boolean(this.tree);
    let opened = false;
    if (this.lastView !== view) {
      opened = true;
      if (this.lastView && this.lastView !== "tree") w.scrolls[this.lastView] = this.place;
      this.clear(this.scroll);
      this.cards.clear();
      this.paneKeys.delete(this.scroll);
      this.lastView = view;
      this.scroll.stickyScroll = w.view === "feed" && !this.tree;
      // A view opens where it was left, and a feed that was never shown opens at its end, once it is laid out with the
      // rows around the input that it shares the screen with.
      if (!this.tree) {
        const place = w.scrolls[view] ?? (this.scroll.stickyScroll ? "end" : 0);
        this.scrollNow(place);
        this.scrollAfterLayout(place);
      }
    }
    this.renderStage();
    this.renderWatch();
    if (this.tree) {
      this.renderTree();
      return;
    }
    const existing = new Set(this.cards.keys());
    this.feedOrder = undefined;
    const used = new Map<string, number>();
    const matches = (text: string) => !w.search || text.toLowerCase().includes(w.search.toLowerCase());
    let order = 0,
      items = 0,
      group = "";
    // The notes of a run that the view names wait here until the last of them comes, which then draws as one block under
    // one header.
    const run: Note[] = [];
    // A card that shows an item of the view says the text that the filter reads, and counts as an item once it shows.
    // A card of a done act names in same what its card is made of, and it is made again only when one of these, or
    // its place, changed: a change of the life makes again only the cards of the acts that changed, so that a frame
    // takes the same time in a long life as in a short one.
    const add = (
      id: string,
      key: string,
      label: Part[] | (() => Part[]),
      body: (box: BoxRenderable) => void,
      options: BlockOptions & { shown?: string; same?: readonly unknown[] } = {},
    ) => {
      if (options.shown !== undefined) {
        if (!matches(options.shown)) return;
        items++;
      }
      // Two cards never share a name: a second card of one name takes the count of its name.
      const taken = used.get(id) ?? 0;
      used.set(id, taken + 1);
      if (taken) id = `${id}~${taken + 1}`;
      existing.delete(id);
      const heading = !options.group || group !== options.group;
      group = options.group ?? "";
      const shape = {
        ...options,
        heading: options.heading ?? heading,
        separate: order > 0 && (options.separate ?? heading),
      };
      const same = options.same && [
        ...options.same,
        shape.heading,
        shape.separate,
        this.theme,
        this.feedWidth,
        this.partsLoaded,
      ];
      const prior = this.cards.get(id);
      if (
        same &&
        prior?.same?.length === same.length &&
        same.every((one, at) => prior.same?.[at] === one) &&
        this.feedChild(order) === prior.node
      ) {
        // The label of an act that works moves with time, which a render reads again as the tick does.
        if (options.title) this.label(prior, [...prior.marker, ...options.title()]);
        order++;
        return;
      }
      this.card(id, key, typeof label === "function" ? label() : label, body, order, shape);
      const made = this.cards.get(id);
      if (made) made.same = same;
      order++;
    };
    // The notes of a run draw as one card: one header, which counts them and the ones not yet seen, and one panel that
    // holds their rows one under the other, with no empty band between them. A run of one draws as it always did.
    const drawNotes = (all: Note[], of?: NoteRun) => {
      const notes = all.filter((one) => matches(one.shown));
      const [first] = notes;
      if (!first) return;
      if (!of || of.count === 1) {
        add(first.act.id, first.key, [], (box) => this.panel(box, first.tone).add(first.row()), {
          group: "user",
          heading: false,
          separate: true,
          open: true,
          act: first.act,
          indent: first.indent,
          shown: first.shown,
        });
        return;
      }
      const unread = w.unreadNotes(of);
      const [dot, ink] = markOf("unread");
      const label: Part[] = [
        [`${glyph.prompt} `, c.faint],
        ["Notes", c.text, bold],
        [`  ${of.count}`, c.faint],
        ...(unread ? [[`  ${dot} ${unread} unread`, ink] as Part] : []),
      ];
      add(
        first.act.id,
        `notes\n${of.count}\n${unread}\n${notes.map((one) => one.key).join("\n--\n")}`,
        label,
        (box) => {
          const inner = this.panel(box, first.tone);
          for (const one of notes) inner.add(one.row());
        },
        {
          group: "user",
          heading: true,
          separate: true,
          open: true,
          act: first.act,
          indent: first.indent,
          shown: notes.map((one) => one.shown).join("\n"),
        },
      );
    };
    let held: NoteRun | undefined;
    const flushNotes = () => {
      if (run.length) drawNotes(run.splice(0), held);
    };
    if (w.preferences.notice)
      add("preferences-notice", w.preferences.notice, [], (box) => {
        const panel = this.panel(box, c.warning);
        panel.add(
          this.text(w.preferences.notice, c.text, {
            run: () => {
              w.preferences.notice = "";
              this.renderContent();
            },
          }),
        );
      });
    if (w.host.lost)
      add("life-lost", w.host.lost, [], (box) => {
        const actions: [string, string, () => void][] = w.host.local
          ? [["Open the life again", "reaches it again, /reopen", () => void w.reopen().catch(w.fail)]]
          : [];
        this.banner(
          box,
          c.danger,
          [
            [`${glyph.failed} `, c.danger],
            ["The life is not open in this window", c.text, bold],
          ],
          this.text(w.host.lost, c.muted),
          ...actions,
        );
      });
    else if (w.error && w.view === "changes" && w.damagedJournal)
      // A damaged journal says what broke; only a writable life offers to repair it or to start a new one.
      add("view-error", `${w.view}:${w.damagedJournal}`, [], (box) => {
        const actions: [string, string, () => void][] = w.host.reading
          ? []
          : [
              [
                "Repair the journal",
                `keeps each change that it can read, ${this.chordOf("repair")}`,
                () => void w.repairJournal().catch(w.fail),
              ],
              [
                "Start a new journal",
                `keeps the damaged one beside it, ${this.chordOf("renew")}`,
                () => void w.renewJournal().catch(w.fail),
              ],
            ];
        this.banner(
          box,
          c.danger,
          [
            [`${glyph.failed} `, c.danger],
            ["The journal of file changes is damaged", c.text, bold],
          ],
          this.text(shortenHomes(w.damagedJournal), c.muted),
          ...actions,
        );
      });
    else if (w.error)
      add("view-error", `${w.view}:${w.error}`, [], (box) =>
        this.banner(
          box,
          c.danger,
          [
            [`${glyph.failed} `, c.danger],
            [`The ${viewLabels[w.view].toLowerCase()} view could not load`, c.text, bold],
          ],
          this.text(shortenHomes(plainFailure(w.error)), c.muted),
          [
            "Refresh view",
            "reads the view again",
            () => {
              w.error = "";
              void w.refresh().catch(w.fail);
            },
          ],
        ),
      );
    // A chain that holds no act of the operator or of a model shows the welcome, so that the steps of the extensions,
    // which each chain runs as it starts, do not hide it.
    const welcomed = this.welcoming && !opened;
    this.welcoming =
      w.view === "feed" &&
      !w.search &&
      !w.loading &&
      !w.error &&
      w.moment === undefined &&
      !w.paused &&
      !w.actOf(w.selected)?.words[1] &&
      !w.activity.some((act) => operated(act.by) || act.kind === "prompt");
    if (this.welcoming) {
      const { width, height } = this.scroll.viewport;
      add("welcome", `welcome:${width}:${height}:${this.theme}:${this.welcomeKey()}`, [], (box) =>
        this.welcome(box, height),
      );
      items++;
      // The welcome is read from its top, so a screen that is lower than it shows its logo first and not its end.
      if (!welcomed) {
        this.scroll.stickyScroll = false;
        this.scrollNow(0);
        this.scrollAfterLayout(0);
      }
    } else if (w.view === "feed") {
      // The feed that replaces the welcome opens at its end, as a feed that was never shown does. The welcome asked
      // the next frame for its top, and a screen that had not drawn that frame yet must not take it for the feed.
      if (welcomed) {
        this.scroll.stickyScroll = !this.tree;
        this.scrollNow("end");
        this.scrollAfterLayout("end");
      }
      // The feed shows the acts as they stood at the moment that the operator moved to, or as they stand now. Its fold
      // reads them from the index of the session, which spares the fold a pass over every act in each frame.
      const shown = w.at(w.acts);
      const past = w.moment !== undefined;
      const index = w.ladderIndex;
      const rows = index.ids;
      const madeBy = (id: string): readonly ActRow[] => index.made.get(id) ?? [];
      // The feed of a chain is its ladder, which the model of a life gives every host: each prompt with the rungs of
      // its ladder, the acts of each rung, and what the prompt closed with.
      // A fold that the operator opened shows the earlier prompts of the thread, or the earlier rungs of a ladder.
      const opened = new Set(
        Object.entries(w.folds).flatMap(([key, closed]) =>
          closed === false && /^(prompts|rungs):/.test(key) ? [key] : [],
        ),
      );
      const owed = new Set(w.host.prompts.keys());
      const word = (rung: ActRow) => this.subject(rung);
      const own = thread(w.selected, word, w.host.streams, owed, opened, index);
      // A branch takes from its source each message before the word that made it, which the word does not leave out,
      // with what answered it. Its feed shows these threads as they stood when the branch began, after the line that
      // names the source and before its own acts, since its model reads them as its own turns.
      const branch = rows.get(w.selected);
      const source = branch?.kind === "chain" ? String(branch.words[1] ?? "") : "";
      const maker = branch && (w.actOf(String(branch.by)) ?? branch);
      const taken = new Set(
        source && maker
          ? branchTaken(shown, source, maker, this.subject(maker), (one) => w.isUserPrompt(one)).map(
              (one) => one.id,
            )
          : [],
      );
      // The threads that the branch took stand as they stood at the fact before the one that made the branch, and no
      // other step of its source stands at their top.
      const frozen = branch && taken.size ? w.ladderAt(source, branch.began - 1) : undefined;
      const takenMade = () => frozen?.made ?? new Map<string, readonly ActRow[]>();
      const takenLines = frozen
        ? thread(source, word, w.host.streams, owed, opened, {
            ...frozen,
            on: frozen.on.filter((one) => taken.has(one.id)),
          })
        : [];
      // The words that retell the source stand first in the feed of the branch, as the acts that the branch made.
      const retelling = own.findIndex((line) => line.depth === 0 && line.act.by !== w.selected);
      const after = retelling < 0 ? own.length : retelling;
      const lines = [...own.slice(0, after), ...takenLines, ...own.slice(after)];
      let retold = false;
      // The feed is one program of threads: each thread is its header, then the words of its steps as python, with
      // what each line came to as a quiet comment beside it. A thread that the operator collapsed shows its header
      // line alone.
      const threadsOwn = w.threads;
      const waiting = new Set(w.desk.asks.map((ask) => ask.id));
      const named = (chain: string) => w.labelOf(chain);
      const madeMap = () => index.made;
      // A program is made again only when what it shows changed: the session counts each time a snapshot changes a step,
      // its word or an act of it, so the key holds those counts and no list of the fields that the drawing reads. A new
      // act on another thread leaves it as it was.
      const stepOf = (rung: ActRow) =>
        [
          rung.id,
          w.revision(rung.id),
          ...madeBy(rung.id).map((one) => `${one.id}.${w.revision(one.id)}`),
        ].join(":");
      const liveOf = (ids: readonly string[]) =>
        ids
          .map((id) =>
            [
              // An act of another branch can take the same id, so the place of the act in its record names it too.
              `${id}@${index.ids.get(id)?.began ?? ""}`,
              ...madeBy(id)
                .filter((one) => one.kind === "rung")
                .map(stepOf),
            ].join(" "),
          )
          .join("|");
      const programFor = (key: string, live: string, make: () => ProgramLine[]): ProgramLine[] => {
        const known = this.programs.get(key);
        if (!past && known?.version !== -1 && known?.session === w && known?.live === live)
          return known.lines;
        const made = make();
        if (this.programs.size > 2000) this.programs.clear();
        this.programs.set(key, { session: w, version: past ? -1 : w.version, live, lines: made });
        return made;
      };
      // What a step took and cost once it is done, and how long the line that waits has waited.
      // The cost stands on the header of a block alone, so a word that joins the block above it shows its time and no cost.
      const took = (step: ActRow, costs = true) =>
        step.kind !== "rung" || !isDone(step)
          ? ""
          : [
              // A time under a millisecond is one that this life did not measure, as a step that it replayed.
              (w.took[step.id] ?? 0) < 1 ? "" : spell(w.took[step.id] ?? 0),
              costs && step.spend ? dollars(step.spend.dollars) : "",
            ]
              .filter(Boolean)
              .join(" ");
      const since = (act?: ActRow) => {
        const began = act && w.started[act.id];
        const ms = began ? clock() - began : 0;
        return ms >= 1000 ? spell(Math.floor(ms / 1000) * 1000) : "";
      };
      const programCard = (
        id: string,
        given: readonly ProgramLine[],
        indent: number,
        {
          tone = threadTone(id),
          frame: framed,
          first,
          shown,
        }: { tone?: RGBA; frame?: boolean; first?: string; shown?: string } = {},
      ) => {
        // The numbers of a thread stand in one column: its widest number sets the gutter of each part of its program.
        const digits = String(
          given.reduce((most, line) => ("display" in line ? Math.max(most, line.display) : most), 1),
        ).length;
        // What a model writes changes at each token, so its row stands in a card of its own under the program, which
        // the tick draws again, and the program stays as it was drawn.
        const all = given.filter((line) => line.kind !== "writes");
        const writers = given.filter((line) => line.kind === "writes");
        const tails = () => {
          for (const line of writers) {
            // Each line that the model finished shows as python, highlighted as it comes, and the card is made again
            // at each line and not at each token. The row under it takes the next number and the line still written.
            const { lines } = writing(w.host.streams.get(line.step.id));
            this.followThinking(line.step.id, lines ? lines.split("\n").length : 0);
            if (lines)
              add(
                `wrote-${line.step.id}`,
                lines,
                [],
                (box) => box.add(this.inset(space.between, this.code(lines))),
                { indent, same: [lines] },
              );
          }
        };
        if (!all.length) return tails();
        // A thread whose program made its last steps alone says how many came before them.
        const earlier = all[0]?.kind === "earlier" ? all[0].count : 0;
        const steps = [...new Set(all.filter((line) => line.kind !== "note").map((line) => line.step))];
        const hide = earlier
          ? earlier
          : w.folds[`steps:${id}`] !== false && steps.length > PROGRAM_STEPS
            ? steps.length - PROGRAM_STEPS
            : 0;
        // The notes that came before the first step shown fold with the earlier steps.
        const from = earlier ? 1 : hide ? all.findIndex((line) => line.step === steps[hide]) : 0;
        const program = from ? all.slice(from) : all;
        // A program that a panel frames, as a word of the operator, leaves the room of the bar and the inset of the panel.
        const room = this.feedWidth - indent - 8 - (framed ? space.between + space.inset : 0);
        // A click on a line opens it in place: its whole text, and under it what each act of the line gave. The
        // program parts at each open line, so that the cards of its acts stand under it.
        const segments: ProgramLine[][] = [[]];
        // A mark whose place falls between two steps of the program parts it there.
        const cuts = new Map<number, PartMark[]>();
        let stepped: ActRow | undefined;
        for (const line of program) {
          if (line.kind === "code" && line.step !== stepped) {
            stepped = line.step;
            const marked = due(line.step.began);
            if (marked.length) {
              if (segments.at(-1)?.length) segments.push([]);
              cuts.set(segments.length - 1, [...(cuts.get(segments.length - 1) ?? []), ...marked]);
            }
          }
          segments.at(-1)?.push(line);
          if (this.lineOpen(line)) segments.push([]);
        }
        // The time and the cost of a step stand on its first line alone, in whichever part of the program it falls.
        const heads = new Set<ProgramLine>();
        let before: ActRow | undefined;
        for (const line of program)
          if (line.kind === "code" && line.step !== before) {
            heads.add(line);
            before = line.step;
          }
        for (const [at, segment] of segments.entries()) {
          for (const mark of cuts.get(at) ?? []) showMark(mark);
          if (!segment.length) continue;
          const key = [
            at ? 0 : hide,
            room,
            ...segment.map((line) =>
              line.kind === "code"
                ? `${line.text}|${this.lineHint(line)}|${line.runs ? ((line.awaits?.act ?? line.acts.at(-1))?.id ?? "runs") : ""}|${line.awaited ?? ""}|${line.fails ?? ""}|${took(line.step)}|${this.lineOpen(line)}|${[...this.openStrings(line)]}`
                : `${line.kind}|${line.text}`,
            ),
          ].join("\n");
          add(
            at ? `program-${id}-${at}` : (first ?? `program-${id}`),
            key,
            [],
            (outer) => {
              const box = framed ? this.panel(outer, tone) : this.box({ paddingLeft: space.between });
              if (!framed) outer.add(box);
              if (hide && !at) box.add(this.foldLine(`steps:${id}`, hide, "step", "earlier"));
              box.add(this.programView(segment, room, took, since, heads, digits));
            },
            {
              indent,
              same: [key],
              ...(framed ? { group: "user", heading: false, open: true, separate: true } : {}),
              ...(shown !== undefined && !at ? { shown } : {}),
            },
          );
          const open = segment.at(-1);
          if (open?.kind === "code" && this.lineOpen(open) && open.acts.length)
            add(
              `opened-${id}-${at}`,
              open.acts.map((one) => `${one.id}:${isDone(one)}:${(one as ActRow).output ?? ""}`).join(" ") +
                this.theme,
              [],
              (box) => {
                const inner = this.box({ paddingLeft: space.between * 2, gap: space.stack });
                for (const one of open.acts) this.actDetails(inner, one as ActRow);
                box.add(inner);
              },
              { indent },
            );
        }
        tails();
      };
      // The marks of the parts stand after the last act that began before their place: a rule with the title of the
      // mark, then its lines. A line that only shows more of an act, or a fold of earlier lines, places no mark.
      const moment = w.moment;
      const marks = this.parts.marks(this.seen()).filter((one) => moment === undefined || one.at <= moment);
      const placing = (line: (typeof lines)[number]) =>
        line.type !== "tail" && line.type !== "value" && line.type !== "more";
      // A mark waits until the first act that began at its place or later, and shows before it: before a line of the
      // feed, or inside a program, between two of its steps.
      const pending = [...marks].sort((one, other) => one.at - other.at);
      const due = (began: number) => {
        const out: PartMark[] = [];
        for (const one of [...pending])
          if (one.at <= began) {
            pending.splice(pending.indexOf(one), 1);
            out.push(one);
          }
        return out;
      };
      const markBody = (mark: PartMark) => (box: BoxRenderable) => {
        box.add(
          this.line(
            [
              [`${glyph.rule.repeat(2)} `, c.border],
              [mark.title, c.muted],
              [
                ` ${glyph.rule.repeat(Math.max(0, this.feedWidth - Bun.stringWidth(mark.title) - 8))}`,
                c.border,
              ],
            ],
            c.muted,
          ),
        );
        for (const one of mark.lines ?? [])
          box.add(this.inset(space.between * 2, this.text(this.partLine(one), c.muted)));
      };
      const showMark = (mark: PartMark) => {
        const said = JSON.stringify(mark.lines ?? []);
        add(`mark-${mark.chain}-${mark.at}`, `${mark.title}\n${said}`, [], markBody(mark), {
          same: [mark.title, said],
        });
      };
      // The answers of the threads that the feed shows, which stand in the box of their thread.
      const valued = new Set(lines.filter((one) => one.type === "value").map((one) => one.act.id));
      type Section = {
        id: string;
        key: string;
        label?: Part[];
        body: (box: BoxRenderable) => void;
        /** A section that a blank row parts from the one above it. */
        apart?: boolean;
        /** The color of the line at its left: the color of the thread for a message of the operator. */
        bar?: RGBA;
        same?: readonly unknown[];
        act?: ActRow;
        shown?: string;
      };
      let collapsed = -1;
      // A thread is one box: the message of the operator with a line of the color of the thread at its left, the
      // program of the thread as windows of code, what the model answered, and an input at its end. The sections of
      // the box are cards of their own that share a panel, so a change of the thread makes again only the one that
      // changed, and the input keeps its text. A thread that the operator folded stands as one line.
      const threadBox = (act: ActRow, depth: number, indent: number) => {
        const message = this.spoken(act);
        const user = w.isUserPrompt(act);
        const thread = threadsOwn.by.get(act.id);
        // A question of another chain is no thread: it stands as a block of the chain that asks, with the prompts on
        // it and its answer under it, and offers no reply.
        const aside = threadsOwn.asides.get(act.id);
        const messages = thread?.messages ?? aside?.messages ?? [];
        const tone = threadTone(act.id);
        // A message that a branch took shows what its thread held when the branch began.
        const mine = taken.has(act.id);
        const made = mine ? (takenMade().get(act.id) ?? []) : madeBy(act.id);
        const folded = Boolean(thread && this.threadFolded(act.id));
        if (folded) collapsed = depth;
        // A thread that stands open shows its work, which the operator read once he leaves it. A folded thread shows
        // the work past the place up to which he read it as unread.
        const work = [act, ...made, ...messages, ...(thread?.asked ?? [])];
        if (thread && !folded) w.shows(act.id, work);
        const news = folded && w.unreadThread(act.id, work);
        const steps = made.length + messages.length;
        // A folded thread keeps its header alone, drawn as the open thread draws it, with how many steps it holds.
        // Its state stands beside its shape, as the open thread says it.
        const meta = folded && steps ? `${steps} ${steps === 1 ? "step" : "steps"}` : "";
        const sections: Section[] = [];
        const emit = () => {
          for (const [at, section] of sections.entries()) {
            const join =
              sections.length === 1
                ? "only"
                : at === 0
                  ? "first"
                  : at === sections.length - 1
                    ? "last"
                    : "middle";
            // A message of the operator holds its own half rows of padding, so no blank row parts a part from it.
            const gap = section.apart && at > 0 && !section.bar && !sections[at - 1]?.bar ? space.section : 0;
            add(
              section.id,
              `${section.key}\n${join}\n${gap}\n${Boolean(section.bar)}`,
              section.label ?? [],
              (box) =>
                section.body(
                  this.barred(box, section.bar, gap, { top: at > 0, bottom: at < sections.length - 1 }),
                ),
              {
                indent,
                join,
                ...(thread ? { thread: act.id } : {}),
                ...(section.bar ? { edge: section.bar } : {}),
                heading: false,
                separate: at === 0,
                open: true,
                same: [...(section.same ?? [section.key]), join, gap, Boolean(section.bar)],
                ...(section.act ? { act: section.act } : {}),
                ...(section.shown !== undefined ? { shown: section.shown } : {}),
              },
            );
          }
        };
        // A message to the thread shows as its sender wrote it, with each image that it attached named by a mark.
        const said = (text: string) =>
          imageReferences(text).length ? this.message(text) : this.markdown(text);
        const { sent, state, bar } = this.threadHead(act, folded);
        // A long message shows its first rows until a click opens it, so that a brief of a page leaves the feed
        // readable at a glance. A folded thread shows its first rows even when the operator opened its message.
        const [shortened, rest] =
          (user || w.folds[`message:${act.id}`] === false) && !folded
            ? [message, 0]
            : this.firstRows(message, folded ? 2 : MESSAGE_ROWS);
        const foldedNote = folded ? [meta, rest ? hidden(rest, "line") : ""].filter(Boolean).join("  ") : "";
        sections.push({
          id: act.id,
          label: user ? [["You", tone]] : [[sent, c.faint]],
          key: `${message}\n${sent}\n${plain(state)}\n${rest}\n${thread?.over}\n${meta}\n${news}\n${folded}\n${this.feedWidth}\n${this.theme}`,
          ...(bar ? { bar } : {}),
          act,
          shown: message,
          body: (box) => {
            // A right click on a message opens its menu, as on the heading of any other act.
            const beside: Part[] = [
              ...(news
                ? [[`${markOf("unread")[0]} `, markOf("unread")[1], markOf("unread")[2]] as Part]
                : []),
              ...(sent && !folded ? [[`${sent}  `, c.faint] as Part] : []),
              ...(folded ? state.slice(1) : state),
              ...(foldedNote ? [[`  ${foldedNote}`, c.faint] as Part] : []),
            ];
            // A folded thread stands as its message, the first rows of it, in the block of a message: the bar of the
            // thread at its left, and at its right how much it holds and the toggle that opens it.
            const text = user ? said(shortened) : this.markdown(shortened);
            const toggle = thread && this.threadToggle(act.id);
            const row = this.sided(
              {
                onMouseDown: (event) => {
                  if (event.button === 2)
                    this.actActions(this.session.acts.find((one) => one.id === act.id) ?? act);
                },
              },
              text,
              [this.text(beside, c.faint, { marginLeft: space.between }), toggle],
              Bun.stringWidth(plain(beside)) +
                space.between +
                (toggle ? Bun.stringWidth(toggle.plainText) + space.between : 0),
              this.feedWidth - indent - 8,
              shortened,
            );
            box.add(row);
            if (rest && !folded) box.add(this.foldLine(`message:${act.id}`, rest, "line"));
          },
        });
        if (folded) return emit();
        const parts = [act, ...messages];
        const program = programFor(
          mine ? `taken:${w.selected}:${act.id}` : act.id,
          liveOf(parts.map((one) => one.id)),
          () => threadProgram(parts, mine ? takenMade() : madeMap(), word, owed, named),
        );
        // What a model answered stands as markdown with no line of color, since it is no message of the operator.
        const answerOf = (target: ActRow, id: string): Section => {
          const value =
            typeof outcomeOf(target) === "boolean"
              ? outcomeOf(target)
                ? "yes"
                : "no"
              : display(outcomeOf(target));
          const model = String(target.words[2] || act.words[2] || "");
          return {
            id,
            key: `${value}\n${model}\n${w.folds[`reply:${id}`]}\n${this.feedWidth}\n${this.theme}`,
            apart: true,
            body: (box) => {
              const { name, effort } = this.model(model);
              const row = this.box({ flexDirection: "row" });
              // A column, and not a row, holds the answer, so that its paragraphs wrap at the width of the feed.
              const answer = this.box({ flexGrow: 1, flexShrink: 1 });
              this.bounded(answer, value, `reply:${id}`);
              row.add(answer);
              row.add(
                this.text([[modelLabel(name, effort), c.muted]], c.faint, { marginLeft: space.between }),
              );
              box.add(row);
            },
          };
        };
        // The program parts at each message of the operator that came to the thread: the code before it, the message,
        // and the code after.
        const runs: { lines: ProgramLine[]; writes?: Extract<ProgramLine, { kind: "writes" }> }[] = [
          { lines: [] },
        ];
        const says: ActRow[] = [];
        for (const line of program) {
          if (line.kind === "note" && line.step.kind === "prompt" && operated(line.step.by)) {
            // A message that the operator took back leaves the box, and the message that replaces it stands in its place.
            if (cancelled(line.step)) continue;
            says.push(line.step as ActRow);
            runs.push({ lines: [] });
            continue;
          }
          const run = runs[runs.length - 1];
          if (!run) continue;
          if (line.kind === "writes") run.writes = line;
          else run.lines.push(line);
        }
        const answered = valued.has(act.id) && act.words[0] !== "None";
        let placed = false;
        // The answer of a message stands after the code that its model ran to answer it, which the next run of the
        // program holds.
        let waiting: Section | undefined;
        // The questions and the notices that the thread made stand in its box, in time: after the code that came
        // before them, and before the next message.
        const asked = (thread?.asked ?? []).map((one) => one as ActRow);
        const slot = (ask: ActRow) => says.filter((say) => say.began <= ask.began).length;
        const askSection = (ask: ActRow): Section => {
          const text = String(ask.words[1] ?? "");
          const notice = ask.words[0] === "None";
          // A notice is unread until a fact seen of its thread or of the chain that the operator left covers it.
          const unseen = notice && w.unreadThread(act.id, [ask]);
          const pending = w.desk.asks.find((one) => one.id === ask.id);
          const aimed = Boolean(pending) && w.aimed?.id === ask.id;
          const value =
            typeof outcomeOf(ask) === "boolean" ? (outcomeOf(ask) ? "yes" : "no") : display(outcomeOf(ask));
          const short = !value.includes("\n") && Bun.stringWidth(value) < this.feedWidth - 24 - indent;
          // A long notice shows its first rows until a click opens it, as a long message does.
          const [shortened, rest] =
            notice && w.folds[`message:${ask.id}`] !== false ? this.firstRows(text, MESSAGE_ROWS) : [text, 0];
          return {
            id: `${act.id}-ask-${ask.id}`,
            key: `${text}\n${notice}\n${unseen}\n${Boolean(pending)}\n${aimed}\n${isDone(ask) ? value : ""}\n${rest}\n${this.feedWidth}\n${this.theme}`,
            apart: true,
            act: ask,
            shown: text,
            body: (box) => {
              box.add(
                this.text(
                  [
                    notice
                      ? [`${markOf(unseen ? "unread" : "seen")[0]} `, markOf(unseen ? "unread" : "seen")[1]]
                      : [`${glyph.asks} `, pending ? c.warning : c.faint],
                    [notice ? "Notice" : "Question", notice ? c.muted : c.text, bold],
                  ],
                  c.text,
                ),
              );
              box.add(this.markdown(shortened));
              if (rest) box.add(this.foldLine(`message:${ask.id}`, rest, "line"));
              // The question that the input answers says so, and a click on another one points the input at it.
              if (pending)
                box.add(
                  this.text(
                    aimed
                      ? this.answeredBelow(pending)
                      : [["Click here to answer this question in the input below", c.muted]],
                    c.muted,
                    {
                      run: () =>
                        aimed ? this.question(pending) : void w.point(pending.id).catch(this.report),
                    },
                  ),
                );
              else if (isDone(ask) && !notice)
                box.add(
                  short
                    ? this.text(
                        [
                          ["You answered  ", c.faint],
                          [value, c.muted],
                        ],
                        c.muted,
                      )
                    : (() => {
                        // A long answer wears the mark of a short one, so that it reads as his and no line of the question.
                        const answer = this.box({});
                        answer.add(
                          this.text(
                            [
                              [`${glyph.done} `, c.success],
                              ["You answered", c.text, bold],
                            ],
                            c.text,
                          ),
                        );
                        answer.add(this.inset(space.between, this.markdown(value)));
                        return answer;
                      })(),
                );
            },
          };
        };
        for (const [index, run] of runs.entries()) {
          const writer = run.writes;
          let given = run.lines;
          let wrote = 0;
          if (writer) {
            // The lines that the model finished stand in the window as lines of the thread, numbered as they come.
            const { lines: done, numbers, count } = writing(w.host.streams.get(writer.step.id));
            const finished = done ? done.split("\n") : [];
            wrote = count;
            given = [
              ...given,
              ...finished.map(
                (text, at): ProgramLine => ({
                  kind: "code",
                  step: writer.step,
                  number: numbers[at] ?? at + 1,
                  display: writer.display + (numbers[at] ?? at + 1) - 1,
                  text,
                  acts: [],
                  runs: false,
                  hint: "",
                }),
              ),
            ];
          }
          if (given.length || writer) {
            const where = `${act.id}:${index}`;
            const window = this.rolling(given, where, Boolean(writer));
            // The numbers of a thread stand in one column: its widest number sets the gutter.
            const digits = String(
              given.reduce(
                (most, line) => ("display" in line ? Math.max(most, line.display) : most),
                writer ? writer.display + wrote : 1,
              ),
            ).length;
            // The panel of the thread and its line take 5 columns, and the numbers about 5.
            const room = this.feedWidth - indent - 13;
            // A click on a line opens it in place: its whole text, and under it what each act of the line gave. The
            // window parts at each open line, so that the cards of its acts stand under it.
            const segments: ProgramLine[][] = [[]];
            const cuts = new Map<number, PartMark[]>();
            let stepped: ActRow | undefined;
            for (const line of window.lines) {
              if (line.kind === "code" && line.step !== stepped) {
                stepped = line.step as ActRow;
                const marked = due(line.step.began);
                if (marked.length) {
                  if (segments.at(-1)?.length) segments.push([]);
                  cuts.set(segments.length - 1, [...(cuts.get(segments.length - 1) ?? []), ...marked]);
                }
              }
              segments.at(-1)?.push(line);
              if (this.lineOpen(line)) segments.push([]);
            }
            // The time and the cost of a step stand on its first line alone.
            const heads = new Set<ProgramLine>();
            let before: ActRow | undefined;
            for (const line of window.lines)
              if (line.kind === "code" && line.step !== before) {
                heads.add(line);
                before = line.step as ActRow;
              }
            for (const [at, segment] of segments.entries()) {
              for (const mark of cuts.get(at) ?? []) {
                const markLines = JSON.stringify(mark.lines ?? []);
                sections.push({
                  id: `mark-${mark.chain}-${mark.at}`,
                  key: `${mark.title}\n${markLines}`,
                  same: [mark.title, markLines],
                  apart: true,
                  body: markBody(mark),
                });
              }
              const thinks = Boolean(writer) && at === segments.length - 1;
              if (!segment.length && !thinks) continue;
              const key = [
                window.opened,
                room,
                digits,
                writer && thinks ? writer.display + wrote : "",
                // The row of a model gains the line of its thought when the first token of it comes.
                writer && thinks ? Boolean(w.host.streams.get(writer.step.id)?.thinking.trim()) : "",
                ...segment.map((line) =>
                  line.kind === "code"
                    ? `${line.text}|${this.lineHint(line)}|${line.runs ? ((line.awaits?.act ?? line.acts.at(-1))?.id ?? "runs") : ""}|${line.awaited ?? ""}|${line.fails ?? ""}|${took(line.step)}|${this.lineOpen(line)}|${[...this.openStrings(line)]}`
                    : `${line.kind}|${line.text}`,
                ),
              ].join("\n");
              sections.push({
                id: index || at ? `program-${act.id}-${index}-${at}` : `program-${act.id}`,
                key,
                // A blank row parts the code from the message above it, as the edge of the box parts the message from
                // the top of the box.
                apart: at === 0,
                body: (box) => {
                  // The code stands on the one surface of the thread, as a code block of the markdown does but with no
                  // surface of its own, and the numbers of its lines stand at its left.
                  const block = this.box({});
                  if (segment.length) {
                    const view = this.programView(segment, room, took, since, heads, digits, {
                      more: () => this.openWindow(where),
                      key: `window:${where}`,
                    });
                    block.add(
                      window.opened
                        ? this.document(view, {
                            // A scroll box takes all the rows that it may, so it takes the rows of its lines.
                            height: Math.max(1, Math.min(segment.length, SCROLL)),
                            flexGrow: 0,
                            flexShrink: 1,
                            minWidth: 0,
                          })
                        : view,
                    );
                  }
                  if (thinks && writer)
                    block.add(this.thinkingRow(writer.step.id, writer.display, digits, room));
                  box.add(block);
                  if (window.opened && at === segments.length - 1)
                    box.add(
                      this.inset(
                        space.inset,
                        this.link(
                          [
                            [`${glyph.open} `, c.faint],
                            ["Roll up", c.muted],
                          ],
                          () => this.openWindow(where),
                        ),
                      ),
                    );
                },
              });
              const open = segment.at(-1);
              if (open?.kind === "code" && this.lineOpen(open) && open.acts.length)
                sections.push({
                  id: `${act.id}-opened-${index}-${at}`,
                  key: `${open.acts.map((one) => `${one.id}:${isDone(one)}:${(one as ActRow).output ?? ""}`).join(" ")}${this.theme}`,
                  body: (box) => {
                    const inner = this.box({ paddingLeft: space.between * 2, gap: space.stack });
                    for (const one of open.acts) this.actDetails(inner, one as ActRow);
                    box.add(inner);
                  },
                });
            }
          }
          if (waiting) sections.push(waiting);
          waiting = undefined;
          for (const ask of asked) if (slot(ask) === index) sections.push(askSection(ask));
          const next = says[index];
          if (next) {
            // The answer of the thread stands where it came in time: before a message that came after it and opened the
            // thread again, so each message stands with its own answer.
            if (answered && !placed && act.ended != null && next.began > act.ended) {
              sections.push(answerOf(act, `${act.id}-answer`));
              placed = true;
            }
            const text = String(next.words[1] ?? "");
            sections.push({
              id: `${act.id}-say-${next.id}`,
              key: `${text}\n${this.feedWidth}\n${this.theme}`,
              apart: true,
              bar: tone,
              body: (box) => {
                // A right click on a later message opens its menu too, as on the first message of the thread.
                const row = this.box({
                  onMouseDown: (event) => {
                    if (event.button === 2)
                      this.actActions(this.session.acts.find((one) => one.id === next.id) ?? next);
                  },
                });
                row.add(said(text));
                box.add(row);
              },
            });
            // A message of a shape that asks an answer stands with its answer, after the code that answered it, so that
            // a thread holds pairs of a message and its answer.
            if (isDone(next) && next.words[0] !== "None" && !cancelled(next))
              waiting = answerOf(next, `${act.id}-say-${next.id}-answer`);
          }
        }
        if (waiting) sections.push(waiting);
        // What the model answered stands last, when no message came after it.
        if (answered && !placed) sections.push(answerOf(act, `${act.id}-answer`));
        // A message that a branch took is answered on its source, so the branch offers no reply to it.
        if (!w.host.reading && !taken.has(act.id) && !aside) {
          const over = Boolean(thread?.over);
          sections.push({
            id: `${act.id}-reply`,
            key: `reply\n${over}\n${w.target === act.id}\n${this.feedWidth}\n${this.theme}`,
            apart: true,
            body: (box) => this.replyRow(box, act.id, over, tone),
          });
        }
        emit();
      };
      for (const line of lines) {
        if (collapsed >= 0 && line.depth > collapsed) continue;
        collapsed = -1;
        // The steps of a thread stand in its program, and its messages as comments there. What a thread closed with
        // and a question that waits for the operator stand on their own.
        if (
          line.depth > 0 &&
          !(line.type === "value" && threadsOwn.by.has(line.act.id)) &&
          !(line.type === "prompt" && waiting.has(line.act.id))
        )
          continue;
        if (line.type === "prompt" && threadsOwn.of.has(line.act.id)) continue;
        // A prompt on an aside stands in the block of that aside, and the answer of an aside in its block too.
        if (line.type === "prompt" && threadsOwn.aside.has(line.act.id)) continue;
        if (line.type === "value" && threadsOwn.asides.has(line.act.id)) continue;
        // A question or a notice that a thread made stands in the box of that thread, with its answer.
        if (threadsOwn.asked.has(line.act.id)) continue;
        // The answer of a thread stands in its box.
        if (line.type === "value" && threadsOwn.by.has(line.act.id)) continue;
        // Only a line that the feed shows places a mark: a step under a step stands folded, whenever it began.
        if (placing(line)) for (const mark of due(line.act.began)) showMark(mark);
        const depth = line.depth;
        const act = line.act as ActRow;
        const indent = line.depth * space.between;
        if (line.type === "rung") {
          const code = this.subject(act);
          // The words that a chain wrote to retell its source stand as one line, which names the source.
          if (rows.get(act.by)?.kind === "chain") {
            if (retold) continue;
            retold = true;
            const source = String(rows.get(act.by)?.words[1] ?? "");
            // The branch says what it takes of its source: each message before the moment it starts at, which the
            // word that made it does not leave out.
            const made = rows.get(act.by);
            const maker = made && (w.actOf(String(made.by)) ?? made);
            const messages =
              act.by === w.selected && source
                ? taken.size
                : maker &&
                  branchMessages(w.acts, source, maker, this.subject(maker), (one) => w.isUserPrompt(one));
            const name = w.labelOf(source);
            if (source)
              add(
                `retold-${act.by}`,
                `${source}:${name}:${messages}:${w.watching}:${this.theme}`,
                [],
                (box) => {
                  box.add(
                    this.link(
                      w.watching
                        ? [
                            ["↳ ", c.secondary],
                            [branchStart(name), c.muted],
                          ]
                        : [
                            ["↳ ", c.secondary],
                            ["Branched from ", c.muted],
                            [name, c.text, bold],
                          ],
                      () => void w.select(source).catch(w.fail),
                    ),
                  );
                  if (messages !== undefined)
                    box.add(this.text(branchTakes(messages), c.faint, { paddingLeft: space.between }));
                },
              );
            continue;
          }
          // A word of the operator that only made a chain, as a branch or a new chain does, reads as the chain that it
          // made, and a click on it opens that chain.
          const made =
            operated(act.by) && /^\s*chain\(/.test(code)
              ? w.chains.find((chain) => chain.by === act.id)
              : undefined;
          if (made) {
            const name = w.labelOf(made.id);
            const here = made.id === w.selected;
            add(
              act.id,
              `made:${made.id}:${name}:${here}:${this.theme}`,
              [],
              (box) =>
                box.add(
                  here
                    ? this.text([
                        ["↳ ", c.secondary],
                        ["This branch starts here", c.muted],
                      ])
                    : this.link(
                        [
                          ["↳ ", c.secondary],
                          [made.words[1] ? "Branched to " : "Started the chain ", c.muted],
                          [name, c.text, bold],
                        ],
                        () => void w.select(made.id).catch(w.fail),
                      ),
                ),
              { shown: code },
            );
            continue;
          }
          // A rung folds to one row: its state, the first line of its word, and the acts that it made. Open, it shows
          // its whole word, and each act that it made stands under it.
          // A rung that a cancel ended before its model wrote a word shows nothing, since the message says the cancel.
          if (!code.trim() && cancelled(act)) continue;
          // A word of the operator stands as his messages stand: its program in a panel of his color.
          programCard(
            act.id,
            programFor(act.id, stepOf(act), () =>
              stepLines(act, madeMap(), word, owed, named, w.host.streams),
            ),
            indent,
            { frame: true, first: act.id, shown: code },
          );
          // A word of the operator that the gate refused says what to do under it.
          const hint = this.gateHint(act);
          if (hint)
            add(`hint-${act.id}`, `${hint}:${this.theme}`, [], (box) =>
              box.add(this.inset(space.between, this.text(hint, c.muted))),
            );
        } else if (line.type === "tail") {
          const maker = rows.get(act.by);
          if (line.model) {
            // A model shows the last lines that it writes, or of its thinking until it writes, with a cursor.
            const thinking = !w.host.streams.get(act.id)?.text;
            add(`stream-${act.id}`, line.text, [], (box) => {
              // A column holds the lines, so that a long line wraps at the width of the feed.
              const tail = this.box({ paddingLeft: indent + space.between });
              tail.add(
                this.text(
                  [
                    [line.text, thinking ? c.faint : c.muted, thinking ? italic : 0],
                    [glyph.mark, c.accent],
                  ],
                  c.muted,
                ),
              );
              box.add(tail);
            });
          } else if (maker?.kind === "rung" && this.folded(maker.id, maker))
            // A command that runs shows its last lines under the folded rung that made it.
            add(
              `tail-${act.id}`,
              `${line.text}\n${w.folds[`tail:${act.id}`]}`,
              [],
              (box) => this.excerpt(box, line.text, `tail:${act.id}`),
              { indent },
            );
        } else if (line.type === "value") {
          // A notice closes with nothing, which its panel says already.
          if (act.words[0] === "None") continue;
          const value =
            typeof outcomeOf(act) === "boolean" ? (outcomeOf(act) ? "yes" : "no") : display(outcomeOf(act));
          const answered = asksOperator(act);
          const short = !value.includes("\n") && Bun.stringWidth(value) < this.feedWidth - 20 - indent;
          // The answer of a model closes its thread as the thread opened: a panel of its color, with the model and its
          // effort at its right.
          const tone = threadTone(threadsOwn.of.get(act.id)?.act.id ?? act.id);
          add(
            `value-${act.id}`,
            `${value}\n${short}\n${answered}\n${w.folds[`reply:${act.id}`]}\n${this.feedWidth}\n${this.theme}`,
            answered
              ? [
                  [`${glyph.done} `, c.success],
                  ["You answered", c.text, bold],
                  [short ? `  ${value}` : "", c.muted],
                ]
              : [],
            (box) => {
              if (answered && short) return;
              if (answered) {
                const answer = this.box({ paddingLeft: space.between });
                answer.add(this.markdown(value));
                box.add(answer);
                return;
              }
              const { name, effort } = this.model(String(act.words[2] ?? ""));
              const row = this.box({ flexDirection: "row" });
              // A column, and not a row, holds the answer, so that its paragraphs wrap at the width of the feed.
              const answer = this.box({ flexGrow: 1, flexShrink: 1 });
              this.bounded(answer, value, `reply:${act.id}`);
              row.add(answer);
              row.add(
                this.text([[modelLabel(name, effort), c.muted]], c.faint, { marginLeft: space.between }),
              );
              this.panel(box, tone).add(row);
            },
            {
              group: answered ? "question" : "assistant",
              act,
              heading: true,
              open: !answered,
              indent: indent - space.between,
              shown: value,
            },
          );
        } else if (line.type === "ear") {
          // The rungs that an ear wrote, as the words of the extensions, or that a chain wrote on itself, as the setup
          // of the root, stand as code: a comment that names the ear or the setup, then the calls that end each word,
          // with what its acts came to. A word that only defines shows no line.
          const ear = act;
          const earLines = programFor(
            `ear:${act.id}`,
            `${line.state}:${line.acts.map((chip) => chip.label + chip.state).join(" ")}`,
            () => {
              return earProgram(
                ear,
                line.rungs,
                line.acts,
                madeBy,
                (rung) => this.subject(rung),
                owed,
                named,
              );
            },
          );
          // An ear with no rung shows nothing.
          if (line.rungs.length)
            programCard(act.id, earLines, indent, {
              shown: `${act.by} ${line.rungs.map((rung) => rung.id).join(" ")}`,
            });
        } else if (line.type === "more") {
          // The words that retell the source of a chain stand as the one line that names the source, folded or not.
          if (line.act.kind === "rung" && rows.get(line.act.by)?.kind === "chain") continue;
          // The earlier prompts of a long thread, or the earlier rungs of a long ladder, stand as one line until a click
          // opens them.
          const { key, count, heads } = line;
          // The line names what it holds: messages, words, or steps of both.
          const noun =
            heads.length === 1 && heads[0] === "rung"
              ? "rung"
              : heads.length === 1 && heads[0] === "prompt"
                ? "message"
                : "step";
          add(
            `more-${key}`,
            `${key}:${count}:${this.theme}`,
            [],
            (box) => box.add(this.foldLine(key, count, noun, "earlier")),
            {
              indent,
            },
          );
        } else if (line.type === "act") {
          // The ceiling of a grant stands in the sidebar, so the feed shows no grant. An act that no word made stands as
          // a line of python under a quiet header, as a word that the operator runs.
          if (act.kind === "grant") continue;
          programCard(
            act.id,
            programFor(`act:${act.id}`, String(w.revision(act.id)), () => actLines(act, owed, named)),
            indent,
            { frame: true, first: act.id, shown: act.id },
          );
        } else if (
          (asksOperator(act) && act.words[0] === "None") ||
          (threadsOwn.asides.has(act.id) && threadsOwn.asides.get(act.id)?.aside.aside !== "question")
        ) {
          // A prompt of None asks nothing: the console closed it as it took it. It stands as a message of the
          // operator stands, a panel of its color, on its first line until a click opens the whole message. A notice
          // of the round, what a word of the operator told the chain, and a wake stand the same, as no thread.
          const aside = threadsOwn.asides.get(act.id)?.aside.aside;
          const message = aside ? this.spoken(act) : String(act.words[1] ?? "");
          const from =
            aside === "wake"
              ? `wake from ${act.by}`
              : aside === "rung"
                ? "your word"
                : act.by === "operator"
                  ? ""
                  : `from ${act.by}`;
          const opened = w.folds[act.id] === false;
          const lines = message.split("\n").filter((one) => one.trim()).length;
          const more = !opened && lines > 1 ? `+${lines - 1} ${lines === 2 ? "line" : "lines"}` : "";
          const meta = [more, from].filter(Boolean).join("  ");
          const room = this.feedWidth - indent - 14 - Bun.stringWidth(meta);
          const tone = threadTone(act.id);
          // A notice shows ● until a fact seen of its chain covers it, and a faint ✓ once one does.
          const unread = w.unreadThread(act.id, [act]);
          const [mark, ink, weight] = markOf(unread ? "unread" : "seen");
          const toggle = () => {
            w.folds[act.id] = opened;
            w.save();
            this.renderContent();
          };
          const note: Note = {
            act,
            indent,
            tone,
            shown: message,
            key: `${message}\n${opened}\n${mark}\n${this.feedWidth}\n${this.theme}`,
            row: () => {
              const row = this.box({ flexDirection: "row" });
              const text = opened
                ? this.markdown(message)
                : this.link([[clip(shortenHomes(gist(message)), room), c.muted]], toggle);
              text.flexGrow = 1;
              text.flexShrink = 1;
              row.add(text);
              if (meta) row.add(this.text(meta, c.faint, { marginLeft: space.between, run: toggle }));
              row.add(this.text([[mark, ink, weight]], ink, { marginLeft: space.between }));
              return row;
            },
          };
          // A prompt that the host names a note stands in the run that the view gives it, which draws once its last note
          // came. A prompt of None that no aside names stands alone, as the console closed it.
          const of = aside && aside !== "question" ? threadsOwn.runs.get(act.id) : undefined;
          if (of) {
            if (held !== of) flushNotes();
            held = of;
            run.push(note);
            if (of.prompts.at(-1)?.id === act.id) flushNotes();
          } else {
            flushNotes();
            drawNotes([note]);
          }
        } else if (asksOperator(act)) {
          const message = String(act.words[1] ?? "");
          const ask = w.desk.asks.find((one) => one.id === act.id);
          const waiting = Boolean(ask);
          const aimed = waiting && w.aimed?.id === act.id;
          add(
            act.id,
            `${message}\n${waiting}\n${aimed}`,
            [
              [`${glyph.asks} `, waiting ? c.warning : c.faint],
              ["Question", c.text, bold],
              [`  ${act.by === "operator" ? "for you" : `from ${act.by}`}`, c.faint],
            ],
            (box) => {
              this.panel(box, waiting ? c.warning : c.border).add(this.markdown(message));
              // The question that the input answers says so, and a click on another one points the input at it.
              if (ask)
                box.add(
                  this.inset(
                    space.between,
                    this.text(
                      aimed
                        ? this.answeredBelow(ask)
                        : [["Click here to answer this question in the input below", c.muted]],
                      c.muted,
                      { run: () => (aimed ? this.question(ask) : void w.point(ask.id).catch(this.report)) },
                    ),
                  ),
                );
            },
            { group: "question", act, shown: message },
          );
        } else threadBox(act, depth, indent);
      }
      flushNotes();
      for (const mark of pending.splice(0)) showMark(mark);
      // A paused chain says so at the end of its feed, with the cause that its pause gave, and a button that
      // wakes it, since nothing new runs on it until then.
      if (w.paused && !w.search) {
        // The pause says its own cause: the act that paused the chain and its error, as the fact pause carries them.
        const said = w.cause ? pauseSaid(w.cause) : undefined;
        const failure = said?.text;
        add("paused", `paused:${failure ?? ""}:${this.theme}`, [], (box) =>
          this.banner(
            box,
            c.warning,
            [
              [`${glyph.held} `, c.warning],
              ["This chain is paused", c.text, bold],
              ["  New work waits for a wake.", c.muted],
            ],
            failure
              ? this.whole(
                  this.text(
                    clip(shortenHomes(failure).split("\n")[0] ?? "", this.feedWidth - 8),
                    said?.error ? c.danger : c.muted,
                  ),
                  () => shortenHomes(failure),
                )
              : undefined,
            ["Wake", "runs what waits, once the cause is fixed", () => this.action("/wake")],
          ),
        );
        items++;
      }
      if (!items && !w.search && !w.loading && !w.error) {
        const { width, height } = this.scroll.viewport;
        add("welcome", `welcome:${width}:${height}:${this.theme}:${this.welcomeKey()}`, [], (box) =>
          this.welcome(box, height),
        );
        items++;
      }
    } else if (w.view === "transcript") {
      w.turns.forEach((turn, index) => {
        const text = turn[1];
        add(
          `transcript-${index}`,
          text,
          [
            [
              // A turn takes no mark of a state: the model speaks, or the chain tells it.
              turn[0] === "assistant" ? `${glyph.chip} ` : `${glyph.prompt} `,
              turn[0] === "assistant" ? c.secondary : c.faint,
            ],
            [turn[0], c.text, bold],
            [`  turn ${index + 1}`, c.faint],
          ],
          (box) => {
            const inner = this.box({ paddingLeft: space.between });
            inner.add(this.turnBody(text));
            box.add(inner);
          },
          { group: turn[0], shown: text },
        );
      });
    } else if (w.view === "changes") {
      const root = w.host.directory;
      const relative = (path: string) => fromDirectory(root, path);
      // The scopes of the review stand first, and in the middle of an empty view under why it is empty.
      if (w.changes.length)
        add("change-scopes", `${w.scope}:${this.theme}`, [], (box) => box.add(this.scopes()));
      const counts = (added: number, removed: number): Part[] => [
        [`+${added}`, added ? c.success : c.faint],
        [` -${removed}`, removed ? c.danger : c.faint],
      ];
      if (w.changes.length) {
        // The list of the files: each with the lines that it added and removed, and a click that goes to its diff.
        const added = w.changes.reduce((sum, change) => sum + change.added, 0);
        const removed = w.changes.reduce((sum, change) => sum + change.removed, 0);
        const files = w.changes.length === 1 ? "1 file" : `${w.changes.length} files`;
        add(
          "change-files",
          `${w.scope}:${this.theme}:${w.changes.map((change) => `${change.path}:${change.added}:${change.removed}`).join()}`,
          [[`${files}  `, c.text, bold], ...counts(added, removed)],
          (box) => {
            const width = Math.max(...w.changes.map((change) => Bun.stringWidth(relative(change.path))));
            for (const change of w.changes) {
              const path = relative(change.path);
              const row = this.row({ paddingLeft: space.between });
              row.add(this.link(path, () => this.jumpTo(`change:${path}`)));
              row.add(
                this.text([
                  [" ".repeat(width - Bun.stringWidth(path) + 2), c.text],
                  ...counts(change.added, change.removed),
                ]),
              );
              box.add(row);
            }
          },
        );
      }
      for (const change of w.changes) {
        const path = relative(change.path);
        // The heading of a change is a bar of the panel: the path, what the change did to the file, and the lines it
        // added and removed at its right.
        const [what, color] = !change.before
          ? ["created", c.success]
          : !change.after
            ? ["deleted", c.danger]
            : ["modified", c.warning];
        const tail = `+${change.added} -${change.removed} `;
        const fill = Math.max(
          1,
          this.feedWidth - Bun.stringWidth(` ${glyph.chip} ${path}  ${what}`) - Bun.stringWidth(tail),
        );
        // Two sides need room for two lines of code side by side, and one side reads better below that.
        const split = this.renderer.width >= 140;
        add(
          `change:${path}`,
          `${w.scope}:${change.before.length}:${change.after.length}:${change.added}:${change.removed}:${split}:${this.theme}`,
          [
            [` ${glyph.chip} `, color, 0, c.panel],
            [path, c.text, bold, c.panel],
            [`  ${what}`, c.muted, 0, c.panel],
            [" ".repeat(fill), c.text, 0, c.panel],
            [`+${change.added}`, change.added ? c.success : c.faint, 0, c.panel],
            [` -${change.removed} `, change.removed ? c.danger : c.faint, 0, c.panel],
          ],
          (box) => {
            for (const hunk of change.hunks) {
              // Each hunk has a bar: where it stands in both files, a revert through the World, and a comment on
              // one of its lines, which goes to the chain as a prompt.
              const bar = this.row({ gap: space.between, paddingLeft: space.between });
              bar.add(
                this.text(
                  `@@ -${hunk.oldStart},${hunk.oldLines} +${hunk.newStart},${hunk.newLines} @@`,
                  c.faint,
                ),
              );
              if (!w.host.reading) {
                bar.add(this.link("Revert", () => void w.revert(change, hunk).catch(this.report)));
                bar.add(this.link("Comment a line", () => this.commentLine(path, hunk)));
              }
              box.add(bar);
              box.add(
                new DiffRenderable(this.renderer, {
                  diff: hunkPatch(path, hunk),
                  view: split ? "split" : "unified",
                  syntaxStyle: this.style,
                  fg: c.text,
                  showLineNumbers: true,
                  lineNumberFg: c.faint,
                  lineNumberBg: c.background,
                  contextBg: c.background,
                  addedBg: c.added,
                  removedBg: c.removed,
                  addedSignColor: c.success,
                  removedSignColor: c.danger,
                  wrapMode: "word",
                }),
              );
            }
          },
          { shown: change.path },
        );
      }
    }
    // A view with nothing to show says why in the middle of the feed, and what brings something to it.
    const { height } = this.scroll.viewport;
    if (!items && w.loading && !w.error) {
      const loading = (): Part[] => [
        [`${spin()} `, c.accent],
        [`Loading the ${viewLabels[w.view].toLowerCase()}`, c.muted],
      ];
      add(
        "view-loading",
        `${w.view}:${height}`,
        [],
        (box) => this.centered(box, height, this.text(loading())),
        {
          title: loading,
          heading: false,
        },
      );
    }
    if (!items && !w.loading && !w.error) {
      const empty: Record<View, [string, string]> = {
        feed: ["Nothing here yet", "Send a message, or run Python with ⌃R."],
        transcript: ["No transcript yet", "The model reads its first turn here once a prompt runs."],
        changes: (
          {
            turn: [
              "No file changes in this turn",
              "Each file that this chain writes after your last message shows here.",
            ],
            chain: [
              "No file changes on this chain",
              "Each file that this chain writes shows here as a diff.",
            ],
            life: ["No file changes yet", "Each file that the life writes shows here as a diff."],
            branch: [
              "No change on this branch",
              "Each file that differs from main in git shows here as a diff.",
            ],
          } satisfies Record<Scope, [string, string]>
        )[w.scope],
      };
      const [title, hint] = w.search
        ? [`Nothing matches “${w.search}”`, "Change the filter, or press Esc to clear it."]
        : w.view === "changes" && this.unrecordedChanges
          ? [
              "No file-change snapshots are available",
              "The Feed still shows the writes that the record kept.",
            ]
          : empty[w.view];
      // An empty review offers its other scopes under its hint, where the eye already is.
      const scoped = w.view === "changes" && !w.search;
      add("empty", `${title}:${height}:${this.theme}:${scoped && w.scope}`, [], (box) =>
        this.centered(
          box,
          height,
          this.text([
            [`${glyph.ring} `, c.faint],
            [title, c.text, bold],
          ]),
          this.text(hint, c.muted),
          ...(scoped ? [this.scopes()] : []),
        ),
      );
    }
    for (const id of existing) {
      this.cards.get(id)?.node.destroyRecursively();
      this.cards.delete(id);
    }
  },
});

export type FeedPart = typeof feed;
