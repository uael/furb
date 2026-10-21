import { python } from "@codemirror/lang-python";
import { syntaxHighlighting } from "@codemirror/language";
import { plain, program, stack, type Thread, watch } from "@furb/engine/life";
import CodeMirror from "@uiw/react-codemirror";
import { Braces, CircleDot, Pause, Play, Square, StepForward, X } from "lucide-react";
import { useEffect, useState } from "react";
import { actorName, type Break, send } from "../data.ts";
import { chainName, chainState, type Held, type Moment, threadState, useTold } from "../life.ts";
import { steps } from "../steps.ts";
import { Code, highlight } from "./Code.tsx";
import { Glyph } from "./Glyph.tsx";

/** The door of a prompt, open: the program of its ladder in an editor. Its write runs the new program in place of the
 * old one, from the first line that differs, as the door of a prompt does. */
function Door({ thread, text, onClose }: { thread: Thread; text: string; onClose: () => void }) {
  const [draft, setDraft] = useState(text);
  const write = () => {
    send({ kind: "door", value: { prompt: thread.act.id, on: thread.chain, program: draft } });
    onClose();
  };
  return (
    <div className="overflow-hidden rounded-2xl border border-accent/50 bg-panel shadow-[var(--shadow)]">
      <div className="flex items-center gap-3 border-b border-hair px-4 py-2.5">
        <Braces size={14} className="text-accent" strokeWidth={1.9} />
        <div className="min-w-0 flex-1">
          <div className="text-[13px] font-medium text-text">The door of {thread.act.id}</div>
          <div className="text-[12px] text-faint">
            The chain keeps the part that stays the same, cancels the old word where it differs, and runs your
            program from there.
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="rounded-lg px-2.5 py-1 text-[12.5px] text-muted hover:bg-text/5 hover:text-text"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={write}
          disabled={draft === text}
          className="rounded-lg bg-accent px-3 py-1 text-[12.5px] font-medium text-accent-ink disabled:opacity-40"
        >
          Write the door
        </button>
      </div>
      <div className="door py-2 pr-3 text-[13px]">
        <CodeMirror
          value={draft}
          onChange={setDraft}
          theme="none"
          autoFocus
          extensions={[python(), syntaxHighlighting(highlight)]}
          basicSetup={{ foldGutter: false, highlightActiveLineGutter: false, autocompletion: false }}
        />
      </div>
    </div>
  );
}

/** One program opened as code, as a debugger opens it: the whole ladder of a thread as one program, with the line
 * that runs and the value of each act beside its line; the controls over it; its door; and its call stack, its watch
 * and its breakpoints. */
