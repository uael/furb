import { type LiveAct, messages, places } from "@furb/engine/life";
import {
  Background,
  BackgroundVariant,
  Controls,
  type Edge as FlowEdge,
  Handle,
  MarkerType,
  type Node,
  type NodeProps,
  Position,
  ReactFlow,
  type ReactFlowInstance,
  useStore,
} from "@xyflow/react";
import "@xyflow/react/dist/base.css";
import { Braces, Pause, Play, StepForward } from "lucide-react";
import { useMemo, useRef } from "react";
import { actorName, dollars, type State, send } from "../data.ts";
import { chainName, chainState, type Held, hirer, type Moment, threadState } from "../life.ts";
import { type Step, steps } from "../steps.ts";
import { Code } from "./Code.tsx";
import { Glyph } from "./Glyph.tsx";

/** The size of a program on the graph, and the room between two columns and two rows of programs. */
const WIDTH = 380;
const HEIGHT = 300;
const COLUMN = 500;
const ROW = 330;
/** The zoom under which a program shows its compact form: its title, its state and its cost. */
const COMPACT = 0.62;

/** The color of a state on an edge. */
const ink: Record<State, string> = {
  asks: "var(--asks)",
  failed: "var(--failed)",
  paused: "var(--paused)",
  runs: "var(--accent)",
  waits: "var(--accent)",
  done: "var(--hair-strong)",
  rest: "var(--hair-strong)",
};

/** The style of each kind of edge, the same everywhere: a hire is a dotted line, a message a solid line with an arrow,
 * and a question to the operator a dashed line. */
const dash = { hire: "1.5 5", message: undefined, question: "7 5" } as const;

type ProgramData = {
  moment: Moment;
  held: Held;
  chain: LiveAct;
  picked: boolean;
  onCode: (chain: string) => void;
};

/** The thread of a chain that matters now: the one that waits for the operator, else one that runs, else the last. */
function current(moment: Moment, chain: string) {
  const threads = moment.threads.filter(
    (one) => one.chain === chain && one.to !== "operator" && moment.rows.get(one.act.by)?.kind !== "chain",
  );
  return (
    threads.find((one) => moment.owed.has(one.act.id)) ??
    threads.find((one) => !one.act.done) ??
    threads.at(-1)
  );
}

/** The lines of the step that runs, around the line that runs: the window that a program shows on the graph. */
function excerpt(step: Step | undefined): Step | undefined {
  if (!step) return undefined;
  const here = Math.max(
    step.lines.findIndex((line) => line.here),
    0,
  );
  const from = Math.max(0, Math.min(here - 2, step.lines.length - 5));
  return { ...step, lines: step.lines.slice(from, from + 5), why: null };
}

/** One program of the life. Zoomed out it shows its title, its state and its cost in large type; zoomed in it holds
 * its threads, the code of the step that runs, and the controls of a debugger. */
