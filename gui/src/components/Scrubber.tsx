import { ArrowRightToLine } from "lucide-react";
import { type PointerEvent, useRef } from "react";
import { chainName, type Moment } from "../life.ts";
import { clip } from "../steps.ts";

const ink = { asks: "var(--asks)", done: "var(--done)", failed: "var(--failed)" } as const;

/** What a mark of time says. */
export function markWords(moment: Moment, kind: string, id: string): string {
  const act = moment.rows.get(id);
  const title = clip(String(act?.words[1] ?? id), 60);
  if (kind === "step") return `${id} starts a step on ${chainName(moment, act?.on ?? "")}`;
  if (kind === "asks") return `${chainName(moment, act?.on ?? "")} asks you: ${title}`;
  return `${kind === "failed" ? "Failed" : "Done"}: ${title}`;
}

/** The axis of time of the life, under the header and the whole width of the view: a mark for each step, each
 * question to the operator and each end of a thread, and the cursor, which a drag moves. The pill says the moment and
 * brings the view back to now. */
export function Scrubber({
  moment,
  len,
  onTravel,
}: {
  moment: Moment;
  len: number;
  onTravel: (at: number | null) => void;
}) {
  const track = useRef<HTMLDivElement>(null);
  const span = Math.max(len, 1);
  const place = (at: number) => `${(at / span) * 100}%`;
  const travel = (event: PointerEvent) => {
    const box = track.current?.getBoundingClientRect();
    if (!box) return;
    const at = Math.round(((event.clientX - box.left) / box.width) * span);
    onTravel(at >= len ? null : Math.max(1, at));
  };
  const steps = moment.marks.filter((one) => one.kind === "step");
  const others = moment.marks.filter((one) => one.kind !== "step");
  const here = moment.marks.findLast((one) => one.at <= moment.at);
  const words =
    here && here.at === moment.at ? markWords(moment, here.kind, here.act) : `Moment ${moment.at} of ${len}`;
  return (
    <div className="flex h-[38px] shrink-0 items-center gap-4 border-b border-hair px-5">
      <div
        ref={track}
        role="slider"
        aria-label="Time"
        aria-valuemin={1}
        aria-valuemax={len}
        aria-valuenow={moment.at}
        tabIndex={0}
        className="relative h-full flex-1 cursor-pointer touch-none"
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          travel(event);
        }}
        onPointerMove={(event) => {
          if (event.currentTarget.hasPointerCapture(event.pointerId)) travel(event);
        }}
      >
        <div className="absolute top-1/2 right-0 left-0 h-px bg-hair-strong" />
        <div className="absolute top-1/2 left-0 h-px bg-muted/60" style={{ width: place(moment.at) }} />
        <svg className="absolute inset-0 h-full w-full overflow-visible" aria-hidden="true">
          {steps.map((one) => (
            <line
              key={`${one.act}-${one.at}`}
              x1={place(one.at)}
              x2={place(one.at)}
              y1="44%"
              y2="56%"
              stroke="var(--faint)"
              strokeWidth={1}
            />
          ))}
          {others.map((one) =>
            one.kind === "asks" ? (
              <rect
                key={`${one.act}-${one.at}`}
                x={place(one.at)}
                y="50%"
                width={7}
                height={7}
                fill={ink.asks}
                style={{
                  transformBox: "fill-box",
                  transformOrigin: "center",
                  transform: "translate(-3.5px, -3.5px) rotate(45deg)",
                }}
              />
            ) : (
              <line
                key={`${one.act}-${one.at}`}
                x1={place(one.at)}
                x2={place(one.at)}
                y1="30%"
                y2="70%"
                stroke={ink[one.kind as "done" | "failed"]}
                strokeWidth={2}
                strokeLinecap="round"
              />
            ),
          )}
        </svg>
        <div
          className={`absolute top-1/2 h-3.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-bg shadow-[var(--shadow)] ${moment.now ? "bg-done" : "bg-accent"}`}
          style={{ left: place(moment.at) }}
        />
      </div>
      {moment.now ? (
        <span className="flex shrink-0 items-center gap-1.5 px-2 text-[12px] text-done">
          <span className="h-1.5 w-1.5 rounded-full bg-done" />
          Now
        </span>
      ) : (
        <button
          type="button"
          onClick={() => onTravel(null)}
          title="Back to now  Esc"
          className="flex max-w-[420px] shrink-0 items-center gap-2 rounded-full bg-accent/15 py-1 pr-2.5 pl-3 text-[12px] text-accent hover:bg-accent/25"
        >
          <span className="truncate">{words}</span>
          <ArrowRightToLine size={13} strokeWidth={2} />
        </button>
      )}
    </div>
  );
}
