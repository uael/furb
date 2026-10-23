import type { LiveAct } from "@furb/engine/life";
import * as Menu from "@radix-ui/react-dropdown-menu";
import { MoreHorizontal } from "lucide-react";
import { type ReactNode, useCallback, useState } from "react";
import { call } from "../data.ts";
import { useSaid } from "../life.ts";
import type { Restore, Said } from "../wire.ts";

/** A call of a control, whose failure the session shows in its bar, as the TUI shows it. */
const control = (name: string, ...args: unknown[]) => call(name, ...args).catch(() => undefined);

/** A rewind that waits for the operator to say what comes back, since a write changed a file after its point. */
export interface Rewinding {
  act: string;
  restore: Restore;
}

/** The menu of one act that the operator picked: a pause, a wake or a cancel of that act alone, with what stands
 * under it, and a rewind to it. A rewind gives the chain that the life shows after it. */
export function ActMenu({
  act,
  live,
  onRewind,
}: {
  act: LiveAct;
  live: boolean;
  onRewind: (rewinding: Rewinding | string) => void;
}) {
  if (!live) return null;
  const runs = !act.done;
  const rewind = async () => {
    const restore = await call<Restore>("files", act.id).catch(() => undefined);
    if (!restore) return;
    if (restore.files.length) return onRewind({ act: act.id, restore });
    const chain = await call<string>("rewind", act.id, "conversation").catch(() => undefined);
    if (chain) onRewind(chain);
  };
  const item =
    "flex cursor-default items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none data-[highlighted]:bg-text/5";
  return (
    <Menu.Root>
      <Menu.Trigger asChild>
        <button
          type="button"
          aria-label={`Controls of ${act.id}`}
          onClick={(event) => event.stopPropagation()}
          className="grid h-6 w-6 shrink-0 place-items-center rounded-md text-faint opacity-60 outline-none hover:bg-text/5 hover:text-text hover:opacity-100 data-[state=open]:opacity-100"
        >
          <MoreHorizontal size={15} />
        </button>
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Content
          align="end"
          sideOffset={4}
          className="z-50 min-w-[200px] rounded-xl border border-hair bg-raised p-1 text-[13px] shadow-[var(--shadow)]"
        >
          <Menu.Label className="px-2.5 pt-1 pb-1.5 font-mono text-[11.5px] text-faint">{act.id}</Menu.Label>
          {runs && !act.paused ? (
            <Menu.Item className={item} onSelect={() => void control("pause", act.id)}>
              Pause {act.id}
            </Menu.Item>
          ) : null}
          {runs && act.paused ? (
            <Menu.Item className={item} onSelect={() => void control("wake", act.id)}>
              Wake {act.id}
            </Menu.Item>
          ) : null}
          {runs ? (
            <Menu.Item className={`${item} text-failed`} onSelect={() => void control("cancel", act.id)}>
              Cancel {act.id}
            </Menu.Item>
          ) : null}
          {act.kind === "rung" || act.kind === "prompt" ? (
            <Menu.Item className={item} onSelect={() => void rewind()}>
              Rewind to {act.id}
            </Menu.Item>
          ) : null}
        </Menu.Content>
      </Menu.Portal>
    </Menu.Root>
  );
}

/** A dialog over the window, with its choices. */
function Dialog({ title, children, choices }: { title: string; children: ReactNode; choices: ReactNode }) {
  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/40 p-8">
      <div
        role="dialog"
        aria-label={title}
        className="w-full max-w-[480px] rounded-2xl border border-hair bg-raised p-5 shadow-[var(--shadow)]"
      >
        <h2 className="text-[15px] font-semibold text-text">{title}</h2>
        <div className="mt-2 text-[13.5px] leading-relaxed text-muted">{children}</div>
        <div className="mt-5 flex flex-wrap justify-end gap-2">{choices}</div>
      </div>
    </div>
  );
}

const choice =
  "rounded-lg border border-hair px-3 py-1.5 text-[13px] text-text hover:border-hair-strong hover:bg-text/5";

/** A cancel that ends running work, which the operator confirms first, as in the TUI. */
export function ConfirmCancel() {
  const [asked, setAsked] = useState<Extract<Said, { confirm: unknown }>["confirm"] | undefined>();
  useSaid(
    useCallback((said: Said) => {
      if ("confirm" in said) setAsked(said.confirm);
    }, []),
  );
  if (!asked) return null;
  const answer = (yes: boolean) => {
    setAsked(undefined);
    void control("confirm", yes);
  };
  const count = asked.acts.length;
  return (
    <Dialog
      title={`Cancel ${asked.act}?`}
      choices={
        <>
          <button type="button" className={choice} onClick={() => answer(false)}>
            Keep it running
          </button>
          <button
            type="button"
            className={`${choice} border-failed/40 text-failed`}
            onClick={() => answer(true)}
          >
            Cancel {count === 1 ? "it and 1 act" : `it and ${count} acts`}
          </button>
        </>
      }
    >
      The cancel ends {count === 1 ? "1 act" : `${count} acts`} that run under it, on {asked.chains}.
    </Dialog>
  );
}

/** What a rewind gives back, as the tree of the TUI asks it when a write changed a file after its point: the
 * conversation, the files, or both. A change that a command made stays. */
export function RewindChoice({
  rewinding,
  onDone,
}: {
  rewinding: Rewinding;
  onDone: (chain: string | undefined) => void;
}) {
  const { act, restore } = rewinding;
  const go = async (mode: string) => {
    onDone(await call<string>("rewind", act, mode).catch(() => undefined));
  };
  const files = restore.files.length;
  return (
    <Dialog
      title={`Rewind to ${act}`}
      choices={
        <>
          <button type="button" className={choice} onClick={() => onDone(undefined)}>
            Keep all as it is
          </button>
          <button type="button" className={choice} onClick={() => void go("conversation")}>
            Restore the conversation
          </button>
          <button type="button" className={choice} onClick={() => void go("files")}>
            Restore the files
          </button>
          <button
            type="button"
            className={`${choice} border-accent/50 text-accent`}
            onClick={() => void go("both")}
          >
            Both
          </button>
        </>
      }
    >
      {files === 1 ? "1 file changed" : `${files} files changed`} after this point:{" "}
      {restore.files.map((one) => one.path.split("/").at(-1)).join(", ")}.
      {restore.commands
        ? ` ${restore.commands} ${restore.commands === 1 ? "command" : "commands"} ran after it, and a change that a command made is not restored.`
        : ""}{" "}
      The conversation can go back on a new branch, the files can come back as they were, or both.
    </Dialog>
  );
}