function Program({ data }: NodeProps<Node<ProgramData>>) {
  const { moment, held, chain, picked, onCode } = data;
  const compact = useStore((store) => store.transform[2] < COMPACT);
  const state = chainState(moment, chain.id);
  const measure = held.measures[chain.id];
  const thread = current(moment, chain.id);
  const threads = moment.threads.filter(
    (one) => one.chain === chain.id && moment.rows.get(one.act.by)?.kind !== "chain",
  );
  const ladder = thread ? steps(moment, thread.act, held.directory, held.streams) : [];
  const step = excerpt(ladder.findLast((one) => one.lines.some((line) => line.here)) ?? ladder.at(-1));
  const live = held.works && moment.now;
  const paused = state === "paused";
  const border = picked ? "border-accent/70" : state === "asks" ? "border-asks/50" : "border-hair";
  const handles = (
    <>
      <Handle type="target" position={Position.Left} id="hired" style={{ top: 28 }} />
      <Handle type="source" position={Position.Right} id="hires" style={{ top: 28 }} />
      <Handle type="target" position={Position.Left} id="in" />
      <Handle type="source" position={Position.Right} id="out" />
      <Handle type="source" position={Position.Top} id="asks" />
    </>
  );
  if (compact)
    return (
      <div
        className={`flex flex-col justify-center gap-3 rounded-3xl border-2 bg-panel px-7 shadow-[var(--shadow)] ${border}`}
        style={{ width: WIDTH, height: 150 }}
      >
        {handles}
        <div className="flex items-center gap-3.5">
          <Glyph state={state} size={26} />
          <span className="truncate text-[30px] leading-tight font-semibold text-text">
            {chainName(moment, chain.id)}
          </span>
        </div>
        <div className="flex items-center gap-3 text-[22px] text-muted">
          <span className="tabular font-mono">{dollars(measure?.dollars ?? 0)}</span>
          <span className="text-faint">·</span>
          <span>{threads.length === 1 ? "1 thread" : `${threads.length} threads`}</span>
        </div>
      </div>
    );
  return (
    <div
      className={`group rounded-2xl border bg-panel shadow-[var(--shadow)] ${border}`}
      style={{ width: WIDTH }}
    >
      {handles}
      <div className="flex items-center gap-2.5 px-4 pt-3.5">
        <Glyph state={state} size={13} />
        <span className="truncate text-[15px] font-semibold text-text">{chainName(moment, chain.id)}</span>
        <span className="shrink-0 text-[12px] text-faint">
          {measure?.actor ? actorName(measure.actor) : ""}
        </span>
        <span className="tabular ml-auto font-mono text-[12px] text-muted">
          {dollars(measure?.dollars ?? 0)}
        </span>
      </div>
      <div
        className="mx-4 mt-2.5 h-[2px] overflow-hidden rounded-full bg-hair"
        title={`${Math.round((measure?.share ?? 0) * 100)}% of the window read`}
      >
        <div
          className="h-full bg-accent/70"
          style={{ width: `${Math.min((measure?.share ?? 0) * 400, 100)}%` }}
        />
      </div>
      <div className="px-2 pt-2">
        {threads.slice(-3).map((one) => (
          <div key={one.act.id} className="flex items-center gap-2 rounded-lg px-2 py-1 text-[12.5px]">
            <Glyph state={threadState(moment, one)} size={10} />
            <span className={`truncate ${one.act.id === thread?.act.id ? "text-text" : "text-muted"}`}>
              {one.title}
            </span>
          </div>
        ))}
        {threads.length > 3 ? (
          <div className="px-2 py-0.5 text-[11.5px] text-faint">and {threads.length - 3} more</div>
        ) : null}
      </div>
      {step?.lines.length ? (
        <div className="nodrag nowheel mx-3 mt-1.5 mb-3 overflow-hidden rounded-xl bg-well py-1 pr-2 text-[11.5px]">
          <Code steps={[step]} heads={false} />
        </div>
      ) : (
        <div className="px-4 pt-1 pb-4 text-[12.5px] text-faint">
          {step?.thinks ? "The model writes its step" : "No step yet"}
        </div>
      )}
      <div
        className={`absolute -top-3.5 right-4 flex items-center gap-0.5 rounded-lg border border-hair bg-raised p-0.5 shadow-[var(--shadow)] transition-opacity duration-150 ${
          picked ? "opacity-100" : "opacity-0 group-hover:opacity-100"
        }`}
      >
        {live ? (
          <>
            <button
              type="button"
              title={paused ? "Continue" : "Pause"}
              onClick={() => send({ kind: paused ? "wake" : "pause", value: chain.id })}
              className="grid h-6 w-6 place-items-center rounded-md text-muted hover:bg-text/5 hover:text-text"
            >
              {paused ? <Play size={13} strokeWidth={1.9} /> : <Pause size={13} strokeWidth={1.9} />}
            </button>
            <button
              type="button"
              title="Step"
              onClick={() => send({ kind: "step", value: chain.id })}
              className="grid h-6 w-6 place-items-center rounded-md text-muted hover:bg-text/5 hover:text-text"
            >
              <StepForward size={13} strokeWidth={1.9} />
            </button>
          </>
        ) : null}
        <button
          type="button"
          title="Open as code  3"
          onClick={() => onCode(chain.id)}
          className="grid h-6 w-6 place-items-center rounded-md text-muted hover:bg-text/5 hover:text-text"
        >
          <Braces size={13} strokeWidth={1.9} />
        </button>
      </div>
    </div>
  );
}

type YouData = { moment: Moment };

/** The operator, as a node: an actor like the models, with the questions that wait for an answer. */
function You({ data }: NodeProps<Node<YouData>>) {
  const owed = data.moment.owed.size;
  const compact = useStore((store) => store.transform[2] < COMPACT);
  return (
    <div
      className={`flex items-center gap-2.5 rounded-full border bg-panel py-2 pr-4 pl-3 shadow-[var(--shadow)] ${owed ? "border-asks/50" : "border-hair"} ${compact ? "origin-left scale-[2]" : ""}`}
    >
      <Handle type="source" position={Position.Right} id="out" />
      <Handle type="target" position={Position.Bottom} id="in" />
      <span className="grid h-6 w-6 place-items-center rounded-full bg-text/[0.07] text-[11px] font-semibold text-muted">
        you
      </span>
      <span className="text-[13px] text-text">You</span>
      {owed ? (
        <span className="flex items-center gap-1.5 text-[12.5px] text-asks">
          <Glyph state="asks" size={10} />
          {owed === 1 ? "1 question" : `${owed} questions`}
        </span>
      ) : null}
    </div>
  );
}

const nodeTypes = { program: Program, you: You };

