import { actState } from "@furb/engine/life";
import * as Menu from "@radix-ui/react-dropdown-menu";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronDown, FolderOpen } from "lucide-react";
import { useRef } from "react";
import { call, dollars } from "../data.ts";
import { chainName, glyphOf, type Held, type Moment, threadState } from "../life.ts";
import { clip } from "../steps.ts";
import { Glyph } from "./Glyph.tsx";
import { Tip } from "./Tip.tsx";

const tickColors = { done: "var(--done)", failed: "var(--failed)", asks: "var(--asks)" } as const;
const tickWords: Record<string, string> = {
  done: "ended well",
  failed: "failed",
  asks: "asks you",
  paused: "is paused",
  runs: "runs",
  rest: "was cancelled",
  waits: "waits",
};

/** The steps of a thread as ticks, one for each of its last six steps, colored by how each ended, with a tip that
 * says so. */
function Ticks({ moment, thread }: { moment: Moment; thread: string }) {
  const rungs = moment.acts.filter((act) => act.kind === "rung" && act.by === thread);
  if (rungs.length === 0) return null;
  const states = rungs.map((rung) => glyphOf(actState(rung, moment.owed)));
  const first = Math.max(0, rungs.length - 6);
  const tip = (
    <div className="space-y-0.5">
      <div className="text-faint">
        {rungs.length === 1 ? "One step" : `${rungs.length} steps`}, the last {Math.min(6, rungs.length)} here
      </div>
      {states.slice(first).map((state, n) => (
        <div key={rungs[first + n]?.id}>
          Step {first + n + 1} {tickWords[state] ?? state}
        </div>
      ))}
    </div>
  );
  return (
    <Tip text={tip} side="right">
      <span className="flex items-center gap-[3px] px-0.5 py-1">
        {states.slice(first).map((state, n) => (
          <span
            key={rungs[first + n]?.id}
            className="h-[10px] w-[3px] rounded-full"
            style={{
              background: tickColors[state as keyof typeof tickColors] ?? "var(--accent)",
              opacity: state === "done" ? 0.55 : 1,
            }}
          />
        ))}
      </span>
    </Tip>
  );
}

/** The rail: the record and what its life spent, the questions to you that were open when it ends, and the threads
 * as a tree. */
export function Rail({
  held,
  moment,
  selected,
  onSelect,
}: {
  held: Held;
  moment: Moment;
  selected: string;
  onSelect: (thread: string) => void;
}) {
  const owed = moment.threads.filter((one) => moment.owed.has(one.act.id));
  // The tree of the threads draws only the rows that the rail shows, so a record of many threads stays light.
  const nav = useRef<HTMLElement>(null);
  const rows = useVirtualizer({
    count: moment.threads.length,
    getScrollElement: () => nav.current,
    estimateSize: () => 46,
    overscan: 12,
  });
  const spent = Object.values(held.measures).reduce((sum, one) => sum + one.dollars, 0);
  const ceiling = held.measures[held.root]?.ceiling;
  const share = ceiling ? Math.min(spent / ceiling, 1) : 0;
  return (
    <aside className="flex h-full w-[272px] shrink-0 flex-col bg-rail">
      <div data-tauri-drag-region className="h-[46px] shrink-0" />
      <div data-tauri-drag-region className="px-4 pb-3">
        <Menu.Root>
          <Menu.Trigger asChild>
            <button
              type="button"
              className="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left outline-none hover:bg-text/5"
            >
              <span className="grid h-6 w-6 place-items-center rounded-md bg-accent text-[13px] font-semibold text-accent-ink">
                f
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[14px] font-semibold">{held.place}</span>
                <span className="block text-[11.5px] text-faint">
                  {held.logged ? "A record and its log" : "A record, as it kept its facts"}
                </span>
              </span>
              <ChevronDown size={14} className="text-faint" />
            </button>
          </Menu.Trigger>
          <Menu.Portal>
            <Menu.Content
              align="start"
              sideOffset={4}
              className="z-50 min-w-[220px] rounded-xl border border-hair bg-raised p-1 text-[13px] shadow-[var(--shadow)]"
            >
              <Menu.Item
                onSelect={() => void call("close")}
                className="flex cursor-default items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none data-[highlighted]:bg-text/5"
              >
                <FolderOpen size={14} strokeWidth={1.8} />
                Read another record
              </Menu.Item>
            </Menu.Content>
          </Menu.Portal>
        </Menu.Root>
      </div>
      {owed.length > 0 ? (
        <div className="mx-4 mt-4 rounded-xl border border-asks/25 bg-asks/[0.07] p-1.5">
          <div className="px-2 pt-1 pb-1.5 text-[11.5px] font-medium text-asks">
            {owed.length === 1
              ? "One question to you was open when the record ends"
              : `${owed.length} questions to you were open when the record ends`}
          </div>
          {owed.map((thread) => (
            <button
              key={thread.act.id}
              type="button"
              onClick={() => onSelect(thread.act.id)}
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
      <nav ref={nav} className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
        <div className="relative w-full" style={{ height: rows.getTotalSize() }}>
          {rows.getVirtualItems().map((row) => {
            const thread = moment.threads[row.index];
            if (!thread) return null;
            const on = thread.act.id === selected;
            const to = thread.to === "operator" ? "asks you" : chainName(moment, thread.chain);
            return (
              <div
                key={thread.act.id}
                data-index={row.index}
                ref={rows.measureElement}
                className="absolute left-0 w-full"
                style={{ top: row.start }}
              >
                <Tip
                  side="right"
                  text={
                    thread.title.length > 30 ? (
                      <span className="block whitespace-pre-wrap">
                        {clip(String(thread.act.words[1] ?? ""), 400)}
                      </span>
                    ) : null
                  }
                >
                  <button
                    type="button"
                    onClick={() => onSelect(thread.act.id)}
                    className={`group relative flex w-full items-center gap-2.5 rounded-lg py-[7px] pr-1.5 text-left transition-colors duration-150 ${
                      on ? "bg-panel shadow-[var(--shadow)]" : "hover:bg-text/[0.04]"
                    }`}
                    style={{ paddingLeft: 8 + Math.min(thread.depth, 6) * 14 }}
                  >
                    {thread.depth > 0 ? (
                      <span
                        className="absolute top-0 bottom-0 w-px bg-hair"
                        style={{ left: 13 + (Math.min(thread.depth, 6) - 1) * 14 }}
                        aria-hidden
                      />
                    ) : null}
                    <Glyph state={threadState(moment, thread)} size={12} />
                    <span className="min-w-0 flex-1">
                      <span
                        className={`block truncate text-[13.5px] ${on ? "text-text" : "text-muted group-hover:text-text"}`}
                      >
                        {thread.title}
                      </span>
                      <span className="block truncate text-[11.5px] text-faint">{to}</span>
                    </span>
                    <Ticks moment={moment} thread={thread.act.id} />
                  </button>
                </Tip>
              </div>
            );
          })}
        </div>
      </nav>
      <div className="border-t border-hair px-6 py-3.5">
        <div className="flex items-baseline justify-between text-[12px]">
          <span className="text-muted">Spent</span>
          <span className="tabular font-mono text-[12px] text-text">
            {dollars(spent)}
            {ceiling ? <span className="text-faint"> of {dollars(ceiling)}</span> : null}
          </span>
        </div>
        <div className="mt-2 h-[3px] overflow-hidden rounded-full bg-hair">
          <div className="h-full rounded-full bg-accent" style={{ width: `${share * 100}%` }} />
        </div>
      </div>
    </aside>
  );
}
