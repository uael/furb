import * as Tooltip from "@radix-ui/react-tooltip";
import { useEffect, useState } from "react";
import { Picker } from "./components/Picker.tsx";
import { Rail } from "./components/Rail.tsx";
import { ThreadView } from "./components/ThreadView.tsx";
import { TopBar } from "./components/TopBar.tsx";
import { call, tauri } from "./data.ts";
import { useHeld, useMoment } from "./life.ts";

const query = new URLSearchParams(window.location.search);

export function App() {
  const held = useHeld();
  const moment = useMoment(held);
  const [picked, setPicked] = useState<string>(query.get("thread") ?? "");
  const [theme] = useState(query.get("theme") ?? "ink");
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  // Once the window took the last part of the record and painted it, it tells the core what it drew, which a check of
  // the app prints.
  const whole = held.state === "open" && held.read;
  useEffect(() => {
    if (!tauri || !whole) return;
    const counts = { chains: moment.chains.length, threads: moment.threads.length, acts: moment.acts.length };
    const frame = requestAnimationFrame(() => void call("drew", counts).catch(() => undefined));
    return () => cancelAnimationFrame(frame);
  }, [whole, moment.chains.length, moment.threads.length, moment.acts.length]);
  if (held.state !== "open" || query.get("picker"))
    return (
      <Picker key={held.len} said={held.said} opening={held.state === "opening" ? held.what : undefined} />
    );
  const thread =
    moment.threads.find((one) => one.act.id === picked) ??
    moment.threads.find((one) => moment.owed.has(one.act.id)) ??
    moment.threads[0];
  return (
    <Tooltip.Provider>
      <div className="flex h-full">
        <Rail held={held} moment={moment} selected={thread?.act.id ?? ""} onSelect={setPicked} />
        <main className="relative flex min-w-0 flex-1 flex-col border-l border-hair bg-bg">
          <TopBar held={held} moment={moment} thread={thread?.act.id ?? ""} />
          <div className="relative min-h-0 flex-1">
            {thread ? (
              <ThreadView
                key={thread.act.id}
                held={held}
                moment={moment}
                thread={thread}
                onSelect={setPicked}
              />
            ) : (
              <div className="grid h-full place-items-center pb-40 text-[14px] text-faint">
                {held.read ? "This record holds no thread." : "Reading the record"}
              </div>
            )}
          </div>
        </main>
      </div>
    </Tooltip.Provider>
  );
}