/** The legend of the edges: one meaning for each style. */
function Legend() {
  const row = (label: string, style: keyof typeof dash, color: string) => (
    <div className="flex items-center gap-2.5">
      <svg width="34" height="8" aria-hidden="true">
        <line
          x1="1"
          y1="4"
          x2="33"
          y2="4"
          stroke={color}
          strokeWidth={style === "hire" ? 2 : 1.6}
          strokeDasharray={dash[style]}
          strokeLinecap="round"
        />
      </svg>
      {label}
    </div>
  );
  return (
    <div className="absolute right-5 bottom-[188px] z-10 space-y-1.5 rounded-xl border border-hair bg-panel/90 px-3.5 py-2.5 text-[11.5px] text-muted shadow-[var(--shadow)] backdrop-blur">
      {row("A step hired a chain", "hire", "var(--muted)")}
      {row("A prompt to a chain", "message", "var(--accent)")}
      {row("A question to you", "question", "var(--asks)")}
    </div>
  );
}

/** The life as a graph of the programs that run and the messages between them. Each chain keeps the place where it
 * was born, which the model of a life gives, so no later chain moves it. */
export function GraphView({
  held,
  moment,
  picked,
  onPick,
  onCode,
  zoom,
}: {
  held: Held;
  moment: Moment;
  picked: string;
  onPick: (chain: string) => void;
  onCode: (chain: string) => void;
  zoom?: "in" | "out";
}) {
  const flow = useRef<ReactFlowInstance | null>(null);
  const { nodes, edges } = useMemo(() => {
    const grid = places(moment.chains, (chain) => hirer(moment, chain));
    const nodes: Node[] = moment.chains.map((chain) => {
      const place = grid.get(chain.id) ?? { column: 0, row: 0 };
      return {
        id: chain.id,
        type: "program",
        position: { x: place.column * COLUMN, y: place.row * ROW },
        data: { moment, held, chain, picked: chain.id === picked, onCode },
        width: WIDTH,
        height: HEIGHT,
      };
    });
    nodes.push({ id: "operator", type: "you", position: { x: 0, y: 0 }, data: { moment } });
    const edges: FlowEdge[] = [];
    for (const chain of moment.chains) {
      const from = hirer(moment, chain);
      if (!from) continue;
      edges.push({
        id: `hire-${chain.id}`,
        source: from,
        target: chain.id,
        sourceHandle: "hires",
        targetHandle: "hired",
        type: "smoothstep",
        style: { stroke: "var(--muted)", strokeWidth: 2, strokeDasharray: dash.hire, opacity: 0.7 },
      });
    }
    const recent = (act: LiveAct) => act.ended !== undefined && moment.at - act.ended < 24;
    for (const message of messages(moment.acts, recent)) {
      if (message.kind === "answer") continue;
      const prompt = moment.rows.get(message.act);
      const thread = moment.threads.find((one) => one.act.id === message.act);
      const state = thread ? threadState(moment, thread) : "runs";
      const question = message.kind === "asks you";
      const color = question ? "var(--asks)" : ink[state];
      edges.push({
        id: `${message.kind}-${message.act}`,
        source: message.from,
        target: message.to,
        sourceHandle: question ? "asks" : "out",
        targetHandle: "in",
        label: question ? "asks you" : undefined,
        labelStyle: { fill: color, fontSize: 11.5, fontFamily: "var(--font-sans)" },
        labelBgStyle: { fill: "var(--bg)" },
        labelBgPadding: [6, 3] as [number, number],
        labelBgBorderRadius: 6,
        style: { stroke: color, strokeWidth: 1.6, strokeDasharray: question ? dash.question : dash.message },
        markerEnd: { type: MarkerType.ArrowClosed, color, width: 14, height: 14 },
        data: { prompt: prompt?.id },
      });
    }
    return { nodes, edges };
  }, [moment, held, picked, onCode]);
  return (
    <div className="relative h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{
          padding: { top: "48px", left: "64px", right: "64px", bottom: "200px" },
          maxZoom: 1,
        }}
        minZoom={0.15}
        maxZoom={2.5}
        proOptions={{ hideAttribution: true }}
        nodesConnectable={false}
        nodesDraggable={false}
        onInit={(instance) => {
          flow.current = instance;
          const padding = { top: "48px", left: "64px", right: "64px", bottom: "200px" } as const;
          if (zoom === "out") void instance.fitView({ maxZoom: 0.55, padding });
          if (zoom === "in")
            void instance.fitView({ nodes: [{ id: picked }], maxZoom: 1, minZoom: 1, padding });
        }}
        onNodeClick={(_, node) => {
          if (node.id !== "operator") onPick(node.id);
        }}
      >
        <Background variant={BackgroundVariant.Dots} gap={22} size={1.2} color="var(--hair-strong)" />
        <Controls showInteractive={false} position="bottom-left" />
      </ReactFlow>
      <Legend />
    </div>
  );
}
