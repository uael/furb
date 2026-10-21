import { ChevronRight, CornerDownRight } from "lucide-react";
import { useState } from "react";
import { actorName, chainName, type Thread, type View } from "../data.ts";
import { Code } from "./Code.tsx";
import { Glyph } from "./Glyph.tsx";

/** One thread read top to bottom: what was asked, each step that its model took, the threads that a step asked, and
 * the answer. A step folds to what it did, and opens to its code with the value of each act beside its line. */
export function ThreadView({
  view,
  thread,
  onSelect,
}: {
  view: View;
  thread: Thread;
  onSelect: (thread: string) => void;
}) {
  const last = thread.ladder.at(-1)?.id;
  const [open, setOpen] = useState<Set<string>>(() => new Set(last ? [last] : []));
  const chain = view.chains.find((one) => one.id === thread.chain);
  const asker =
    thread.from === "operator" ? "You asked" : `${chainName(view, thread.from)} asked at ${thread.rung}`;
  const owed = view.life.owed.includes(thread.id);
  const toggle = (id: string) => {
    const next = new Set(open);
    if (!next.delete(id)) next.add(id);
    setOpen(next);
  };
  return (
    <div className="h-full overflow-y-auto">
      <article className="mx-auto max-w-[760px] px-10 pt-10 pb-56">
        <div className="mb-2.5 flex items-center gap-2 text-[12.5px] text-faint">
          <span>{asker}</span>
          <span className="ml-1 rounded-md bg-text/[0.06] px-1.5 py-px text-[11.5px] text-muted">
            {owed ? "you answer" : chain ? actorName(chain.actor) : ""}
          </span>
          <span className="rounded-md bg-text/[0.06] px-1.5 py-px font-mono text-[11px] text-muted">
            {thread.shape}
          </span>
        </div>
        <h1 className="text-[21px] leading-[1.35] font-semibold tracking-[-0.01em] text-text">
          {thread.title}
        </h1>
        {thread.ladder.length > 0 ? (
          <ol className="relative mt-8">
            <span className="absolute top-2 bottom-2 left-[6px] w-px bg-hair" aria-hidden />
            {thread.ladder.map((step, n) => {
              const shown = open.has(step.id);
              return (
                <li key={step.id} className="relative pb-5 pl-8">
                  <span className="absolute top-[5px] left-0 grid h-[13px] w-[13px] place-items-center bg-bg">
                    <Glyph state={step.state} size={12} title />
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
                    <ChevronRight
                      size={14}
                      className={`text-faint transition-transform duration-200 ease-[var(--ease-out-soft)] ${shown ? "rotate-90" : ""}`}
                    />
                  </button>
                  {shown ? (
                    <div className="mt-2.5 overflow-hidden rounded-xl bg-well py-2 pr-3">
                      <Code steps={[step]} heads={false} wrap />
                    </div>
                  ) : null}
                  {step.asks.map((id) => {
                    const asked = view.threads.find((one) => one.id === id);
                    if (!asked) return null;
                    return (
                      <button
                        key={id}
                        type="button"
                        onClick={() => onSelect(id)}
                        className="mt-2.5 flex w-full items-center gap-3 rounded-xl border border-hair bg-panel px-3.5 py-2.5 text-left transition-colors duration-150 hover:border-hair-strong"
                      >
                        <CornerDownRight size={14} className="text-faint" />
                        <Glyph state={asked.state} size={12} title />
                        <span className="min-w-0 flex-1 truncate text-[13.5px] text-text">{asked.title}</span>
                        <span className="shrink-0 text-[12px] text-faint">
                          {asked.to === "operator" ? "to you" : chainName(view, asked.chain)}
                        </span>
                      </button>
                    );
                  })}
                </li>
              );
            })}
          </ol>
        ) : null}
        {owed ? (
          <div className="mt-8 flex items-start gap-3 rounded-xl border border-asks/30 bg-asks/[0.06] px-4 py-3.5">
            <span className="pt-[5px]">
              <Glyph state="asks" size={12} />
            </span>
            <div className="text-[13.5px] leading-relaxed">
              <div className="text-text">This thread waits for your answer.</div>
              <div className="text-muted">
                The word of {chainName(view, thread.from)} holds at its await until you answer below. Your
                answer becomes the value of the await in that word.
              </div>
            </div>
          </div>
        ) : null}
        {thread.answer !== null ? (
          <section className="mt-6 pl-8">
            <div
              className={`text-[15px] leading-[1.65] whitespace-pre-wrap ${thread.failed ? "text-failed" : "text-text"}`}
            >
              {thread.answer}
            </div>
          </section>
        ) : null}
      </article>
    </div>
  );
}