export function CodeView({
  held,
  moment,
  thread,
  door: opened,
}: {
  held: Held;
  moment: Moment;
  thread: Thread;
  door?: boolean;
}) {
  const [door, setDoor] = useState(Boolean(opened));
  const chain = thread.chain;
  const live = held.works && moment.now;
  const paused = chainState(moment, chain) === "paused";
  const breaks = held.breaks;
  const stepping = breaks.some((one) => one.kind === "chain" && one.value === chain);
  const told = useTold(chain, held.count);
  const groups = watch(told, moment.at);
  const frames = stack(chain, moment.acts);
  const ladder = steps(moment, thread.act, held.directory, held.streams);
  const text = program(thread.act.id, moment.acts, moment.word);
  const toggle = () => {
    const next: Break[] = stepping
      ? breaks.filter((one) => !(one.kind === "chain" && one.value === chain))
      : [...breaks, { kind: "chain", value: chain }];
    send({ kind: "breaks", value: next });
  };
  const controls = [
    {
      Icon: paused ? Play : Pause,
      label: paused ? "Continue" : "Pause",
      key: " ",
      run: () => send({ kind: paused ? "wake" : "pause", value: chain }),
    },
    { Icon: StepForward, label: "Step", key: "s", run: () => send({ kind: "step", value: chain }) },
    {
      Icon: Square,
      label: "Cancel this thread",
      key: "x",
      run: () => send({ kind: "cancel", value: thread.act.id }),
    },
    {
      Icon: CircleDot,
      label: stepping ? "Pause at each step: on" : "Pause at each step",
      key: "b",
      run: toggle,
      on: stepping,
    },
    { Icon: Braces, label: "Edit the door", key: "e", run: () => setDoor(true), on: door },
  ];
  useEffect(() => {
    if (!live) return;
    const keys = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (event.metaKey || event.ctrlKey || target.closest("input, textarea, .cm-editor")) return;
      const hit = controls.find((one) => one.key === event.key.toLowerCase());
      if (hit) {
        event.preventDefault();
        hit.run();
      }
    };
    window.addEventListener("keydown", keys);
    return () => window.removeEventListener("keydown", keys);
  });
  const actor = held.measures[chain]?.actor;
  return (
    <div className="flex h-full">
      <section className="flex min-w-0 flex-1 flex-col">
        <div className="flex items-center gap-3 px-6 pt-5 pb-3">
          <Glyph state={threadState(moment, thread)} size={13} />
          <div className="min-w-0">
            <div className="truncate text-[15px] font-semibold text-text">{thread.title}</div>
            <div className="text-[12px] text-faint">
              <span className="font-mono">{thread.act.id}</span>
              <span className="px-1.5">on</span>
              {chainName(moment, chain)}
              {actor ? <span className="pl-1.5">{actorName(actor)}</span> : null}
            </div>
          </div>
          {live ? (
            <div className="ml-auto flex items-center gap-0.5 rounded-xl border border-hair bg-panel p-1 shadow-[var(--shadow)]">
              {controls.map(({ Icon, label, key, run, on }) => (
                <button
                  key={label}
                  type="button"
                  onClick={run}
                  title={`${label}  ${key === " " ? "Space" : key.toUpperCase()}`}
                  className={`grid h-7 w-8 place-items-center rounded-lg transition-colors duration-150 hover:bg-text/5 ${on ? "bg-accent/15 text-accent" : "text-muted hover:text-text"}`}
                >
                  <Icon size={14.5} strokeWidth={1.9} />
                </button>
              ))}
            </div>
          ) : null}
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-6 pb-56">
          {door && live ? (
            <Door thread={thread} text={text} onClose={() => setDoor(false)} />
          ) : (
            <div className="rounded-2xl border border-hair bg-panel py-2 pr-4 text-[13px]">
              <Code steps={ladder} wrap />
            </div>
          )}
        </div>
      </section>
      <aside className="w-[300px] shrink-0 overflow-y-auto border-l border-hair px-5 pt-5 pb-56">
        <h2 className="text-[12px] font-medium text-faint">Call stack</h2>
        <ul className="mt-2 space-y-0.5">
          {frames.map((frame, n) => (
            <li
              key={frame.act}
              className={`flex items-baseline gap-2 rounded-lg px-2 py-1.5 font-mono text-[12px] ${n === 0 ? "bg-text/[0.05] text-text" : "text-muted"}`}
            >
              <span>{frame.act}</span>
              <span className="truncate font-sans text-[11.5px] text-faint">
                {chainName(moment, frame.chain)}
                {frame.awaits ? `, awaits ${frame.awaits}` : ""}
              </span>
            </li>
          ))}
          {frames.length ? null : <li className="px-2 text-[12.5px] text-faint">No word waits</li>}
        </ul>
        <h2 className="mt-6 text-[12px] font-medium text-faint">Watch</h2>
        <div className="mt-2 space-y-2.5">
          {groups.slice(0, 10).map((group) => (
            <dl key={group.act ?? "names"} className="space-y-px">
              {group.act ? (
                <dt className="px-2 pb-0.5 font-mono text-[11px] text-faint">{group.act}</dt>
              ) : null}
              {group.items.slice(0, 8).map((one) => (
                <div
                  key={one.name}
                  className="grid grid-cols-[minmax(0,0.8fr)_minmax(0,1.4fr)] gap-3 rounded-md px-2 py-1 font-mono text-[11.5px]"
                >
                  <dt className={`truncate ${one.fresh ? "text-accent" : "text-muted"}`}>
                    {group.act ? one.name.slice(group.act.length + 1) : one.name}
                  </dt>
                  <dd className="truncate text-text" title={plain(one.value, held.directory)}>
                    {plain(one.value, held.directory)}
                  </dd>
                </div>
              ))}
            </dl>
          ))}
          {groups.length ? null : (
            <p className="px-2 text-[12.5px] text-faint">The chain told no value yet</p>
          )}
        </div>
        <h2 className="mt-6 text-[12px] font-medium text-faint">Breakpoints</h2>
        <ul className="mt-2 space-y-0.5">
          {breaks.map((one) => (
            <li
              key={`${one.kind}-${one.value}`}
              className="flex items-center gap-2 rounded-lg px-2 py-1 text-[12.5px] text-text"
            >
              <CircleDot size={12} className="text-accent" />
              <span className="flex-1 truncate">
                {one.kind === "chain"
                  ? `Each step of ${chainName(moment, String(one.value))}`
                  : one.kind === "kind"
                    ? `Each ${one.value}`
                    : `Past $${one.value}`}
              </span>
              {live ? (
                <button
                  type="button"
                  aria-label="Remove the breakpoint"
                  onClick={() => send({ kind: "breaks", value: breaks.filter((other) => other !== one) })}
                  className="text-faint hover:text-text"
                >
                  <X size={12} />
                </button>
              ) : null}
            </li>
          ))}
          {breaks.length ? null : (
            <li className="px-2 text-[12.5px] leading-relaxed text-faint">
              None. Pause at each step sets one on this chain.
            </li>
          )}
        </ul>
      </aside>
    </div>
  );
}
