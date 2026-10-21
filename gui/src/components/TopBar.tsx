import { Braces, MessagesSquare, Workflow } from "lucide-react";
import type { View } from "../data.ts";
import { chainName } from "../data.ts";
import { Glyph } from "./Glyph.tsx";

export type Mode = "thread" | "programs" | "code";

const modes: { id: Mode; label: string; icon: typeof Braces; key: string }[] = [
  { id: "thread", label: "Thread", icon: MessagesSquare, key: "1" },
  { id: "programs", label: "Programs", icon: Workflow, key: "2" },
  { id: "code", label: "Code", icon: Braces, key: "3" },
];

/** The life on its axis of time: one tick for each answer of a model, and the cursor, which is now while the life is
 * live. */
function Time({ view }: { view: View }) {
  const { at, len } = view.life;
  const live = at >= len;
  return (
    <div className="flex items-center gap-2.5">
      <div className="relative h-6 w-[148px]" title={`Fact ${at} of ${len}`}>
        <div className="absolute top-1/2 right-0 left-0 h-px bg-hair-strong" />
        <div
          className="absolute top-1/2 left-0 h-px bg-muted"
          style={{ width: `${(at / Math.max(len, 1)) * 100}%` }}
        />
        <div
          className="absolute top-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-bg bg-text"
          style={{ left: `${(at / Math.max(len, 1)) * 100}%` }}
        />
      </div>
      <span
        className={`flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[11.5px] ${
          live ? "text-done" : "bg-accent/15 text-accent"
        }`}
      >
        {live ? <span className="h-1.5 w-1.5 rounded-full bg-done" /> : null}
        {live ? "Live" : "Back to now"}
      </span>
    </div>
  );
}

/** The bar over the view: where the operator is, the switch of views, and time. */
export function TopBar({
  view,
  thread,
  mode,
  onMode,
}: {
  view: View;
  thread: string;
  mode: Mode;
  onMode: (mode: Mode) => void;
}) {
  const held = view.threads.find((one) => one.id === thread);
  return (
    <header
      data-tauri-drag-region
      className="flex h-[46px] shrink-0 items-center gap-4 border-b border-hair px-5"
    >
      <div className="flex min-w-0 flex-1 items-center gap-2 text-[13px]">
        {held ? (
          <>
            <Glyph state={held.state} size={11} />
            <span className="shrink-0 text-faint">{chainName(view, held.chain)}</span>
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
      <div className="flex flex-1 justify-end">
        <Time view={view} />
      </div>
    </header>
  );
}
