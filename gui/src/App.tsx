import * as Tooltip from "@radix-ui/react-tooltip";
import { useEffect, useRef, useState } from "react";
import { Composer } from "./components/Composer.tsx";
import { Picker } from "./components/Picker.tsx";
import { Rail } from "./components/Rail.tsx";
import { ThreadView } from "./components/ThreadView.tsx";
import { TopBar } from "./components/TopBar.tsx";
import { core, tauri } from "./data.ts";
import { chainName, useHeld, useMoment } from "./life.ts";

const query = new URLSearchParams(window.location.search);

export function App() {
  const held = useHeld();
  const moment = useMoment(held);
  const life = held.life;
  const [picked, setPicked] = useState<string>(query.get("thread") ?? "");
  const [picker, setPicker] = useState(Boolean(query.get("picker")));
  // A message that the operator sent: the window shows its thread once the life holds it.
  const [sent, setSent] = useState<{ chain: string; after: number } | undefined>();
  const [theme] = useState(query.get("theme") ?? "ink");
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  // A life that opens after another one shows itself, in place of the picker.
  const record = useRef<string | undefined>(undefined);
  useEffect(() => {
    if (!life?.record) return;
    if (record.current !== undefined && record.current !== life.record) setPicker(false);
    record.current = life.record;
  }, [life?.record]);
  useEffect(() => {
    if (!sent) return;
    const mine = moment.threads.findLast(
      (one) => one.act.by === "operator" && one.chain === sent.chain && one.act.began > sent.after,
    );
    if (mine) {
      setPicked(mine.act.id);
      setSent(undefined);
    }
  }, [sent, moment.threads]);
  // Once the window took the whole life and painted it, it tells the core what it drew, which a check of the app
  // prints.
  const whole = Boolean(life && !life.loading);
  useEffect(() => {
    if (!tauri || !whole) return;
    const counts = { chains: moment.chains.length, threads: moment.threads.length, acts: moment.acts.length };
    const frame = requestAnimationFrame(() => void core("drew", counts).catch(() => undefined));
    return () => cancelAnimationFrame(frame);
  }, [whole, moment.chains.length, moment.threads.length, moment.acts.length]);
  if (!life || picker) return <Picker held={held} onBack={life ? () => setPicker(false) : undefined} />;
  const thread =
    moment.threads.find((one) => one.act.id === picked) ??
    moment.threads.find((one) => moment.owed.has(one.act.id)) ??
    moment.threads.findLast((one) => one.act.by === "operator") ??
    moment.threads.at(-1);
  // The composer sends to the chain of the thread shown, and to the root when the life holds no thread yet.
  const chain = thread?.chain ?? life.root;
  return (
    <Tooltip.Provider>
      <div className="flex h-full">
        <Rail
          held={held}
          moment={moment}
          selected={thread?.act.id ?? ""}
          onSelect={setPicked}
          onPicker={() => setPicker(true)}
        />
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
              <div className="grid h-full place-items-center pb-24 text-[14px] text-faint">
                {life.loading
                  ? "Opening the life"
                  : life.reading || life.recorded
                    ? "This record holds no thread."
                    : "Send a message to start the work."}
              </div>
            )}
          </div>
          <Composer
            held={held}
            chain={chain}
            label={chainName(moment, chain)}
            onSent={(to) => setSent({ chain: to, after: moment.acts.at(-1)?.began ?? -1 })}
          />
        </main>
      </div>
    </Tooltip.Provider>
  );
}
