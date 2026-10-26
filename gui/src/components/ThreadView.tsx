import { useVirtualizer } from "@tanstack/react-virtual";
import { compactedRun, type Exit, type LiveAct, reason, type Thread } from "furb/life";
import { ChevronRight, CornerDownRight } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { actorName } from "../data.ts";
import { chainName, type Held, type Moment, type Streamed, threadState } from "../life.ts";
import { placed, steps } from "../steps.ts";
import type { Mark } from "../wire.ts";
import { Code } from "./Code.tsx";
import { ActMenu, type Rewinding } from "./Controls.tsx";
import { Glyph } from "./Glyph.tsx";

/** How many characters of what a model writes, or of what a command prints, a step shows while it runs. */
const TAIL = 4000;

/** The last lines of a text, at most TAIL characters of it. */
const tail = (text: string) => (text.length > TAIL ? `…${text.slice(-TAIL)}` : text);

/** What a model writes for a step while its reply runs: its word as it comes, or its thought before it writes, and
 * the retry of its call when one waits. */
function Writing({ stream }: { stream: Streamed }) {
  const text = stream.text || stream.thinking;
  return (
    <div className="mt-2.5 overflow-hidden rounded-xl bg-well px-3.5 py-2.5">
      {stream.retry ? (
        <div className="mb-1.5 text-[12px] text-asks">
          The call of the model failed: {stream.retry.why}. Attempt {stream.retry.attempt} of{" "}
          {stream.retry.most} comes next.
        </div>
      ) : null}
      <pre
        data-stream
        className={`font-mono text-[12.5px] leading-[1.6] break-words whitespace-pre-wrap ${stream.text ? "text-text" : "text-faint italic"}`}
      >
        {tail(text)}
        <span className="ml-px inline-block h-[1.05em] w-[7px] translate-y-[2px] animate-pulse bg-accent" />
      </pre>
    </div>
  );
}

/** What a command that runs prints, as it prints it. */
function Printing({ act, live }: { act: LiveAct; live: boolean }) {
  const exit = act.value as Exit | null;
  const out = `${exit?.stdout?.content ?? ""}${exit?.stderr?.content ?? ""}`;
  const command = String(act.words[0] ?? "").split("\n")[0] ?? "";
  return (
    <div className="mt-2.5 overflow-hidden rounded-xl bg-well px-3.5 py-2.5">
      <div className="mb-1 flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-muted">$ {command}</span>
        <ActMenu act={act} live={live} onRewind={() => undefined} />
      </div>
      <pre
        data-output
        className="font-mono text-[12px] leading-[1.55] break-words whitespace-pre-wrap text-muted"
      >
        {out ? tail(out) : "Nothing printed yet"}
      </pre>
    </div>
  );
}

/** The class of the color of each tone that a part of an extension gives a run of text. */
const TONES: Record<string, string> = {
  text: "text-text",
  muted: "text-muted",
  faint: "text-faint",
  accent: "text-accent",
  success: "text-done",
  warning: "text-asks",
  danger: "text-failed",
};

/** A mark of a part of an extension, as a compaction: a rule with its title across the thread, then its lines. */
export function Rule({ mark }: { mark: Mark }) {
  return (
    <section aria-label={mark.title} className="pb-5">
      <div className="flex items-center gap-3 text-[12px] text-faint">
        <span className="h-px w-4 bg-hair-strong" aria-hidden />
        <span className="shrink-0">{mark.title}</span>
        <span className="h-px flex-1 bg-hair-strong" aria-hidden />
      </div>
      {mark.lines.map((line, at) => (
        <div
          // biome-ignore lint/suspicious/noArrayIndexKey: the lines of a mark have no name but their place.
          key={at}
          className="mt-1 pl-7 text-[12.5px] leading-relaxed whitespace-pre-wrap"
        >
          {line.map((run, n) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: the runs of a line have no name but their place.
            <span key={n} className={TONES[run.tone] ?? "text-muted"}>
              {run.text}
            </span>
          ))}
        </div>
      ))}
    </section>
  );
}

/** One thread read top to bottom: what was asked, each step that its model took, folded to what it did and open to
 * its code, what a model writes and what a command prints while they run, the threads that a step asked, and the
 * answer. */
