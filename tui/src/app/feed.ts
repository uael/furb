import { type BoxRenderable, DiffRenderable } from "@opentui/core";
import { display } from "furb";
import {
  actLines,
  asksOperator,
  cancelled,
  condense,
  gist,
  ladder,
  makers,
  type ProgramLine,
  stepLines,
  thread,
  threadProgram,
  wants,
  writing,
} from "furb/life";
import { fromDirectory, shortenHomes } from "../files.ts";
import { clip, dollars, plainFailure } from "../format.ts";
import type { Mark as PartMark } from "../parts.ts";
import { hunkPatch, type Scope } from "../review.ts";
import type { ActRow, View } from "../session.ts";
import { theme as c, glyph, spacing as space, spin, threadTone } from "../theme.ts";
import { bold, italic, type Part, plain } from "../ui.ts";
import { branchMessages, branchStart, branchTakes, earlierSteps } from "../watching.ts";
import { part } from "./part.ts";
import { type BlockOptions, MESSAGE_ROWS, PROGRAM_STEPS, spell, threadMarks, viewLabels } from "./shared.ts";

export const feed = part({
  renderContent(): void {
    const w = this.session;
    const view = this.tree ? "tree" : `${w.selected}:${w.view}`;
    this.treeBar.visible = Boolean(this.tree);
    if (this.lastView !== view) {
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
    if (w.error && w.view === "changes" && w.damagedJournal)
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
          this.text(w.damagedJournal, c.muted),
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
          this.text(plainFailure(w.error), c.muted),
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
    this.welcoming =
      w.view === "feed" &&
      !w.search &&
      !w.loading &&
      !w.error &&
      w.moment === undefined &&
      !w.paused &&
      !w.actOf(w.selected)?.words[1] &&
      !w.activity.some((act) => act.by === "operator" || act.kind === "prompt");
    if (this.welcoming) {
      const { width, height } = this.scroll.viewport;
      add("welcome", `welcome:${width}:${height}:${this.theme}:${this.welcomeKey()}`, [], (box) =>
        this.welcome(box, height),
      );
      items++;
    } else if (w.view === "feed") {
      // The feed shows the acts as they stood at the moment that the operator moved to, or as they stand now.
      const shown = w.at(w.acts);
      // The acts of now are found by the index of the session, and those of a moment of the past by a map of their own.
      const then = shown === w.acts ? undefined : new Map(shown.map((one) => [one.id, one]));
      const rows = { get: (id: string) => (then ? then.get(id) : w.actOf(id)) };
      // The acts of now are read from the index of the session, which spares a pass over every act in each frame, and
      // those of a moment of the past from an index of their own.
      const index = then ? undefined : w.ladderIndex;
      // The acts that each act made, in the order of the acts, which each open rung reads once the first one asks.
      let made: ReadonlyMap<string, readonly ActRow[]> | undefined = index?.made;
      const madeBy = (id: string): readonly ActRow[] => {
        made ??= makers(shown);
        return made?.get(id) ?? [];
      };
      // The feed of a chain is its ladder, which the model of a life gives every host: each prompt with the rungs of
      // its ladder, the acts of each rung, and what the prompt closed with.
      // A fold that the operator opened shows the earlier prompts of the thread, or the earlier rungs of a ladder.
      const opened = new Set(
        Object.entries(w.folds).flatMap(([key, closed]) =>
          closed === false && /^(prompts|rungs):/.test(key) ? [key] : [],
        ),
      );
      // The thread of now is read from the index, in the time of what it shows, and that of a moment of the past whole.
      const owed = new Set(w.host.prompts.keys());
      const word = (rung: ActRow) => this.subject(rung);
      // A run of compactions into new chains shows as one feed: the feed of each chain of the run in its order, which
      // the rule of each compaction parts.
      const run = w.runOf(w.selected);
      const continuing = new Set(run.slice(1));
      const lines = run.flatMap((chain) =>
        index && chain === w.selected
          ? thread(chain, word, w.host.streams, owed, opened, index)
          : condense(ladder(chain, shown, word, w.host.streams, owed), opened),
      );
      let retold = false;
      // The feed is one program of threads: each thread is its header, then the words of its steps as python, with
      // what each line came to as a quiet comment beside it. A thread that the operator collapsed shows its header
      // line alone.
      const threadsOwn = this.chainThreadsBy(w.selected);
      const waiting = new Set(w.desk.asks.map((ask) => ask.id));
      const named = (chain: string) => w.labelOf(chain);
      const madeMap = () => {
        made ??= makers(shown);
        return made;
      };
      // A program is made again only when what it shows changed: a step, its word, its state, or an act of it, as its
      // state and what its command printed. A new act on another thread leaves it as it was.
      const stepOf = (rung: ActRow) => {
        const acts = madeBy(rung.id);
        let done = 0;
        let printed = 0;
        for (const one of acts) {
          if (one.done) done++;
          printed += one.output ?? 0;
        }
        // The word of a step that runs can still come, and the word of a step that ended stays as it is.
        const said = rung.done ? "" : (w.program[rung.id]?.length ?? 0);
        return `${rung.id}:${rung.done}:${rung.run?.status ?? ""}:${said}:${acts.length}:${done}:${printed}`;
      };
      const liveOf = (ids: readonly string[]) =>
        ids
          .map((id) =>
            [
              id,
              ...madeBy(id)
                .filter((one) => one.kind === "rung")
                .map(stepOf),
            ].join(" "),
          )
          .join("|");
      const programFor = (key: string, live: string, make: () => ProgramLine[]): ProgramLine[] => {
        const known = this.programs.get(key);
        if (!then && known?.version !== -1 && known?.live === live) return known.lines;
        const made = make();
        if (this.programs.size > 2000) this.programs.clear();
        this.programs.set(key, { version: then ? -1 : w.version, live, lines: made });
        return made;
      };
      const programCard = (id: string, given: readonly ProgramLine[], indent: number) => {
        // What a model writes changes at each token, so it stands in a card of its own under the program, and the
        // program stays as it was drawn.
        const all = given.filter((line) => line.kind !== "writes");
        const writers = given.filter((line) => line.kind === "writes");
        const tails = () => {
          for (const line of writers) {
            const stream = w.host.streams.get(line.step.id);
            // Each line that the model finished shows as python, highlighted as it comes, and the line that it
            // still writes stays a comment under it, so the card is made again at each line and not at each token.
            const { lines, tail: open } = writing(stream);
            if (lines)
              add(
                `wrote-${line.step.id}`,
                lines,
                [],
                (box) => box.add(this.inset(space.between, this.code(lines))),
                { indent, same: [lines] },
              );
            const tail = open || (lines ? "writes" : line.text);
            add(
              `writes-${line.step.id}`,
              `${tail}:${this.feedWidth}:${this.theme}`,
              [],
              (box) =>
                box.add(
                  this.inset(
                    space.between,
                    this.text([
                      [`${glyph.running} `, c.accent],
                      [`# ${clip(tail, this.feedWidth - indent - 12)}`, c.syntaxComment],
                    ]),
                  ),
                ),
              { indent },
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
        const took = (step: ActRow) =>
          step.kind !== "rung" || !step.done
            ? ""
            : [
                w.took[step.id] === undefined ? "" : spell(w.took[step.id] ?? 0),
                step.cost ? dollars(step.cost) : "",
              ]
                .filter(Boolean)
                .join(" ");
        const since = (act?: ActRow) => {
          const began = act && w.started[act.id];
          const ms = began ? Date.now() - began : 0;
          return ms >= 1000 ? spell(Math.floor(ms / 1000) * 1000) : "";
        };
        const room = this.feedWidth - indent - 8;
        // A click on a line opens it in place: its whole text, and under it what each act of the line gave. The
        // program parts at each open line, so that the cards of its acts stand under it.
        const segments: ProgramLine[][] = [[]];
        // A mark whose place falls between two steps of the program parts it there.
        const cuts = new Map<number, PartMark[]>();
        let stepped: ActRow | undefined;
        for (const line of program) {
          if (line.kind === "code" && line.step !== stepped) {
            stepped = line.step;
            const marked = due(line.step.began, line.step.on);
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
                ? `${line.text}|${this.lineHint(line)}|${line.runs ? since(line.acts.at(-1)) : ""}|${line.fails ?? ""}|${took(line.step)}|${this.lineOpen(line)}|${[...this.openStrings(line)]}`
                : `${line.kind}|${line.text}`,
            ),
          ].join("\n");
          add(
            at ? `program-${id}-${at}` : `program-${id}`,
            key,
            [],
            (box) => {
              if (hide && !at)
                box.add(
                  this.link(
                    [
                      [`${glyph.closed} `, c.faint],
                      [`# ${hide} earlier ${hide === 1 ? "step" : "steps"}`, c.syntaxComment],
                    ],
                    () => {
                      w.folds[`steps:${id}`] = false;
                      w.save();
                      this.renderContent();
                    },
                  ),
                );
              box.add(this.programView(segment, room, took, since, heads));
            },
            { indent, same: [key] },
          );
          const open = segment.at(-1);
          if (open?.kind === "code" && this.lineOpen(open) && open.acts.length)
            add(
              `opened-${id}-${at}`,
              open.acts.map((one) => `${one.id}:${one.done}:${(one as ActRow).output ?? ""}`).join(" ") +
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
      const marks = this.parts.marks(this.seen()).filter((one) => moment === undefined || one.at < moment);
      const placing = (line: (typeof lines)[number]) =>
        line.type !== "tail" && line.type !== "value" && line.type !== "more";
      // A mark waits until the first act that began at its place or later, and shows before it: before a line of the
      // feed, or inside a program, between two of its steps.
      const pending = [...marks].sort((one, other) => one.at - other.at);
      // A mark of a later chain of the run waits for the feed of that chain, since the feed of an earlier chain ends
      // where the compaction cut it, whatever began after in time.
      const due = (began: number, on: string) => {
        const out: PartMark[] = [];
        const here = run.indexOf(on);
        for (const one of [...pending])
          if (one.at <= began && (here < 0 || run.indexOf(one.chain) <= here)) {
            pending.splice(pending.indexOf(one), 1);
            out.push(one);
          }
        return out;
      };
      const showMark = (mark: PartMark) => {
        const said = JSON.stringify(mark.lines ?? []);
        add(
          `mark-${mark.chain}-${mark.at}`,
          `${mark.title}\n${said}`,
          [],
          (box) => {
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
          },
          { same: [mark.title, said] },
        );
      };
      let collapsed = -1;
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
        // Only a line that the feed shows places a mark: a step under a step stands folded, whenever it began.
        if (placing(line)) for (const mark of due(line.act.began, (line.act as ActRow).on)) showMark(mark);
        const depth = line.depth;
        const act = line.act as ActRow;
        const indent = line.depth * space.between;
        if (line.type === "rung") {
          const code = this.subject(act);
          // The words that a chain wrote to retell its source stand as one line, which names the source.
          if (rows.get(act.by)?.kind === "chain") {
            if (retold || continuing.has(act.by)) continue;
            retold = true;
            const source = String(rows.get(act.by)?.words[1] ?? "");
            // The branch says what it takes of its source: each message before the moment it starts at, which the
            // word that made it does not leave out.
            const maker = w.actOf(String(rows.get(act.by)?.by ?? ""));
            const messages =
              maker &&
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
            act.by === "operator" && /^\s*chain\(/.test(code)
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
          // A word of the operator stands as its program under a quiet header.
          add(
            act.id,
            `you:${this.theme}`,
            [
              [glyph.mark, threadTone(act.id)],
              ["You ran", c.muted],
            ],
            () => {},
            { indent, heading: true, shown: code },
          );
          programCard(
            act.id,
            programFor(act.id, stepOf(act), () =>
              stepLines(act, madeMap(), word, owed, named, w.host.streams),
            ),
            indent,
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
            add(`tail-${act.id}`, line.text, [], (box) => this.excerpt(box, line.text, true), { indent });
        } else if (line.type === "value") {
          // A notice closes with nothing, which its quiet line says already.
          if (act.words[0] === "None") continue;
          const value = typeof act.value === "boolean" ? (act.value ? "yes" : "no") : display(act.value);
          const answered = asksOperator(act);
          const short = !value.includes("\n") && Bun.stringWidth(value) < this.feedWidth - 20 - indent;
          add(
            `value-${act.id}`,
            `${value}\n${short}\n${answered}`,
            answered
              ? [
                  [`${glyph.done} `, c.success],
                  ["You answered", c.text, bold],
                  [short ? `  ${value}` : "", c.muted],
                ]
              : this.answerLabel(act, false),
            (box) => {
              if (answered && short) return;
              // A column, and not a row, holds the answer, so that its paragraphs wrap at the width of the feed.
              const answer = this.box({ paddingLeft: space.between });
              answer.add(this.markdown(value));
              box.add(answer);
            },
            {
              group: answered ? "question" : "assistant",
              act,
              heading: true,
              indent: indent - space.between,
              shown: value,
            },
          );
        } else if (line.type === "ear") {
          // The rungs that an ear wrote, as the words of the extensions, stand on one quiet row.
          const ids = line.rungs.map((rung) => rung.id);
          const span =
            ids.length > 1 ? `${ids[0]}…${(ids.at(-1) ?? "").replace(/^\D+/, "")}` : (ids[0] ?? "");
          add(
            `ear-${act.id}`,
            `${span}:${line.state}:${line.acts.map((chip) => chip.label + chip.state).join(" ")}:${this.theme}`,
            [
              [
                `${line.state === "done" ? glyph.ring : glyph.dot} `,
                line.state === "failed" ? c.danger : c.faint,
              ],
              [act.by, c.muted],
              [`  ${span}`, c.faint],
              ...this.chipParts(line.acts, this.feedWidth - indent - Bun.stringWidth(act.by + span) - 8),
            ],
            () => {},
            { heading: true, indent, shown: `${act.by} ${span}` },
          );
        } else if (line.type === "more") {
          // The earlier prompts of a long thread, or the earlier rungs of a long ladder, stand as one line until a click
          // opens them.
          const { key, count, heads } = line;
          // The line names what it holds: messages, words, or steps of both.
          const noun =
            heads.length === 1 && heads[0] === "rung"
              ? count === 1
                ? "rung"
                : "rungs"
              : heads.length === 1 && heads[0] === "prompt"
                ? count === 1
                  ? "message"
                  : "messages"
                : count === 1
                  ? "step"
                  : "steps";
          add(
            `more-${key}`,
            `${key}:${count}:${this.theme}`,
            [],
            (box) =>
              box.add(
                this.link(
                  [
                    [`${glyph.closed} `, c.faint],
                    [w.watching ? earlierSteps(count) : `${count} earlier ${noun}`, c.muted],
                  ],
                  () => {
                    w.folds[key] = false;
                    w.save();
                    this.renderContent();
                  },
                ),
              ),
            { indent },
          );
        } else if (line.type === "act") {
          // The ceiling of a grant stands in the sidebar, so the feed shows no grant. An act that no word made stands as
          // a line of python under a quiet header, as a word that the operator runs.
          if (act.kind === "grant") continue;
          add(
            act.id,
            `ran:${act.by}:${this.theme}`,
            [
              [glyph.mark, threadTone(act.id)],
              [act.by === "operator" ? "You ran" : `${act.by} ran`, c.muted],
            ],
            () => {},
            { indent, heading: true, shown: act.id },
          );
          programCard(
            act.id,
            programFor(`act:${act.id}`, `${act.done}:${act.paused}:${act.output ?? 0}`, () =>
              actLines(act, owed, named),
            ),
            indent,
          );
        } else if (asksOperator(act) && act.words[0] === "None") {
          // A prompt of None asks nothing: the console closed it as it took it, and it settles as a quiet line, its
          // first line, which opens to the whole message. No key dismisses it.
          const message = String(act.words[1] ?? "");
          const from = act.by === "operator" ? "for you" : `from ${act.by}`;
          const lines = message.split("\n").filter((one) => one.trim()).length;
          const more = lines > 1 ? `  +${lines - 1} ${lines === 2 ? "line" : "lines"}` : "";
          const room = this.feedWidth - indent - 12 - Bun.stringWidth(from + more);
          add(
            act.id,
            `${message}\n${this.feedWidth}\n${this.theme}`,
            [
              [`${glyph.notice} `, c.faint],
              ["Notice", c.muted],
              [`  ${from}`, c.faint],
              [`  ${clip(shortenHomes(gist(message)), room)}`, c.muted],
              [more, c.faint],
            ],
            (box) => {
              const body = this.box({ paddingLeft: space.between });
              body.add(this.markdown(message, c.muted));
              box.add(body);
            },
            { compact: true, group: "notice", act, indent, shown: message },
          );
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
              this.panel(box, waiting ? c.warning : c.border).add(
                this.markdown(message, c.text, c.background),
              );
              // The question that the input answers says so, and a click on another one points the input at it.
              if (ask)
                box.add(
                  this.inset(
                    space.between,
                    this.text(
                      aimed
                        ? [
                            ["The input below answers this question", c.muted],
                            [ask.choices.length ? ` with ${wants(ask)}` : `, as ${ask.shape}`, c.muted],
                            [", or press ", c.faint],
                            ["⌃A", c.muted],
                          ]
                        : [["Click here to answer this question in the input below", c.muted]],
                      c.muted,
                      { run: () => (aimed ? this.question(ask) : void w.point(ask.id).catch(this.report)) },
                    ),
                  ),
                );
            },
            { group: "question", act, shown: message },
          );
        } else {
          const message = String(act.words[1] ?? "");
          const user = w.isUserPrompt(act);
          // Each thread carries its color on its header and on the mark at its left.
          // A message of a thread takes the color of its thread, and hides with it.
          const own = this.chainThreadsBy(w.selected);
          const thread = own.by.get(act.id);
          const host = thread ?? own.of.get(act.id);
          const tone = threadTone(host?.act.id ?? act.id);
          if (host && this.threadFolded(host.act.id)) {
            collapsed = depth;
            if (!thread) continue;
            const [mark, color] = thread ? threadMarks(thread) : [glyph.ring, c.faint];
            const steps = (madeBy(act.id).length || 0) + (thread?.messages.length ?? 0);
            const room = this.feedWidth - indent - 16;
            add(
              act.id,
              `folded\n${message}\n${mark}\n${steps}\n${this.feedWidth}\n${this.theme}`,
              [],
              (box) =>
                box.add(
                  this.link(
                    [
                      [`${glyph.mark}`, tone],
                      [`${mark} `, color],
                      [`${glyph.closed} `, c.faint],
                      [clip(shortenHomes(gist(message)), room), c.muted],
                      [steps ? `  ${steps} ${steps === 1 ? "step" : "steps"}` : "", c.faint],
                      // A folded thread that a cancel or a failure ended says so in words, beside its mark.
                      [
                        thread.state === "cancelled" || thread.state === "failed" ? `  ${thread.state}` : "",
                        thread.state === "failed" ? c.danger : c.faint,
                      ],
                    ],
                    () => this.foldThread(act.id, false),
                  ),
                ),
              { act, indent, shown: message },
            );
            continue;
          }
          // A message that the chain or a rung sent is its heading when it is one short line, and stands under its
          // heading otherwise. The heading says who sent it, and to which model.
          const sent = user ? "" : this.sender(act);
          const line =
            !message.includes("\n") && Bun.stringWidth(message) + Bun.stringWidth(sent) < this.feedWidth - 8;
          const state = user ? this.promptState(act) : [];
          // A long message shows its first rows until a click opens it, so that a brief of a page leaves the feed
          // readable at a glance.
          const [shortened, rest] =
            w.folds[`message:${act.id}`] === false ? [message, 0] : this.firstRows(message, MESSAGE_ROWS);
          add(
            act.id,
            `${message}\n${line}\n${sent}\n${plain(state)}\n${rest}\n${thread?.resolved}\n${this.theme}`,
            user
              ? [["You", tone]]
              : [
                  [`${glyph.mark}`, tone],
                  [`${glyph.ring} `, c.faint],
                  [line ? message : sent, c.muted, line ? bold : 0],
                  [line ? `  ${sent}` : "", c.faint],
                ],
            (box) => {
              // A message shows as the operator typed it, with each image it attached named by a mark that opens it.
              // Its first line says at its right the type of the answer that it asks for, and where it stands.
              if (user) {
                // A right click on a message opens its menu, as on the heading of any other act.
                const row = this.box({
                  flexDirection: "row",
                  onMouseDown: (event) => {
                    if (event.button === 2)
                      this.actActions(this.session.acts.find((one) => one.id === act.id) ?? act);
                  },
                });
                const text = this.message(shortened);
                text.flexGrow = 1;
                text.flexShrink = 1;
                row.add(text);
                row.add(this.text(state, c.faint, { marginLeft: space.between }));
                const controls = this.threadControls(act);
                if (controls) row.add(controls);
                this.panel(box, tone).add(row);
              } else if (!line) {
                const note = this.box({ paddingLeft: space.between });
                note.add(this.markdown(shortened, c.muted));
                box.add(note);
              }
              if (rest)
                box.add(
                  this.inset(
                    space.between,
                    this.link(
                      [
                        [`${glyph.closed} `, c.faint],
                        [`${rest} more ${rest === 1 ? "line" : "lines"}`, c.muted],
                      ],
                      () => {
                        w.folds[`message:${act.id}`] = false;
                        w.save();
                        this.renderContent();
                      },
                    ),
                  ),
                );
              const controls = user ? undefined : this.threadControls(act);
              if (controls) box.add(controls);
            },
            { group: user ? "user" : "sent", act, heading: user ? false : undefined, indent, shown: message },
          );
          const parts = [act, ...(thread?.messages ?? [])];
          programCard(
            act.id,
            programFor(act.id, `${w.folds[`steps:${act.id}`]}|${liveOf(parts.map((one) => one.id))}`, () =>
              threadProgram(
                parts,
                madeMap(),
                word,
                owed,
                named,
                undefined,
                w.folds[`steps:${act.id}`] === false ? undefined : PROGRAM_STEPS,
              ),
            ),
            indent,
          );
        }
      }
      for (const mark of pending.splice(0)) showMark(mark);
      // A paused chain says so at the end of its feed, with the reason that the last failure gave, and a button that
      // wakes it, since nothing new runs on it until then.
      if (w.paused && !w.search) {
        const failure = w.activity.findLast((act) => act.kind === "rung" && act.run?.status === "failed")?.run
          ?.reason;
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
                  this.text(clip(shortenHomes(failure).split("\n")[0] ?? "", this.feedWidth - 8), c.danger),
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
              turn[0] === "assistant" ? `${glyph.dot} ` : `${glyph.ring} `,
              turn[0] === "assistant" ? c.secondary : c.faint,
            ],
            [turn[0], c.text, bold],
            [`  turn ${index + 1}`, c.faint],
          ],
          (box) => {
            const inner = this.box({ paddingLeft: space.between });
            inner.add(turn[0] === "assistant" ? this.code(text) : this.transcriptText(text));
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
          this.feedWidth - Bun.stringWidth(` ${glyph.dot} ${path}  ${what}`) - Bun.stringWidth(tail),
        );
        // Two sides need room for two lines of code side by side, and one side reads better below that.
        const split = this.renderer.width >= 140;
        add(
          `change:${path}`,
          `${w.scope}:${change.before.length}:${change.after.length}:${change.added}:${change.removed}:${split}:${this.theme}`,
          [
            [` ${glyph.dot} `, color, 0, c.panel],
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
