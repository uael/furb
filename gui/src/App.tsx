import { useEffect, useState } from "react";
import { CodeView } from "./components/CodeView.tsx";
import { Composer } from "./components/Composer.tsx";
import { GraphView } from "./components/GraphView.tsx";
import { Rail } from "./components/Rail.tsx";
import { ThreadView } from "./components/ThreadView.tsx";
import { type Mode, TopBar } from "./components/TopBar.tsx";
import { fixture, type View } from "./data.ts";

const query = new URLSearchParams(window.location.search);
const tauri = "__TAURI_INTERNALS__" in window;

/** The view of the life: the one the core of the app hands over in its window, and the demo in a browser. */
function useView(): View {
  const [view, setView] = useState<View>(fixture);
  useEffect(() => {
    if (!tauri) return;
    let gone = false;
    let stop: (() => void) | undefined;
    void (async () => {
      const { invoke } = await import("@tauri-apps/api/core");
      const { listen } = await import("@tauri-apps/api/event");
      const first = await invoke<View | null>("view");
      if (!gone && first) setView(first);
      stop = await listen<View>("view", (event) => setView(event.payload));
    })();
    return () => {
      gone = true;
      stop?.();
    };
  }, []);
  return view;
}

export function App() {
  const view = useView();
  const [mode, setMode] = useState<Mode>((query.get("screen") as Mode | null) ?? "thread");
  const [thread, setThread] = useState<string>(
    query.get("thread") ?? view.life.owed[0] ?? view.threads[0]?.id ?? "",
  );
  const [theme] = useState(query.get("theme") ?? "ink");
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
  useEffect(() => {
    const keys = (event: KeyboardEvent) => {
      if (!event.metaKey) return;
      const picked = ({ "1": "thread", "2": "programs", "3": "code" } as const)[event.key as "1" | "2" | "3"];
      if (picked) {
        event.preventDefault();
        setMode(picked);
      }
    };
    window.addEventListener("keydown", keys);
    return () => window.removeEventListener("keydown", keys);
  }, []);
  const held = view.threads.find((one) => one.id === thread) ?? view.threads[0];
  const chain = held?.chain ?? view.life.root;
  return (
    <div className="flex h-full">
      <Rail
        view={view}
        selected={held?.id ?? ""}
        onSelect={setThread}
        onNew={() => setThread(view.threads[0]?.id ?? "")}
      />
      <main className="relative flex min-w-0 flex-1 flex-col border-l border-hair bg-bg">
        <TopBar view={view} thread={held?.id ?? ""} mode={mode} onMode={setMode} />
        <div className="relative min-h-0 flex-1">
          {held && mode === "thread" ? (
            <ThreadView key={held.id} view={view} thread={held} onSelect={setThread} />
          ) : null}
          {mode === "programs" ? (
            <GraphView
              view={view}
              picked={chain}
              onPick={(id) => {
                const first = view.chains.find((one) => one.id === id)?.threads[0];
                if (first) setThread(first);
              }}
            />
          ) : null}
          {held && mode === "code" ? <CodeView view={view} thread={held} /> : null}
          <Composer view={view} thread={held?.id ?? ""} chain={chain} />
        </div>
      </main>
    </div>
  );
}
