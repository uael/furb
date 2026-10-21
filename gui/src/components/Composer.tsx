import * as Menu from "@radix-ui/react-dropdown-menu";
import { ArrowUp, ChevronDown, Code2 } from "lucide-react";
import { forwardRef, useState } from "react";
import { send } from "../data.ts";
import { chainName, type Held, type Moment } from "../life.ts";
import { Glyph } from "./Glyph.tsx";

/** The shapes that a prompt of the operator can take. */
const shapes = ["str", "None", "int", "float", "bool", "list[str]"];

/** A menu of choices, as a quiet button that names the choice. */
function Choose({
  value,
  label,
  options,
  onPick,
  mono,
}: {
  value: string;
  label: string;
  options: { id: string; name: string }[];
  onPick: (id: string) => void;
  mono?: boolean;
}) {
  return (
    <Menu.Root>
      <Menu.Trigger asChild>
        <button
          type="button"
          className={`flex items-center gap-1.5 rounded-lg px-2 py-1 text-muted outline-none hover:bg-text/5 hover:text-text ${mono ? "font-mono text-[12px]" : "text-[12.5px]"}`}
        >
          {label}
          <ChevronDown size={13} />
        </button>
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Content
          side="top"
          align="start"
          sideOffset={6}
          className="z-50 max-h-[320px] min-w-[180px] overflow-y-auto rounded-xl border border-hair bg-raised p-1 text-[13px] shadow-[var(--shadow)]"
        >
          {options.map((one) => (
            <Menu.Item
              key={one.id}
              onSelect={() => onPick(one.id)}
              className={`flex cursor-default items-center rounded-lg px-2.5 py-1.5 outline-none data-[highlighted]:bg-text/5 ${one.id === value ? "text-accent" : "text-text"} ${mono ? "font-mono text-[12px]" : ""}`}
            >
              {one.name}
            </Menu.Item>
          ))}
        </Menu.Content>
      </Menu.Portal>
    </Menu.Root>
  );
}

/** The composer: it answers the question that waits in the thread, or asks a chain in prose with a shape, or runs a
 * word of python on it. It shows only while the life works and the view stands at now. */
export const Composer = forwardRef<
  HTMLTextAreaElement,
  {
    held: Held;
    moment: Moment;
    thread: string;
    chain: string;
    onChain: (chain: string) => void;
    inset?: number;
  }
>(function Composer({ held, moment, thread, chain, onChain, inset = 0 }, ref) {
  const [text, setText] = useState("");
  const [python, setPython] = useState(false);
  const [shape, setShape] = useState("str");
  if (!held.works || !moment.now) return null;
  const owed = moment.owed.has(thread) ? moment.threads.find((one) => one.act.id === thread) : undefined;
  const ready = text.trim().length > 0;
  const submit = () => {
    if (!ready) return;
    if (owed) send({ kind: "answer", value: { id: owed.act.id, shape: owed.shape, line: text } });
    else if (python) send({ kind: "word", value: { on: chain, word: text } });
    else send({ kind: "prompt", value: { on: chain, shape, message: text } });
    setText("");
  };
  const chains = moment.chains.map((one) => ({ id: one.id, name: chainName(moment, one.id) }));
  return (
    <div
      className="pointer-events-none absolute bottom-0 left-0 flex justify-center px-8 pt-10 pb-5"
      style={{ right: inset }}
    >
      <div
        className={`pointer-events-auto w-full max-w-[720px] rounded-2xl border bg-panel shadow-[var(--shadow)] transition-colors duration-150 ${
          owed ? "border-asks/45" : "border-hair focus-within:border-hair-strong"
        }`}
      >
        {owed ? (
          <div className="flex items-start gap-2 px-4 pt-3 text-[13px]">
            <span className="pt-[4px]">
              <Glyph state="asks" size={11} />
            </span>
            <span className="text-muted">
              <span className="text-asks">{chainName(moment, owed.chain)} asks you</span> {owed.title}
            </span>
          </div>
        ) : null}
        <textarea
          ref={ref}
          rows={2}
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              submit();
            }
          }}
          placeholder={
            owed
              ? `Your answer, as ${owed.shape}`
              : python
                ? "A word of python"
                : `Ask ${chainName(moment, chain)}`
          }
          className={`block w-full resize-none bg-transparent px-4 pt-3 pb-1 text-[14px] text-text outline-none placeholder:text-faint ${
            python && !owed ? "font-mono text-[13px]" : ""
          }`}
        />
        <div className="flex items-center gap-1 px-2.5 pb-2.5">
          {owed ? null : (
            <>
              <Choose value={chain} label={chainName(moment, chain)} options={chains} onPick={onChain} />
              {python ? null : (
                <Choose
                  value={shape}
                  label={shape}
                  options={shapes.map((one) => ({ id: one, name: one }))}
                  onPick={setShape}
                  mono
                />
              )}
              <button
                type="button"
                onClick={() => setPython(!python)}
                title="A word of python, which runs as a rung on the chain"
                className={`flex items-center gap-1.5 rounded-lg px-2 py-1 text-[12.5px] hover:bg-text/5 ${python ? "text-accent" : "text-muted hover:text-text"}`}
              >
                <Code2 size={14} strokeWidth={1.8} />
                Python
              </button>
            </>
          )}
          <span className="ml-auto pr-2 text-[11.5px] text-faint">
            {owed ? "Enter answers" : "Enter sends, Shift Enter breaks the line"}
          </span>
          <button
            type="button"
            disabled={!ready}
            onClick={submit}
            className={`grid h-8 w-8 place-items-center rounded-full transition-colors duration-150 ${
              ready
                ? owed
                  ? "bg-asks text-accent-ink"
                  : "bg-accent text-accent-ink"
                : "bg-text/[0.07] text-faint"
            }`}
            aria-label={owed ? "Answer" : "Send"}
          >
            <ArrowUp size={16} strokeWidth={2.2} />
          </button>
        </div>
      </div>
    </div>
  );
});
