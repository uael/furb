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
} from "@xyflow/react";
import "@xyflow/react/dist/base.css";
import { Braces, Pause, StepForward } from "lucide-react";
import { useMemo } from "react";
import { actorName, type Chain, type State, type Step, type Thread, type View } from "../data.ts";
import { Code } from "./Code.tsx";
import { Glyph } from "./Glyph.tsx";

const ink: Record<State, string> = {
  asks: "var(--asks)",
  failed: "var(--failed)",
  paused: "var(--paused)",
  runs: "var(--accent)",
  waits: "var(--accent)",
  done: "var(--hair-strong)",
  rest: "var(--hair-strong)",
};

/** The width of a program on the graph, and the room between two columns and two rows. */
const WIDTH = 380;
const COLUMN = 470;
const ROW = 44;

/** The thread of a chain that matters now: the one that waits for the operator, else one that runs, else the last. */
function current(view: View, chain: Chain): Thread | undefined {
  const threads = chain.threads
    .map((id) => view.threads.find((one) => one.id === id))
    .filter((one) => one !== undefined);
  return (
    threads.find((one) => one.state === "asks") ??
    threads.find((one) => one.state === "runs") ??
    threads.at(-1)
  );
}

/** The lines of the step that runs, around the line that runs: the window a program shows on the graph. */
function excerpt(thread: Thread | undefined): Step | undefined {
  const step = thread?.ladder.findLast((one) => one.lines.some((line) => line.here)) ?? thread?.ladder.at(-1);
  if (!step) return undefined;
  const here = Math.max(
    step.lines.findIndex((line) => line.here),
    0,
  );
  const from = Math.max(0, Math.min(here - 2, step.lines.length - 5));
  return { ...step, lines: step.lines.slice(from, from + 5), findings: null, raised: null };
}

type ProgramData = { view: View; chain: Chain; picked: boolean };

/** One program of the life: a real component that holds its state, its threads, the code of the step that runs, and
 * the controls of a debugger, which show when the program is picked or under the pointer. */
function Program({ data }: NodeProps<Node<ProgramData>>) {
  const { view, chain, picked } = data;
  const thread = current(view, chain);
  const step = excerpt(thread);
  const threads = chain.threads
    .map((id) => view.threads.find((one) => one.id === id))
    .filter((one) => one !== undefined);
  return (
    <div
      className={`group w-[380px] rounded-2xl border bg-panel shadow-[var(--shadow)] transition-[border-color] duration-150 ${
        picked ? "border-accent/70" : chain.state === "asks" ? "border-asks/40" : "border-hair"
      }`}
    >
      <Handle type="target" position={Position.Left} id="in" />
      <Handle type="target" position={Position.Bottom} id="under" />
      <Handle type="source" position={Position.Right} id="out" />
      <Handle type="source" position={Position.Left} id="back" />
      <div className="flex items-center gap-2.5 px-4 pt-3.5">
        <Glyph state={chain.state} size={13} title />
        <span className="text-[15px] font-semibold text-text">{chain.label || chain.id}</span>
        <span className="text-[12px] text-faint">{actorName(chain.actor)}</span>
        <span className="tabular ml-auto font-mono text-[12px] text-muted">${chain.cost.toFixed(2)}</span>
      </div>
      <div
        className="mx-4 mt-2.5 h-[2px] overflow-hidden rounded-full bg-hair"
        title={`${Math.round(chain.share * 100)}% of the window read`}
      >
        <div className="h-full bg-accent/70" style={{ width: `${Math.min(chain.share * 400, 100)}%` }} />
      </div>
      <div className="px-2 pt-2">
        {threads.map((one) => (
          <div key={one.id} className="flex items-center gap-2 rounded-lg px-2 py-1 text-[12.5px]">
            <Glyph state={one.state} size={10} />
            <span className={`truncate ${one.id === thread?.id ? "text-text" : "text-muted"}`}>
              {one.title}
            </span>
          </div>
        ))}
      </div>
      {step ? (
        <div className="nodrag nowheel mx-3 mt-1.5 mb-3 overflow-hidden rounded-xl bg-well py-1 pr-2 text-[11.5px]">
          <Code steps={[step]} heads={false} />
        </div>
      ) : (
        <div className="px-4 pt-1 pb-4 text-[12.5px] text-faint">No step yet</div>
      )}
      <div
        className={`absolute -top-3.5 right-4 flex items-center gap-0.5 rounded-lg border border-hair bg-raised p-0.5 shadow-[var(--shadow)] transition-opacity duration-150 ${
          picked ? "opacity-100" : "opacity-0 group-hover:opacity-100"
        }`}
      >
        {[
          { Icon: Pause, label: "Pause  Space" },
          { Icon: StepForward, label: "Step  S" },
          { Icon: Braces, label: "Open as code  3" },
        ].map(({ Icon, label }) => (
          <button
            key={label}
            type="button"
            title={label}
            className="grid h-6 w-6 place-items-center rounded-md text-muted hover:bg-text/5 hover:text-text"
          >
            <Icon size={13} strokeWidth={1.9} />
          </button>
        ))}
      </div>
    </div>
  );
}

