import { ChevronDown, Plus } from "lucide-react";
import { chainName, type Thread, type View } from "../data.ts";
import { Glyph } from "./Glyph.tsx";

/** The progress of a thread as its steps: one tick each, colored by how it ended. */
function Ticks({ thread }: { thread: Thread }) {
  const ticks = thread.ladder.slice(-6);
  if (ticks.length === 0) return null;
  return (
    <span
      className="flex items-center gap-[3px]"
      title={`${thread.steps.done} of ${thread.steps.all} steps ended well`}
    >
      {ticks.map((step) => (
        <span
          key={step.id}
          className="h-[10px] w-[3px] rounded-full"
          style={{
            background:
              step.state === "done"
                ? "var(--done)"
                : step.state === "failed"
                  ? "var(--failed)"
                  : step.state === "asks"
                    ? "var(--asks)"
                    : "var(--accent)",
            opacity: step.state === "done" ? 0.55 : 1,
          }}
        />
      ))}
    </span>
  );
}

/** The rail: the life and what it spent, then its threads as a tree, the questions that wait for you first. */
export function Rail({
  view,
  selected,
  onSelect,
  onNew,
}: {
  view: View;
  selected: string;
  onSelect: (thread: string) => void;
  onNew: () => void;
}) {
  const { life } = view;
  const owed = view.threads.filter((one) => life.owed.includes(one.id));
  const share = life.ceiling ? Math.min(life.spent / life.ceiling, 1) : 0;
  return (
    <aside className="flex h-full w-[272px] shrink-0 flex-col bg-rail">
      <div data-tauri-drag-region className="h-[46px] shrink-0" />
      <div className="px-4 pb-3">
        <button
          type="button"
          className="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left hover:bg-text/5"
        >
          <span className="grid h-6 w-6 place-items-center rounded-md bg-accent text-[13px] font-semibold text-accent-ink">
            f
          </span>
          <span className="flex-1 truncate text-[14px] font-semibold">{life.place}</span>
          <ChevronDown size={14} className="text-faint" />
        </button>
      </div>
      <div className="px-4">
        <button
          type="button"
          onClick={onNew}
          className="flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-[13.5px] text-muted hover:bg-text/5 hover:text-text"
        >
          <Plus size={15} strokeWidth={1.8} />
          New thread
          <kbd className="ml-auto font-sans text-[11px] text-faint">A</kbd>
        </button>
      </div>
      {owed.length > 0 ? (
        <div className="mx-4 mt-4 rounded-xl border border-asks/25 bg-asks/[0.07] p-1.5">
          <div className="px-2 pt-1 pb-1.5 text-[11.5px] font-medium text-asks">
            {owed.length === 1 ? "One question waits for you" : `${owed.length} questions wait for you`}
          </div>
          {owed.map((thread) => (
            <button
              key={thread.id}
              type="button"
              onClick={() => onSelect(thread.id)}
              className="flex w-full items-start gap-2 rounded-lg px-2 py-1.5 text-left text-[13px] leading-snug hover:bg-asks/10"
            >
              <span className="pt-[3px]">
                <Glyph state="asks" size={11} />
              </span>
              <span className="line-clamp-2">{thread.title}</span>
            </button>
          ))}
        </div>
      ) : null}
      <div className="mt-5 px-6 pb-1.5 text-[11.5px] text-faint">Threads</div>
      <nav className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
        {view.threads.map((thread) => {
          const on = thread.id === selected;
          return (
            <button
              key={thread.id}
              type="button"
              onClick={() => onSelect(thread.id)}
              className={`group relative flex w-full items-center gap-2.5 rounded-lg py-[7px] pr-2 text-left transition-colors duration-150 ${
                on ? "bg-panel shadow-[var(--shadow)]" : "hover:bg-text/[0.04]"
              }`}
              style={{ paddingLeft: 8 + thread.depth * 16 }}
            >
              {thread.depth > 0 ? (
                <span
                  className="absolute top-0 bottom-0 w-px bg-hair"
                  style={{ left: 13 + (thread.depth - 1) * 16 }}
                  aria-hidden
                />
              ) : null}
              <Glyph state={thread.state} size={12} title />
              <span className="min-w-0 flex-1">
                <span
                  className={`block truncate text-[13.5px] ${on ? "text-text" : "text-muted group-hover:text-text"}`}
                >
                  {thread.title}
                </span>
                <span className="block truncate text-[11.5px] text-faint">
                  {thread.to === "operator" ? "asks you" : chainName(view, thread.chain)}
                </span>
              </span>
              <Ticks thread={thread} />
            </button>
          );
        })}
      </nav>
      <div className="border-t border-hair px-6 py-3.5">
        <div className="flex items-baseline justify-between text-[12px]">
          <span className="text-muted">Spent</span>
          <span className="tabular font-mono text-[12px] text-text">
            ${life.spent.toFixed(2)}
            {life.ceiling ? <span className="text-faint"> of ${life.ceiling.toFixed(2)}</span> : null}
          </span>
        </div>
        <div className="mt-2 h-[3px] overflow-hidden rounded-full bg-hair">
          <div className="h-full rounded-full bg-accent" style={{ width: `${share * 100}%` }} />
        </div>
      </div>
    </aside>
  );
}
