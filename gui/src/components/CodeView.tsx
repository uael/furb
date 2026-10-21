import { Braces, CircleDot, Pause, Play, Square, StepForward } from "lucide-react";
import { actorName, chainName, type Thread, type View } from "../data.ts";
import { Code } from "./Code.tsx";
import { Glyph } from "./Glyph.tsx";

/** One program opened as code, as a debugger opens it: the whole ladder of a thread as one program, with the line
 * that runs and the value of each act beside its line; the controls over it; and its call stack and its watch. */
export function CodeView({ view, thread }: { view: View; thread: Thread }) {
  const chain = view.chains.find((one) => one.id === thread.chain);
  const paused = chain?.paused ?? false;
  const controls = [
    { Icon: paused ? Play : Pause, label: paused ? "Continue" : "Pause", key: "Space" },
    { Icon: StepForward, label: "Step", key: "S" },
    { Icon: Square, label: "Cancel", key: "X" },
    { Icon: CircleDot, label: "Pause at each step", key: "B" },
    { Icon: Braces, label: "Edit the door", key: "E" },
  ];
  return (
    <div className="flex h-full">
      <section className="flex min-w-0 flex-1 flex-col">
        <div className="flex items-center gap-3 px-6 pt-5 pb-3">
          <Glyph state={thread.state} size={13} title />
          <div className="min-w-0">
            <div className="truncate text-[15px] font-semibold text-text">{thread.title}</div>
            <div className="text-[12px] text-faint">
              <span className="font-mono">{thread.id}</span>
              <span className="px-1.5">on</span>
              {chainName(view, thread.chain)}
              {chain ? <span className="pl-1.5">{actorName(chain.actor)}</span> : null}
            </div>
          </div>
          <div className="ml-auto flex items-center gap-0.5 rounded-xl border border-hair bg-panel p-1 shadow-[var(--shadow)]">
            {controls.map(({ Icon, label, key }) => (
              <button
                key={label}
                type="button"
                title={`${label}  ${key}`}
                className="grid h-7 w-8 place-items-center rounded-lg text-muted transition-colors duration-150 hover:bg-text/5 hover:text-text"
              >
                <Icon size={14.5} strokeWidth={1.9} />
              </button>
            ))}
          </div>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-6 pb-56">
          <div className="rounded-2xl border border-hair bg-panel py-2 pr-4 text-[13px]">
            <Code steps={thread.ladder} wrap />
          </div>
        </div>
      </section>
      <aside className="w-[300px] shrink-0 overflow-y-auto border-l border-hair px-5 pt-5 pb-56">
        <h2 className="text-[12px] font-medium text-faint">Call stack</h2>
        <ul className="mt-2 space-y-0.5">
          {(chain?.stack ?? []).map((frame, n) => (
            <li
              key={frame.act}
              className={`flex items-baseline gap-2 rounded-lg px-2 py-1.5 font-mono text-[12px] ${n === 0 ? "bg-text/[0.05] text-text" : "text-muted"}`}
            >
              <span>{frame.act}</span>
              <span className="truncate font-sans text-[11.5px] text-faint">
                {chainName(view, frame.chain)}
                {frame.awaits ? `, awaits ${frame.awaits}` : ""}
              </span>
            </li>
          ))}
          {chain?.stack.length ? null : <li className="px-2 text-[12.5px] text-faint">No word waits</li>}
        </ul>
        <h2 className="mt-6 text-[12px] font-medium text-faint">Watch</h2>
        <dl className="mt-2 space-y-px">
          {(chain?.watch ?? [])
            .filter((one) => !one.value.startsWith("Act["))
            .slice(0, 16)
            .map((one) => (
              <div
                key={one.name}
                className="grid grid-cols-[minmax(0,0.9fr)_minmax(0,1.3fr)] gap-3 rounded-md px-2 py-1 font-mono text-[11.5px]"
              >
                <dt className={`truncate ${one.fresh ? "text-accent" : "text-muted"}`}>{one.name}</dt>
                <dd className="truncate text-text" title={one.value}>
                  {one.value}
                </dd>
              </div>
            ))}
        </dl>
        <h2 className="mt-6 text-[12px] font-medium text-faint">Breakpoints</h2>
        <p className="mt-2 px-2 text-[12.5px] leading-relaxed text-faint">
          None yet. A breakpoint pauses a chain at each step, at each act of a kind, or past a cost.
        </p>
      </aside>
    </div>
  );
}
