import type { Held, Moment } from "../life.ts";
import { chainName, threadState } from "../life.ts";
import { Glyph } from "./Glyph.tsx";
import { Tip } from "./Tip.tsx";

/** The bar over the view: the thread that the operator reads, while the host opens the life, and the mark of a record
 * that opens to read. Each empty part of it moves the window. */
export function TopBar({ held, moment, thread }: { held: Held; moment: Moment; thread: string }) {
  const shown = moment.threads.find((one) => one.act.id === thread);
  const life = held.life;
  return (
    <header data-tauri-drag-region className="flex h-[46px] shrink-0 items-center gap-4 px-5">
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-2 text-[13px]">
        {shown ? (
          <>
            <Glyph state={threadState(moment, shown)} size={11} />
            <span className="shrink-0 text-faint">{chainName(moment, shown.chain)}</span>
            <span className="text-faint">/</span>
            <span className="truncate font-medium text-text">{shown.title}</span>
          </>
        ) : null}
      </div>
      {life?.loading ? (
        <span className="flex items-center gap-2 text-[12px] text-faint">
          <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-accent" />
          Opening the life
        </span>
      ) : null}
      {life?.notice || life?.error ? (
        <span className={`max-w-[40%] truncate text-[12px] ${life.error ? "text-failed" : "text-muted"}`}>
          {life.error || life.notice}
        </span>
      ) : null}
      {life?.reading ? (
        <Tip
          side="bottom"
          text={`A life on this record was refused, so it opens to read: no word runs again, no model is asked, and no command runs. ${life.reading.why}`}
        >
          <span className="rounded-md border border-accent/40 bg-accent/10 px-2 py-0.5 text-[11.5px] font-medium text-accent">
            Read only
          </span>
        </Tip>
      ) : null}
    </header>
  );
}
