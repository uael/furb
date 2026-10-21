import { threadOf } from "@furb/engine/life";
import * as Tooltip from "@radix-ui/react-tooltip";
import { useEffect, useRef, useState } from "react";
import { CodeView } from "./components/CodeView.tsx";
import { Composer } from "./components/Composer.tsx";
import { GraphView } from "./components/GraphView.tsx";
import { Picker } from "./components/Picker.tsx";
import { Rail } from "./components/Rail.tsx";
import { Scrubber } from "./components/Scrubber.tsx";
import { ThreadView } from "./components/ThreadView.tsx";
import { type Mode, TopBar } from "./components/TopBar.tsx";
import { call, tauri } from "./data.ts";
import { useHeld, useMoment } from "./life.ts";

const query = new URLSearchParams(window.location.search);

export function App() {
  const held = useHeld();
  const [at, setAt] = useState<number | null>(null);
  const moment = useMoment(held, at);
  const [mode, setMode] = useState<Mode>((query.get("screen") as Mode | null) ?? "thread");
  const [picked, setPicked] = useState<string>(query.get("thread") ?? "");
  const [target, setTarget] = useState<string>("");
  const composer = useRef<HTMLTextAreaElement>(null);
  const [theme] = useState(query.get("theme") ?? "ink");
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  // A capture of the past puts the cursor at a share of the steps of the demo, as past=0.5 for the middle.
  const past = query.get("past");
  useEffect(() => {
    if (past === null || !moment.marks.length) return;
    const steps = moment.marks.filter((one) => one.kind === "step");
    setAt(steps[Math.round(Number(past) * (steps.length - 1))]?.at ?? null);
  }, [past, moment.marks]);
  useEffect(() => {
    const keys = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      const typing = target.closest("input, textarea, .cm-editor");
      if (event.metaKey) {
        const mode = ({ "1": "thread", "2": "programs", "3": "code" } as const)[event.key as "1" | "2" | "3"];
        if (mode) {
          event.preventDefault();
          setMode(mode);
        }
        return;
      }
      if (typing) return;
      if (event.key === "Escape") setAt(null);
      if (event.key === "[" || event.key === "]") {
        const cursor = moment.at;
        const mark =
          event.key === "["
            ? moment.marks.findLast((one) => one.at < cursor)
            : moment.marks.find((one) => one.at > cursor);
        setAt(!mark || mark.at >= held.len ? null : mark.at);
      }
    };
    window.addEventListener("keydown", keys);
    return () => window.removeEventListener("keydown", keys);
  }, [moment.at, moment.marks, held.len]);
  // The window tells the core what it drew, which a check of the app reads.
  useEffect(() => {
    if (tauri && held.state === "open")
      void call("drew", { threads: moment.threads.length, chains: moment.chains.length }).catch(
        () => undefined,
      );
  }, [held.state, moment.threads.length, moment.chains.length]);
  if (held.state !== "open" || query.get("picker"))
    return <Picker said={held.said} opening={held.state === "opening" ? held.what : undefined} />;
  const thread =
    moment.threads.find((one) => one.act.id === picked) ??
    moment.threads.find((one) => moment.owed.has(one.act.id)) ??
    moment.threads[0];
  const chain = target || thread?.chain || held.root;
  // A question to the operator has no ladder of its own: its code is the ladder of the thread whose rung asked it.
  const asker = thread?.to === "operator" ? threadOf(thread.act, moment.rows) : undefined;
  const coded = moment.threads.find((one) => one.act.id === asker) ?? thread;
  const answering = mode === "thread" && thread !== undefined && moment.owed.has(thread.act.id);
  const select = (id: string) => {
    setPicked(id);
    setTarget("");
  };
  return (
    <Tooltip.Provider>
      <div className="flex h-full">
        <Rail
          held={held}
          moment={moment}
          selected={thread?.act.id ?? ""}
          onSelect={select}
          onNew={() => {
            setTarget(held.root);
            composer.current?.focus();
          }}
        />
        <main className="relative flex min-w-0 flex-1 flex-col border-l border-hair bg-bg">
          <TopBar moment={moment} thread={thread?.act.id ?? ""} mode={mode} onMode={setMode} />
          <Scrubber moment={moment} len={held.len} onTravel={setAt} />
          <div className="relative min-h-0 flex-1">
            {thread && mode === "thread" ? (
              <ThreadView key={thread.act.id} held={held} moment={moment} thread={thread} onSelect={select} />
            ) : null}
            {mode === "programs" ? (
              <GraphView
                held={held}
                moment={moment}
                picked={thread?.chain ?? held.root}
                zoom={(query.get("zoom") as "in" | "out" | null) ?? undefined}
                onPick={(id) => {
                  const first = moment.threads.find((one) => one.chain === id);
                  if (first) select(first.act.id);
                }}
                onCode={(id) => {
                  const first = moment.threads.findLast((one) => one.chain === id);
                  if (first) select(first.act.id);
                  setMode("code");
                }}
              />
            ) : null}
            {coded && mode === "code" ? (
              <CodeView
                key={coded.act.id}
                held={held}
                moment={moment}
                thread={coded}
                door={query.get("door") === "1"}
              />
            ) : null}
            {!thread && mode !== "programs" ? (
              <div className="grid h-full place-items-center pb-40 text-[14px] text-faint">
                No thread yet. Ask the life below.
              </div>
            ) : null}
            {answering ? null : (
              <Composer
                ref={composer}
                held={held}
                moment={moment}
                thread={thread?.act.id ?? ""}
                chain={chain}
                onChain={setTarget}
                inset={mode === "code" ? 300 : 0}
              />
            )}
          </div>
        </main>
      </div>
    </Tooltip.Provider>
  );
}