export function ThreadView({
  held,
  moment,
  thread,
  onSelect,
  onRewind,
}: {
  held: Held;
  moment: Moment;
  thread: Thread;
  onSelect: (thread: string) => void;
  onRewind: (rewinding: Rewinding | string) => void;
}) {
  const life = held.life;
  const directory = life?.directory ?? "";
  const live = Boolean(life && !life.reading && !life.recorded);
  const ladder = steps(moment, thread.act, directory);
  const run = compactedRun(thread.chain, moment.compactions);
  const marks = placed(
    ladder,
    (life?.marks ?? []).filter((mark) => run.includes(mark.chain)),
    (id) => moment.rows.get(id)?.began ?? 0,
  );
  const last = ladder.at(-1)?.id;
  const [open, setOpen] = useState<Set<string>>(() => new Set(last ? [last] : []));
  const [whole, setWhole] = useState(false);
  const message = String(thread.act.words[1] ?? "");
  const maker = moment.rows.get(thread.act.by);
  const asker = maker?.kind === "rung" ? `${chainName(moment, maker.on)} asked at ${maker.id}` : "You asked";
  const owed = moment.owed.has(thread.act.id);
  const actor = life?.measures[thread.chain]?.actor ?? "";
  const toggle = (id: string) => {
    const next = new Set(open);
    if (!next.delete(id)) next.add(id);
    setOpen(next);
  };
  const answer = thread.act.done ? thread.act.value : undefined;
  const failed = threadState(moment, thread) === "failed";
  // The ladder draws only the steps that the view shows, so a thread of hundreds of steps stays light.
  const scroller = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLOListElement>(null);
  const [margin, setMargin] = useState(0);
  useLayoutEffect(() => {
    setMargin(list.current?.offsetTop ?? 0);
  });
  // A view that stands at its end stays there while the thread grows, as a model writes or a command prints.
  const end = useRef(true);
  useEffect(() => {
    const one = scroller.current;
    if (one && end.current) one.scrollTop = one.scrollHeight;
  });
  const rows = useVirtualizer({
    count: ladder.length,
    getScrollElement: () => scroller.current,
    estimateSize: () => 44,
    overscan: 6,
    scrollMargin: margin,
    getItemKey: (at) => ladder[at]?.id ?? at,
  });
  return (
    <div
      ref={scroller}
      onScroll={(event) => {
        const one = event.currentTarget;
        end.current = one.scrollHeight - one.scrollTop - one.clientHeight < 80;
      }}
      className="h-full overflow-y-auto"
    >
      <article className="mx-auto max-w-[760px] px-10 pt-9 pb-56">
        <div className="mb-2.5 flex items-center gap-2 text-[12.5px] text-faint">
          <span>{asker}</span>
          {owed || actor ? (
            <span className="ml-1 rounded-md bg-text/[0.06] px-1.5 py-px text-[11.5px] text-muted">
              {owed ? "you answer" : actorName(actor)}
            </span>
          ) : null}
          <span className="rounded-md bg-text/[0.06] px-1.5 py-px font-mono text-[11px] text-muted">
            {thread.shape}
          </span>
          <span className="flex-1" />
          <ActMenu act={thread.act} live={live} onRewind={onRewind} />
        </div>
        <h1 className="text-[21px] leading-[1.35] font-semibold tracking-[-0.01em] text-text">
          {thread.title}
        </h1>
        {message.trim() !== thread.title.trim() ? (
          <div className="mt-3">
            <div
              className={`overflow-hidden text-[14px] leading-relaxed whitespace-pre-wrap text-muted ${whole ? "" : "max-h-[110px] [mask-image:linear-gradient(to_bottom,black_55%,transparent)]"}`}
            >
              {message.slice(message.indexOf(thread.title) + thread.title.length).trim()}
            </div>
            <button
              type="button"
              onClick={() => setWhole(!whole)}
              className="mt-1 text-[12.5px] text-accent hover:underline"
            >
              {whole ? "Show less" : "Show the whole message"}
            </button>
          </div>
        ) : null}
        {ladder.length > 0 ? (
          <ol ref={list} className="relative mt-8" style={{ height: rows.getTotalSize() }}>
            <span className="absolute top-2 bottom-2 left-[6px] w-px bg-hair" aria-hidden />
            {rows.getVirtualItems().map((row) => {
              const step = ladder[row.index];
              if (!step) return null;
              const n = row.index;
              const shown = open.has(step.id) || step.thinks;
              const stream = held.streams.get(step.id);
              const said =
                step.thinks && live
                  ? stream?.text
                    ? "Its model writes this step"
                    : "Its model thinks"
                  : step.said;
              const running = moment.acts.filter(
                (act) => act.by === step.id && act.kind === "bash" && !act.done,
              );
              return (
                <li
                  key={step.id}
                  data-index={row.index}
                  ref={rows.measureElement}
                  className="absolute left-0 w-full pb-5 pl-8"
                  style={{ top: row.start - margin }}
                >
                  {marks[row.index]?.map((mark) => (
                    <div key={`${mark.chain} ${mark.at} ${mark.title}`} className="-ml-8">
                      <Rule mark={mark} />
                    </div>
                  ))}
                  <div className="relative">
                    <span className="absolute top-[5px] -left-8 grid h-[13px] w-[13px] place-items-center bg-bg">
                      <Glyph state={step.state} size={12} />
                    </span>
                    <div className="flex items-center gap-2">
                      <button
                        type="button"
                        onClick={() => toggle(step.id)}
                        className="group flex min-w-0 flex-1 items-center gap-2 text-left"
                      >
                        <span
                          className={`text-[14px] ${step.state === "failed" ? "text-failed" : "text-text"}`}
                        >
                          {said}
                        </span>
                        <span className="text-[12px] text-faint">Step {n + 1}</span>
                        {step.thinks ? null : (
                          <ChevronRight
                            size={14}
                            className={`text-faint transition-transform duration-200 ease-[var(--ease-out-soft)] ${shown ? "rotate-90" : ""}`}
                          />
                        )}
                      </button>
                      {moment.rows.get(step.id) ? (
                        <ActMenu act={moment.rows.get(step.id) as LiveAct} live={live} onRewind={onRewind} />
                      ) : null}
                    </div>
                    {shown && !step.thinks ? (
                      <div className="mt-2.5 overflow-hidden rounded-xl bg-well py-2 pr-3">
                        <Code steps={[step]} heads={false} wrap />
                      </div>
                    ) : null}
                    {step.thinks && stream ? <Writing stream={stream} /> : null}
                    {running.map((act) => (
                      <Printing key={act.id} act={act} live={live} />
                    ))}
                    {step.asks.map((id) => {
                      const asked = moment.threads.find((one) => one.act.id === id);
                      if (!asked) return null;
                      // A notice to you asks nothing, so it reads as a quiet line, and opens no thread.
                      if (asked.to === "operator" && asked.shape === "None")
                        return (
                          <div key={id} className="mt-2 flex items-start gap-2 pl-0.5 text-[13px] text-muted">
                            <span className="pt-[1px] text-faint">·</span>
                            <span className="whitespace-pre-wrap">
                              {String(asked.act.words[1] ?? asked.title)}
                            </span>
                          </div>
                        );
                      return (
                        <button
                          key={id}
                          type="button"
                          onClick={() => onSelect(id)}
                          className="mt-2.5 flex w-full items-center gap-3 rounded-xl border border-hair bg-panel px-3.5 py-2.5 text-left transition-colors duration-150 hover:border-hair-strong"
                        >
                          <CornerDownRight size={14} className="text-faint" />
                          <Glyph state={threadState(moment, asked)} size={12} />
                          <span className="min-w-0 flex-1 truncate text-[13.5px] text-text">
                            {asked.title}
                          </span>
                          <span className="shrink-0 text-[12px] text-faint">
                            {asked.to === "operator" ? "to you" : chainName(moment, asked.chain)}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </li>
              );
            })}
          </ol>
        ) : null}
        {marks[ladder.length]?.map((mark) => (
          <div key={`${mark.chain} ${mark.at} ${mark.title}`} className={ladder.length ? "" : "mt-8"}>
            <Rule mark={mark} />
          </div>
        ))}
        {life?.reading &&
        !life.reading.logged &&
        !life.loading &&
        !thread.act.done &&
        thread.to !== "operator" ? (
          <div className="mt-6 ml-8 rounded-xl border border-dashed border-hair-strong px-4 py-3 text-[13px] leading-relaxed text-muted">
            The record shows no end of this thread. A word may have closed it, or raised, and the record does
            not say which: it keeps only what the World, the models and the operator said, and no log beside
            it agrees with it.
          </div>
        ) : null}
        {answer !== undefined && answer !== null ? (
          <section className="mt-6 pl-8">
            <div
              className={`text-[15px] leading-[1.65] whitespace-pre-wrap ${failed ? "text-failed" : "text-text"}`}
            >
              {failed
                ? reason(thread.act, directory)
                : typeof answer === "string"
                  ? answer
                  : JSON.stringify(answer)}
            </div>
          </section>
        ) : null}
      </article>
    </div>
  );
}
