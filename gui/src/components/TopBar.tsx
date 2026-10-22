import { Braces, MessagesSquare, Workflow } from "lucide-react";
import { chainName, type Held, type Moment, threadState } from "../life.ts";
import { Glyph } from "./Glyph.tsx";
import { Tip } from "./Tip.tsx";

export type Mode = "thread" | "programs" | "code";

const modes: { id: Mode; label: string; icon: typeof Braces; key: string }[] = [
  { id: "thread", label: "Thread", icon: MessagesSquare, key: "1" },
  { id: "programs", label: "Programs", icon: Workflow, key: "2" },
  { id: "code", label: "Code", icon: Braces, key: "3" },
];

/** The bar over the view: the thread that the operator reads, the switch of views, and for a record whether the core
 * still reads it and the mark that says that it runs nothing. Each empty part of it moves the window. */
export function TopBar({
  life,
  moment,
  thread,
  mode,
  onMode,
}: {
  life: Held;
  moment: Moment;
  thread: string;
  mode: Mode;
  onMode: (mode: Mode) => void;
}) {
  const held = moment.threads.find((one) => one.act.id === thread);
  return (
    <header data-tauri-drag-region className="flex h-[46px] shrink-0 items-center gap-4 px-5">
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-2 text-[13px]">
        {held ? (
          <>
            <Glyph state={threadState(moment, held)} size={11} />
            <span className="shrink-0 text-faint">{chainName(moment, held.chain)}</span>
            <span className="text-faint">/</span>
            <span className="truncate font-medium text-text">{held.title}</span>
          </>
        ) : null}
      </div>
      <nav className="flex items-center rounded-[10px] bg-text/[0.05] p-[3px]">
        {modes.map(({ id, label, icon: Icon, key }) => (
          <button
            key={id}
            type="button"
            onClick={() => onMode(id)}
            title={`${label}  ⌘${key}`}
            className={`flex items-center gap-1.5 rounded-[8px] px-3 py-1 text-[12.5px] transition-all duration-150 ${
              mode === id ? "bg-raised text-text shadow-[var(--shadow)]" : "text-muted hover:text-text"
            }`}
          >
            <Icon size={13.5} strokeWidth={1.8} />
            {label}
          </button>
        ))}
      </nav>
      <div data-tauri-drag-region className="flex flex-1 items-center justify-end gap-4">
        {life.read ? null : (
          <span className="flex items-center gap-2 text-[12px] text-faint">
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-accent" />
            Reading the record
          </span>
        )}
        {life.works ? null : (
          <Tip
            side="bottom"
            text="A record opens with no life: no word runs again, no model is asked, and no command runs."
          >
            <span className="rounded-md border border-accent/40 bg-accent/10 px-2 py-0.5 text-[11.5px] font-medium text-accent">
              Read only
            </span>
          </Tip>
        )}
      </div>
    </header>
  );
}
