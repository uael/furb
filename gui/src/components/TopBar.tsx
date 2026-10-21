import { Braces, MessagesSquare, Workflow } from "lucide-react";
import { chainName, type Moment, threadState } from "../life.ts";
import { Glyph } from "./Glyph.tsx";

export type Mode = "thread" | "programs" | "code";

const modes: { id: Mode; label: string; icon: typeof Braces; key: string }[] = [
  { id: "thread", label: "Thread", icon: MessagesSquare, key: "1" },
  { id: "programs", label: "Programs", icon: Workflow, key: "2" },
  { id: "code", label: "Code", icon: Braces, key: "3" },
];

/** The bar over the view: the thread that the operator reads, and the switch of views. Each empty part of it moves
 * the window. */
export function TopBar({
  moment,
  thread,
  mode,
  onMode,
}: {
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
      <div data-tauri-drag-region className="flex-1" />
    </header>
  );
}
