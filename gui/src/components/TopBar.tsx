import type { Held, Moment } from "../life.ts";
import { chainName, threadState } from "../life.ts";
import { Glyph } from "./Glyph.tsx";
import { Tip } from "./Tip.tsx";

/** The bar over the view: the thread that the operator reads, and the mark that says that the app is a preview that
 * reads records. Each empty part of it moves the window. */
export function TopBar({ held, moment, thread }: { held: Held; moment: Moment; thread: string }) {
  const shown = moment.threads.find((one) => one.act.id === thread);
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
      {held.read ? null : (
        <span className="flex items-center gap-2 text-[12px] text-faint">
          <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-accent" />
          Reading the record
        </span>
      )}
      <Tip
        side="bottom"
        text="This preview reads a record and runs nothing: no word runs again, no model is asked, and no command runs."
      >
        <span className="rounded-md border border-accent/40 bg-accent/10 px-2 py-0.5 text-[11.5px] font-medium text-accent">
          Preview: read only
        </span>
      </Tip>
    </header>
  );
}
