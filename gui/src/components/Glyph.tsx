import { type State, stateWords } from "../data.ts";

const color: Record<State, string> = {
  asks: "var(--asks)",
  failed: "var(--failed)",
  paused: "var(--paused)",
  runs: "var(--accent)",
  waits: "var(--faint)",
  done: "var(--done)",
  rest: "var(--faint)",
};

/** The shape of a state, so a state reads by its shape as well as by its color: a dot that breathes for work that
 * runs, a diamond for a question to the operator, a cross for a failure, a check for an end, two bars for a pause,
 * and a ring for a wait or a rest. */
export function Glyph({ state, size = 12 }: { state: State; size?: number; title?: boolean }) {
  const ink = color[state];
  return (
    <svg
      role="img"
      aria-label={stateWords[state]}
      width={size}
      height={size}
      viewBox="-6 -6 12 12"
      className="shrink-0 overflow-visible"
    >
      <title>{stateWords[state]}</title>
      {state === "runs" ? (
        <>
          <circle r={3.2} fill={ink} className="breathe" />
          <circle r={3.2} fill={ink} />
        </>
      ) : null}
      {state === "asks" ? <path d="M0 -4.6 L4.6 0 L0 4.6 L-4.6 0 Z" fill={ink} /> : null}
      {state === "failed" ? (
        <path
          d="M-3.2 -3.2 L3.2 3.2 M-3.2 3.2 L3.2 -3.2"
          stroke={ink}
          strokeWidth={1.8}
          strokeLinecap="round"
        />
      ) : null}
      {state === "done" ? (
        <path
          d="M-3.6 0.2 L-1.1 2.8 L3.8 -2.6"
          stroke={ink}
          strokeWidth={1.8}
          fill="none"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      ) : null}
      {state === "paused" ? (
        <path d="M-1.8 -3.4 V3.4 M1.8 -3.4 V3.4" stroke={ink} strokeWidth={1.9} strokeLinecap="round" />
      ) : null}
      {state === "waits" ? (
        <circle r={3.4} fill="none" stroke={ink} strokeWidth={1.5} strokeDasharray="2.2 1.6" />
      ) : null}
      {state === "rest" ? <circle r={3} fill="none" stroke={ink} strokeWidth={1.4} /> : null}
    </svg>
  );
}
