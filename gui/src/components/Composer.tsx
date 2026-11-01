import { aimed, gist } from "furb/life";
import { ArrowUp, Undo2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../data.ts";
import { type Held, useSaid } from "../life.ts";
import type { Said } from "../wire.ts";

/** The composer under a chain: a message to the chain shown, which its model answers, or the answer of the question
 * that waits for you on that chain, as the composer of the TUI sends it, with a button for each value of a choice.
 * Enter sends, and Shift+Enter adds a line. The last message of yours on the chain can be taken back. A record that is
 * open to read takes no message. */
export function Composer({
  held,
  chain,
  label,
  mine,
  onSent,
  onChain,
}: {
  held: Held;
  chain: string;
  label: string;
  /** Whether the chain holds a message of yours, which can be taken back. */
  mine: boolean;
  onSent: (chain: string) => void;
  onChain: (chain: string) => void;
}) {
  const life = held.life;
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [error, setError] = useState("");
  const [sending, setSending] = useState(false);
  const box = useRef<HTMLTextAreaElement>(null);
  // A message that you took back comes back to the composer of its chain.
  useSaid(
    useCallback((said: Said) => {
      if ("compose" in said) setDrafts((was) => ({ ...was, [said.compose.chain]: said.compose.text }));
    }, []),
  );
  const text = drafts[chain] ?? "";
  const ask = life ? aimed(life.asks, chain) : undefined;
  const read = Boolean(life?.reading || life?.recorded);
  // The box grows with its lines, up to a third of the window.
  // biome-ignore lint/correctness/useExhaustiveDependencies: the height follows the text.
  useEffect(() => {
    const one = box.current;
    if (!one) return;
    one.style.height = "auto";
    one.style.height = `${Math.min(one.scrollHeight, window.innerHeight / 3)}px`;
  }, [text]);
  if (!life) return null;
  if (read)
    return (
      <div className="border-t border-hair px-6 py-3 text-center text-[12.5px] text-faint">
        {life.recorded
          ? "This is a recorded life: it shows its log and runs nothing."
          : `This record opens to read: ${life.reading?.made ? `it was made by ${life.reading.made}` : "another engine made it"}, so no word runs again and no message goes.`}
      </div>
    );
  const send = async () => {
    if (!text.trim() || sending) return;
    setSending(true);
    setError("");
    try {
      await call("send", chain, text);
      setDrafts({ ...drafts, [chain]: "" });
      onSent(chain);
    } catch (why) {
      setError(why instanceof Error ? why.message : String(why));
    } finally {
      setSending(false);
      box.current?.focus();
    }
  };
  const answer = async (value: string) => {
    if (!ask) return;
    setError("");
    await call("answer", ask.id, value).catch((why: unknown) =>
      setError(why instanceof Error ? why.message : String(why)),
    );
  };
  const takeBack = async () => {
    setError("");
    try {
      onChain(await call<string>("takeBack", chain));
    } catch (why) {
      setError(why instanceof Error ? why.message : String(why));
    }
  };
  const placeholder = ask ? `Answer: ${gist(ask.message)}` : `Message ${label}`;
  return (
    <div className="shrink-0 px-6 pb-5">
      <div className="mx-auto max-w-[760px]">
        {error ? <div className="mb-2 text-[12.5px] text-failed">{error}</div> : null}
        {ask?.choices.length ? (
          <div className="mb-2 flex flex-wrap items-center gap-1.5">
            <span className="mr-1 text-[12.5px] text-asks">{gist(ask.message)}</span>
            {ask.choices.map((value) => (
              <button
                key={value}
                type="button"
                onClick={() => void answer(value)}
                className="rounded-lg border border-asks/40 bg-asks/[0.07] px-2.5 py-1 text-[12.5px] text-text hover:bg-asks/15"
              >
                {value}
              </button>
            ))}
          </div>
        ) : null}
        <div
          className={`flex items-end gap-2 rounded-2xl border bg-panel px-3.5 py-2.5 shadow-[var(--shadow)] transition-colors duration-150 focus-within:border-hair-strong ${ask ? "border-asks/40" : "border-hair"}`}
        >
          <textarea
            ref={box}
            aria-label={ask ? "Your answer" : `A message to ${label}`}
            value={text}
            rows={1}
            placeholder={placeholder}
            onChange={(event) => setDrafts({ ...drafts, [chain]: event.target.value })}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
                event.preventDefault();
                void send();
              }
            }}
            className="max-h-[33vh] min-h-[24px] flex-1 resize-none bg-transparent text-[14px] leading-[1.55] text-text outline-none placeholder:text-faint"
          />
          {mine && !text ? (
            <button
              type="button"
              aria-label="Take back your last message"
              title="Take back your last message"
              onClick={() => void takeBack()}
              className="grid h-7 w-7 shrink-0 place-items-center rounded-lg text-faint hover:bg-text/5 hover:text-text"
            >
              <Undo2 size={15} />
            </button>
          ) : null}
          <button
            type="button"
            aria-label={ask ? "Send the answer" : "Send the message"}
            disabled={!text.trim() || sending}
            onClick={() => void send()}
            className="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-accent text-accent-ink transition-opacity duration-150 disabled:opacity-30"
          >
            <ArrowUp size={15} strokeWidth={2.2} />
          </button>
        </div>
      </div>
    </div>
  );
}
