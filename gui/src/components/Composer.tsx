import { ArrowUp, ChevronDown, Code2 } from "lucide-react";
import { useState } from "react";
import { chainName, type View } from "../data.ts";
import { Glyph } from "./Glyph.tsx";

/** The composer: it answers the question that waits in the thread, or asks a chain in prose or in python. */
export function Composer({ view, thread, chain }: { view: View; thread: string; chain: string }) {
  const [text, setText] = useState("");
  const [python, setPython] = useState(false);
  const owed = view.life.owed.includes(thread) ? view.threads.find((one) => one.id === thread) : undefined;
  const ready = text.trim().length > 0;
  return (
    <div className="pointer-events-none absolute right-0 bottom-0 left-0 flex justify-center px-8 pt-10 pb-5">
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
              <span className="text-asks">{chainName(view, owed.chain)} asks you</span> {owed.title}
            </span>
          </div>
        ) : null}
        <textarea
          rows={2}
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder={
            owed
              ? `Your answer, as ${owed.shape}`
              : python
                ? "A word of python"
                : `Ask ${chainName(view, chain)}`
          }
          className={`block w-full resize-none bg-transparent px-4 pt-3 pb-1 text-[14px] text-text outline-none placeholder:text-faint ${
            python && !owed ? "font-mono text-[13px]" : ""
          }`}
        />
        <div className="flex items-center gap-1 px-2.5 pb-2.5">
          {owed ? null : (
            <>
              <button
                type="button"
                className="flex items-center gap-1.5 rounded-lg px-2 py-1 text-[12.5px] text-muted hover:bg-text/5 hover:text-text"
              >
                {chainName(view, chain)}
                <ChevronDown size={13} />
              </button>
              <button
                type="button"
                className="flex items-center gap-1.5 rounded-lg px-2 py-1 font-mono text-[12px] text-muted hover:bg-text/5 hover:text-text"
              >
                str
                <ChevronDown size={13} />
              </button>
              <button
                type="button"
                onClick={() => setPython(!python)}
                title="Python  :"
                className={`flex items-center gap-1.5 rounded-lg px-2 py-1 text-[12.5px] hover:bg-text/5 ${python ? "text-accent" : "text-muted hover:text-text"}`}
              >
                <Code2 size={14} strokeWidth={1.8} />
                Python
              </button>
            </>
          )}
          <span className="ml-auto pr-2 text-[11.5px] text-faint">
            {owed ? "Enter answers" : "Enter sends"}
          </span>
          <button
            type="button"
            disabled={!ready}
            className={`grid h-8 w-8 place-items-center rounded-full transition-colors duration-150 ${
              ready ? "bg-accent text-accent-ink" : "bg-text/[0.07] text-faint"
            }`}
            aria-label="Send"
          >
            <ArrowUp size={16} strokeWidth={2.2} />
          </button>
        </div>
      </div>
    </div>
  );
}