type YouData = { view: View };

/** The operator, as a node: an actor like the models, with the questions that wait for an answer. */
function You({ data }: NodeProps<Node<YouData>>) {
  const owed = data.view.life.owed.length;
  return (
    <div
      className={`flex items-center gap-2.5 rounded-full border bg-panel py-2 pr-4 pl-3 shadow-[var(--shadow)] ${owed ? "border-asks/50" : "border-hair"}`}
    >
      <Handle type="source" position={Position.Top} id="out" />
      <Handle type="target" position={Position.Right} id="in" />
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

/** The height a program takes on the graph, as its parts add up. */
function height(view: View, chain: Chain): number {
  const lines = excerpt(current(view, chain))?.lines.length ?? 0;
  return 70 + chain.threads.length * 28 + (lines ? lines * 22 + 22 : 28);
}

/** The places of the programs: each chain in the column of its depth, at the height of the chain that made it or under
 * the last program of its column, so a new chain takes the next place and no program moves for it. */
export function layout(view: View): Map<string, { x: number; y: number }> {
  const places = new Map<string, { x: number; y: number }>();
  const bottoms = new Map<number, number>();
  for (const chain of view.chains) {
    const parent = places.get(chain.parent)?.y ?? 0;
    const below = bottoms.has(chain.depth) ? (bottoms.get(chain.depth) ?? 0) + ROW : -Infinity;
    const y = Math.max(parent, below);
    places.set(chain.id, { x: chain.depth * COLUMN, y });
    bottoms.set(chain.depth, y + height(view, chain));
  }
  return places;
}

/** The life as a graph of running programs and the messages between them. */
export function GraphView({
  view,
  picked,
  onPick,
}: {
  view: View;
  picked: string;
  onPick: (chain: string) => void;
}) {
  const { nodes, edges } = useMemo(() => {
    const places = layout(view);
    const nodes: Node[] = view.chains.map((chain) => ({
      id: chain.id,
      type: "program",
      position: places.get(chain.id) ?? { x: 0, y: 0 },
      data: { view, chain, picked: chain.id === picked },
      width: WIDTH,
    }));
    const under = Math.max(
      ...view.chains
        .filter((one) => one.depth === 0)
        .map((one) => (places.get(one.id)?.y ?? 0) + height(view, one)),
      0,
    );
    nodes.push({ id: "operator", type: "you", position: { x: 60, y: under + 110 }, data: { view } });
    const edges: FlowEdge[] = view.edges.map((edge) => {
      const open = edge.state !== "done" && edge.state !== "failed" && edge.state !== "rest";
      const color = ink[edge.state];
      const thread = view.threads.find((one) => one.id === edge.id);
      const toYou = edge.to === "operator";
      const fromYou = edge.from === "operator";
      return {
        id: edge.id,
        source: edge.from,
        target: edge.to,
        sourceHandle: toYou ? "back" : "out",
        targetHandle: fromYou ? "under" : "in",
        animated: open,
        label: toYou && open && thread ? "asks you" : undefined,
        labelStyle: { fill: "var(--asks)", fontSize: 11.5, fontFamily: "var(--font-sans)" },
        labelBgStyle: { fill: "var(--bg)" },
        labelBgPadding: [6, 3] as [number, number],
        labelBgBorderRadius: 6,
        style: { stroke: color, strokeWidth: open ? 1.6 : 1.2, strokeDasharray: open ? "5 7" : undefined },
        markerEnd: { type: MarkerType.ArrowClosed, color, width: 14, height: 14 },
      };
    });
    return { nodes, edges };
  }, [view, picked]);
  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{
          padding: { top: "56px", left: "64px", right: "64px", bottom: "200px" },
          maxZoom: 1,
        }}
        minZoom={0.2}
        maxZoom={2.5}
        proOptions={{ hideAttribution: true }}
        nodesConnectable={false}
        onNodeClick={(_, node) => onPick(node.id)}
      >
        <Background variant={BackgroundVariant.Dots} gap={22} size={1.2} color="var(--hair-strong)" />
        <Controls showInteractive={false} position="bottom-left" />
      </ReactFlow>
    </div>
  );
}
