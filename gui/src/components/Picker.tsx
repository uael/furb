import { open as pickOne } from "@tauri-apps/plugin-dialog";
import { FileClock, FolderOpen, FolderPlus, Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { call, demo, type Pick, type Places, tauri } from "../data.ts";

/** How long ago a time was, in words. */
function ago(millis: number): string {
  const minutes = Math.round((Date.now() - millis) / 60000);
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return new Date(millis).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

/** A size in bytes, in words. */
function size(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/** The first screen of the app: the projects that the TUI and the app know, each with its records. A project opens
 * a new life, a record of it opens its life again to go on, or opens it to read, which asks no model and runs no
 * command. */
export function Picker({ said, opening }: { said: string[]; opening?: string }) {
  const [places, setPlaces] = useState<Places>(() => demo.places ?? { projects: [] });
  const [error, setError] = useState("");
  useEffect(() => {
    if (tauri) void call<Places>("places").then((got) => got && setPlaces(got));
  }, []);
  const open = (pick: Pick) => {
    setError("");
    void call("open", { pick }).catch((why: unknown) => setError(String(why)));
  };
  const folder = async () => {
    const directory = await pickOne({ directory: true, title: "Open a project" });
    if (typeof directory === "string") open({ kind: "project", value: { directory, record: null } });
  };
  const record = async () => {
    const path = await pickOne({
      title: "Read a record",
      filters: [{ name: "Record", extensions: ["jsonl"] }],
    });
    if (typeof path === "string") open({ kind: "record", value: path });
  };
  const last = said.at(-1);
  return (
    <div data-tauri-drag-region className="flex h-full justify-center overflow-y-auto bg-bg">
      <div className="w-full max-w-[760px] px-10 pt-24 pb-16">
        <div data-tauri-drag-region className="flex items-center gap-3">
          <span className="grid h-9 w-9 place-items-center rounded-xl bg-accent text-[18px] font-semibold text-accent-ink">
            f
          </span>
          <h1 className="text-[22px] font-semibold tracking-[-0.01em]">Open a life</h1>
        </div>
        <p className="mt-2 text-[14px] text-muted">
          A project opens a new life, on the models of the catalog. A record goes on where its life stopped,
          or opens to read, with travel in time, which asks no model and runs no command.
        </p>
        {opening ? (
          <div className="mt-6 flex items-center gap-2.5 rounded-xl border border-hair bg-panel px-4 py-3 text-[13.5px]">
            <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
            Opening {opening}
          </div>
        ) : null}
        {error || (last && !opening) ? (
          <div className="mt-6 rounded-xl border border-failed/30 bg-failed/[0.07] px-4 py-3 text-[13.5px] text-failed">
            {error || last}
          </div>
        ) : null}
        <div className="mt-8 flex gap-2">
          <button
            type="button"
            disabled={!tauri}
            onClick={() => void folder()}
            className="flex items-center gap-2 rounded-xl border border-hair bg-panel px-3.5 py-2 text-[13.5px] shadow-[var(--shadow)] hover:border-hair-strong disabled:opacity-50"
          >
            <FolderPlus size={15} strokeWidth={1.8} />
            Open a project
          </button>
          <button
            type="button"
            disabled={!tauri}
            onClick={() => void record()}
            className="flex items-center gap-2 rounded-xl border border-hair bg-panel px-3.5 py-2 text-[13.5px] shadow-[var(--shadow)] hover:border-hair-strong disabled:opacity-50"
          >
            <FileClock size={15} strokeWidth={1.8} />
            Read a record
          </button>
        </div>
        <div className="mt-10 space-y-8">
          {places.projects.map((project) => (
            <section key={project.directory}>
              <div className="flex items-center gap-3">
                <FolderOpen size={15} className="text-faint" strokeWidth={1.8} />
                <h2 className="text-[15px] font-semibold">{project.name}</h2>
                <span className="truncate text-[12px] text-faint">{project.directory}</span>
                <button
                  type="button"
                  onClick={() =>
                    open({ kind: "project", value: { directory: project.directory, record: null } })
                  }
                  className="ml-auto flex shrink-0 items-center gap-1.5 rounded-lg px-2.5 py-1 text-[12.5px] text-accent hover:bg-accent/10"
                >
                  <Plus size={14} strokeWidth={2} />
                  New life
                </button>
              </div>
              <ul className="mt-2 overflow-hidden rounded-xl border border-hair bg-panel">
                {project.records.slice(0, 8).map((one) => (
                  <li
                    key={one.path}
                    className="group flex items-center gap-3 border-t border-hair px-4 py-2.5 first:border-t-0"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[13.5px] text-text">{one.title || one.name}</div>
                      <div className="truncate text-[11.5px] text-faint">
                        {one.name} · {ago(one.changed)} · {size(one.size)}
                      </div>
                    </div>
                    <button
                      type="button"
                      onClick={() => open({ kind: "record", value: one.path })}
                      className="rounded-lg px-2.5 py-1 text-[12.5px] text-muted hover:bg-text/5 hover:text-text"
                    >
                      Read
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        open({ kind: "project", value: { directory: project.directory, record: one.path } })
                      }
                      className="rounded-lg px-2.5 py-1 text-[12.5px] text-accent hover:bg-accent/10"
                    >
                      Go on
                    </button>
                  </li>
                ))}
                {project.records.length === 0 ? (
                  <li className="px-4 py-3 text-[13px] text-faint">No record yet</li>
                ) : null}
              </ul>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
