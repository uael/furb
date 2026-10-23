import { open as pickOne } from "@tauri-apps/plugin-dialog";
import { ArrowLeft, FileClock, FolderOpen, FolderPlus, Plus } from "lucide-react";
import { useState } from "react";
import { call, tauri } from "../data.ts";
import type { Held } from "../life.ts";

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

/** The words of the status of a record, as the TUI says them. */
const statuses: Record<string, string> = {
  saved: "Saved",
  idle: "Ready",
  working: "Working",
  blocked: "Input needed",
  paused: "Paused",
  done: "Finished, unread",
  error: "Error",
  opening: "Opening",
};

/** The first screen of the app: the projects that the TUI and the app know, each with its records. A project opens
 * to a new life on the models of the catalog, and a record goes on as a life, as the TUI opens them. */
export function Picker({ held, onBack }: { held: Held; onBack?: () => void }) {
  const [error, setError] = useState("");
  const run = (name: string, ...args: unknown[]) => {
    setError("");
    void call(name, ...args).catch((why: unknown) =>
      setError(why instanceof Error ? why.message : String(why)),
    );
  };
  const record = async () => {
    const path = await pickOne({
      title: "Open a record",
      filters: [{ name: "Record", extensions: ["jsonl"] }],
    });
    if (typeof path === "string") run("open", path);
  };
  const project = async () => {
    const path = await pickOne({ title: "Start a life in a project", directory: true });
    if (typeof path === "string") run("create", path);
  };
  const opening = held.projects.flatMap((one) => one.records).find((one) => one.status === "opening");
  const button =
    "flex items-center gap-2 rounded-xl border border-hair bg-panel px-3.5 py-2 text-[13.5px] shadow-[var(--shadow)] hover:border-hair-strong disabled:opacity-50";
  return (
    <div data-tauri-drag-region className="flex h-full justify-center overflow-y-auto bg-bg">
      <div className="w-full max-w-[760px] px-10 pt-24 pb-16">
        {onBack ? (
          <button
            type="button"
            onClick={onBack}
            className="mb-6 flex items-center gap-1.5 text-[13px] text-muted hover:text-text"
          >
            <ArrowLeft size={14} />
            Back to the life
          </button>
        ) : null}
        <div data-tauri-drag-region className="flex items-center gap-3">
          <span className="grid h-9 w-9 place-items-center rounded-xl bg-accent text-[18px] font-semibold text-accent-ink">
            f
          </span>
          <h1 className="text-[22px] font-semibold tracking-[-0.01em]">furb</h1>
        </div>
        <p className="mt-2 text-[14px] text-muted">
          Start a new life in a project, or go on with a record. A new life works on the models that this
          machine offers.
        </p>
        {!held.connected ? (
          <div className="mt-6 flex items-center gap-2.5 rounded-xl border border-hair bg-panel px-4 py-3 text-[13.5px]">
            <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
            Waiting for the host of the life
          </div>
        ) : opening ? (
          <div className="mt-6 flex items-center gap-2.5 rounded-xl border border-hair bg-panel px-4 py-3 text-[13.5px]">
            <span className="h-2 w-2 animate-pulse rounded-full bg-accent" />
            Opening {opening.name}
          </div>
        ) : null}
        {error ? (
          <div className="mt-6 rounded-xl border border-failed/30 bg-failed/[0.07] px-4 py-3 text-[13.5px] text-failed">
            {error}
          </div>
        ) : null}
        <div className="mt-8 flex gap-2">
          <button type="button" disabled={!tauri} onClick={() => void project()} className={button}>
            <FolderPlus size={15} strokeWidth={1.8} />
            Start a life in a project
          </button>
          <button type="button" disabled={!tauri} onClick={() => void record()} className={button}>
            <FileClock size={15} strokeWidth={1.8} />
            Open a record
          </button>
        </div>
        <div className="mt-10 space-y-8">
          {held.projects.map((project) => (
            <section key={project.directory}>
              <div className="flex items-center gap-3">
                <FolderOpen size={15} className="text-faint" strokeWidth={1.8} />
                <h2 className="text-[15px] font-semibold">{project.name}</h2>
                <span className="min-w-0 flex-1 truncate text-[12px] text-faint">{project.directory}</span>
                <button
                  type="button"
                  onClick={() => run("create", project.directory)}
                  className="flex shrink-0 items-center gap-1 rounded-lg px-2.5 py-1 text-[12.5px] text-accent hover:bg-accent/10"
                >
                  <Plus size={13} />
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
                      <div className="truncate text-[13.5px] text-text">{one.name}</div>
                      <div className={`truncate text-[11.5px] ${one.error ? "text-failed" : "text-faint"}`}>
                        {one.error ??
                          [
                            statuses[one.status] ?? one.status,
                            one.modified ? ago(one.modified) : "",
                            one.size ? size(one.size) : "",
                          ]
                            .filter(Boolean)
                            .join(" · ")}
                      </div>
                    </div>
                    <button
                      type="button"
                      onClick={() => run("open", one.path)}
                      className="rounded-lg px-2.5 py-1 text-[12.5px] text-muted hover:bg-text/5 hover:text-text"
                    >
                      Open
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
