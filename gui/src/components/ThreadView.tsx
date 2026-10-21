import { reason, type Thread } from "@furb/engine/life";
import { ChevronRight, CornerDownRight } from "lucide-react";
import { useState } from "react";
import { actorName, send } from "../data.ts";
import { chainName, type Held, type Moment, threadState } from "../life.ts";
import { steps } from "../steps.ts";
import { Code } from "./Code.tsx";
import { Glyph } from "./Glyph.tsx";

/** The answer of the operator to a question, in place, under the question. */
function Answer({ thread }: { thread: Thread }) {
  const [line, setLine] = useState("");
  const answer = () => {
    if (!line.trim()) return;
    send({ kind: "answer", value: { id: thread.act.id, shape: thread.shape, line } });
    setLine("");
  };
  return (
    <div className="mt-8 rounded-xl border border-asks/35 bg-asks/[0.06] p-3">
      <div className="flex items-center gap-2 px-1 text-[13px] text-asks">
        <Glyph state="asks" size={11} />
        This thread waits for your answer, as {thread.shape}
      </div>
      <div className="mt-2.5 flex gap-2">
        <input
          value={line}
          onChange={(event) => setLine(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") answer();
          }}
          placeholder="Your answer"
          className="min-w-0 flex-1 rounded-lg border border-hair bg-panel px-3 py-1.5 text-[14px] text-text outline-none placeholder:text-faint focus:border-asks/60"
        />
        <button
          type="button"
          onClick={answer}
          disabled={!line.trim()}
          className="rounded-lg bg-asks px-3.5 py-1.5 text-[13px] font-medium text-accent-ink disabled:opacity-40"
        >
          Answer
        </button>
      </div>
    </div>
  );
}

/** One thread read top to bottom: what was asked, each step that its model took, folded to what it did and open to
 * its code, what a model writes now, the threads that a step asked, and the answer. */
export function ThreadView({
  held,
  moment,
  thread,
  onSelect,
}: {
  held: Held;
  moment: Moment;
  thread: Thread;
  onSelect: (thread: string) => void;
}) {
  const ladder = steps(moment, thread.act, held.directory, held.streams);
  const last = ladder.at(-1)?.id;
  const [open, setOpen] = useState<Set<string>>(() => new Set(last ? [last] : []));
  const [whole, setWhole] = useState(false);
  const message = String(thread.act.words[1] ?? "");
  const maker = moment.rows.get(thread.act.by);
  const asker = maker?.kind === "rung" ? `${chainName(moment, maker.on)} asked at ${maker.id}` : "You asked";
  const owed = moment.owed.has(thread.act.id);
  const actor = held.measures[thread.chain]?.actor ?? "";
  const toggle = (id: string) => {
    const next = new Set(open);
    if (!next.delete(id)) next.add(id);
    setOpen(next);
  };
  const answer = thread.act.done ? thread.act.value : undefined;
  const failed = threadState(moment, thread) === "failed";
  return (
    <div className="h-full overflow-y-auto">
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
          <ol className="relative mt-8">
            <span className="absolute top-2 bottom-2 left-[6px] w-px bg-hair" aria-hidden />
            {ladder.map((step, n) => {
              const shown = open.has(step.id) || step.thinks;
              return (
                <li key={step.id} className="relative pb-5 pl-8">
                  <span className="absolute top-[5px] left-0 grid h-[13px] w-[13px] place-items-center bg-bg">
                    <Glyph state={step.state} size={12} />
                  </span>
                  <button
                    type="button"
                    onClick={() => toggle(step.id)}
                    className="group flex w-full items-center gap-2 text-left"
                  >
                    <span className={`text-[14px] ${step.state === "failed" ? "text-failed" : "text-text"}`}>
                      {step.said}
                    </span>
                    <span className="text-[12px] text-faint">Step {n + 1}</span>
                    {step.thinks ? null : (
                      <ChevronRight
                        size={14}
                        className={`text-faint transition-transform duration-200 ease-[var(--ease-out-soft)] ${shown ? "rotate-90" : ""}`}
                      />
                    )}
                  </button>
                  {step.thinks && step.writing ? (
                    <pre className="mt-2.5 max-h-[320px] overflow-y-auto rounded-xl bg-well px-4 py-3 font-mono text-[12.5px] leading-relaxed whitespace-pre-wrap text-muted">
                      {step.writing}
                      <span className="ml-0.5 inline-block h-[13px] w-[7px] translate-y-[2px] animate-pulse bg-accent" />
                    </pre>
                  ) : null}
                  {shown && !step.thinks ? (
                    <div className="mt-2.5 overflow-hidden rounded-xl bg-well py-2 pr-3">
                      <Code steps={[step]} heads={false} wrap />
                    </div>
                  ) : null}
                  {step.asks.map((id) => {
                    const asked = moment.threads.find((one) => one.act.id === id);
                    if (!asked) return null;
                    return (
                      <button
                        key={id}
                        type="button"
                        onClick={() => onSelect(id)}
                        className="mt-2.5 flex w-full items-center gap-3 rounded-xl border border-hair bg-panel px-3.5 py-2.5 text-left transition-colors duration-150 hover:border-hair-strong"
                      >
                        <CornerDownRight size={14} className="text-faint" />
                        <Glyph state={threadState(moment, asked)} size={12} />
                        <span className="min-w-0 flex-1 truncate text-[13.5px] text-text">{asked.title}</span>
                        <span className="shrink-0 text-[12px] text-faint">
                          {asked.to === "operator" ? "to you" : chainName(moment, asked.chain)}
                        </span>
                      </button>
                    );
                  })}
                </li>
              );
            })}
          </ol>
        ) : null}
        {owed && held.works && moment.now ? <Answer thread={thread} /> : null}
        {answer !== undefined && answer !== null ? (
          <section className="mt-6 pl-8">
            <div
              className={`text-[15px] leading-[1.65] whitespace-pre-wrap ${failed ? "text-failed" : "text-text"}`}
            >
              {failed
                ? reason(thread.act, held.directory)
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
