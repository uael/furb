import type { Gesture, Node, Slot, Tone } from "furb/protocol";
import { type ReactNode, useEffect, useState } from "react";

/** The color class of each tone of a run, from the tokens of the theme. A tone that the window does not know reads as text. */
export const tones: Record<Tone, string> = {
  text: "text-text",
  muted: "text-muted",
  faint: "text-faint",
  accent: "text-accent",
  success: "text-done",
  warning: "text-asks",
  danger: "text-failed",
};

/** What a view tells its skill: the node by its id, what the person did to it, and the value of an input. */
export type Told = (node: string, kind: Gesture, value?: string) => void;

/** A view: the tree of a skill, drawn in the DOM. A view that reads is inert: it draws each node and sends no event. */
export interface ViewProps {
  tree: Node;
  inert?: boolean;
  onEvent?: Told;
}

/** The block of a markdown that a view draws. */
export type Block =
  | { kind: "heading"; level: number; text: string }
  | { kind: "code"; language: string; text: string }
  | { kind: "list"; items: string[] }
  | { kind: "paragraph"; text: string };

/** The blocks of a markdown: headings, fenced code, lists of bullets and paragraphs. */
export function blocks(source: string): Block[] {
  const made: Block[] = [];
  const lines = source.split("\n");
  let at = 0;
  const flat = (line: string) => line.trim() === "";
  while (at < lines.length) {
    const line = lines[at] ?? "";
    const fence = /^```\s*(\S*)\s*$/.exec(line);
    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    if (flat(line)) {
      at += 1;
    } else if (fence) {
      const body: string[] = [];
      at += 1;
      while (at < lines.length && !/^```\s*$/.test(lines[at] ?? "")) {
        body.push(lines[at] ?? "");
        at += 1;
      }
      at += 1;
      made.push({ kind: "code", language: fence[1] ?? "", text: body.join("\n") });
    } else if (heading) {
      made.push({ kind: "heading", level: (heading[1] ?? "#").length, text: heading[2] ?? "" });
      at += 1;
    } else if (/^\s*[-*]\s+/.test(line)) {
      const items: string[] = [];
      while (at < lines.length && /^\s*[-*]\s+/.test(lines[at] ?? "")) {
        items.push((lines[at] ?? "").replace(/^\s*[-*]\s+/, ""));
        at += 1;
      }
      made.push({ kind: "list", items });
    } else {
      const body: string[] = [];
      while (at < lines.length) {
        const next = lines[at] ?? "";
        if (flat(next) || /^```/.test(next) || /^#{1,6}\s/.test(next) || /^\s*[-*]\s+/.test(next)) break;
        body.push(next.trim());
        at += 1;
      }
      made.push({ kind: "paragraph", text: body.join(" ") });
    }
  }
  return made;
}

/** A part of a line of markdown: plain text, code, strong text, or a link to an address of the web. */
export type Inline =
  | { kind: "text"; text: string }
  | { kind: "code"; text: string }
  | { kind: "strong"; text: string }
  | { kind: "link"; text: string; href: string };

/** The parts of a line of markdown. Only a link to http or https is a link: any other address stays text. */
export function inlines(line: string): Inline[] {
  const parts: Inline[] = [];
  const pattern = /`([^`]+)`|\*\*([^*]+)\*\*|\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g;
  let from = 0;
  for (let found = pattern.exec(line); found; found = pattern.exec(line)) {
    if (found.index > from) parts.push({ kind: "text", text: line.slice(from, found.index) });
    if (found[1] !== undefined) parts.push({ kind: "code", text: found[1] });
    else if (found[2] !== undefined) parts.push({ kind: "strong", text: found[2] });
    else parts.push({ kind: "link", text: found[3] ?? "", href: found[4] ?? "" });
    from = found.index + found[0].length;
  }
  if (from < line.length) parts.push({ kind: "text", text: line.slice(from) });
  return parts;
}

/** How a line of a diff reads. */
export type Change = "add" | "del" | "hunk" | "head" | "same";

/** The kind of a line of a diff, by its first characters. */
export function changed(line: string): Change {
  if (line.startsWith("+++") || line.startsWith("---")) return "head";
  if (line.startsWith("@@")) return "hunk";
  if (line.startsWith("+")) return "add";
  if (line.startsWith("-")) return "del";
  return "same";
}

const changes: Record<Change, string> = {
  add: "bg-done/10 text-done",
  del: "bg-failed/10 text-failed",
  hunk: "text-accent",
  head: "text-muted",
  same: "text-text",
};

/** The key of a part that has no name but its place: a run of a text, a line of a diff, a block of a markdown. */
const placed = (at: number) => `#${at}`;

/** A line of markdown, with its code, its strong text and its links. */
function Line({ text }: { text: string }) {
  return (
    <>
      {inlines(text).map((one, at) => {
        const key = placed(at);
        if (one.kind === "code") {
          return (
            <code key={key} className="rounded bg-well px-1 font-mono text-[0.92em]">
              {one.text}
            </code>
          );
        }
        if (one.kind === "strong") return <strong key={key}>{one.text}</strong>;
        if (one.kind === "link") {
          return (
            <a
              key={key}
              href={one.href}
              target="_blank"
              rel="noreferrer noopener"
              className="text-accent underline"
            >
              {one.text}
            </a>
          );
        }
        return <span key={key}>{one.text}</span>;
      })}
    </>
  );
}

/** A markdown, drawn as DOM elements and never as a string of HTML. */
function Markdown({ text }: { text: string }) {
  return (
    <div className="flex flex-col gap-2">
      {blocks(text).map((one, at) => {
        const key = placed(at);
        if (one.kind === "heading") {
          const Heading = `h${one.level}` as "h1" | "h2" | "h3" | "h4" | "h5" | "h6";
          return (
            <Heading key={key} className="font-semibold">
              <Line text={one.text} />
            </Heading>
          );
        }
        if (one.kind === "code") {
          return (
            <pre
              key={key}
              data-language={one.language || undefined}
              className="overflow-x-auto rounded bg-well p-2 font-mono text-[0.92em]"
            >
              <code>{one.text}</code>
            </pre>
          );
        }
        if (one.kind === "list") {
          return (
            <ul key={key} className="list-disc pl-5">
              {one.items.map((item, place) => (
                <li key={placed(place)}>
                  <Line text={item} />
                </li>
              ))}
            </ul>
          );
        }
        return (
          <p key={key}>
            <Line text={one.text} />
          </p>
        );
      })}
    </div>
  );
}

/** A button: a press goes to the skill, unless the view is inert. */
export function Button({
  id,
  label,
  inert,
  onEvent,
}: {
  id: string;
  label: string;
  inert: boolean;
  onEvent?: Told;
}) {
  return (
    <button
      type="button"
      data-node={id}
      disabled={inert}
      onClick={() => !inert && onEvent?.(id, "press")}
      className="rounded-lg border border-hair-strong px-3 py-1.5 text-sm enabled:hover:bg-text/5 disabled:opacity-60"
    >
      {label}
    </button>
  );
}

/** An input: what a person types goes to the skill as a change, and the key Enter as a submit. The text stands in the
 * field as typed, and follows the tree when the skill gives it another value. */
export function Field({
  id,
  value,
  hint,
  inert,
  onEvent,
}: {
  id: string;
  value: string;
  hint?: string | null;
  inert: boolean;
  onEvent?: Told;
}) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  return (
    <input
      data-node={id}
      value={draft}
      placeholder={hint ?? undefined}
      readOnly={inert}
      onChange={(event) => {
        if (inert) return;
        setDraft(event.target.value);
        onEvent?.(id, "change", event.target.value);
      }}
      onKeyDown={(event) => {
        if (!inert && event.key === "Enter") onEvent?.(id, "submit", draft);
      }}
      className="rounded-lg border border-hair-strong bg-well px-2.5 py-1.5 text-sm outline-none focus:border-accent read-only:opacity-60"
    />
  );
}

/** One node of a view, and what stands under it. */
function Part({ node, inert, onEvent }: { node: Node; inert: boolean; onEvent?: Told }): ReactNode {
  const below = (one: Node) => <Part key={one.id} node={one} inert={inert} onEvent={onEvent} />;
  switch (node.node) {
    case "box":
      return (
        <div
          data-node={node.id}
          style={{ display: "flex", flexDirection: node.axis, gap: `${node.gap * 0.25}rem` }}
          className={node.border ? "rounded-lg border border-hair p-2" : undefined}
        >
          {node.children.map(below)}
        </div>
      );
    case "text":
      return (
        <p data-node={node.id} className="whitespace-pre-wrap">
          {node.runs.map((run, at) => (
            <span key={placed(at)} className={tones[run.tone as Tone] ?? tones.text}>
              {run.text}
            </span>
          ))}
        </p>
      );
    case "list":
      return (
        <ul data-node={node.id} className="flex list-none flex-col gap-1">
          {node.items.map((one) => (
            <li key={one.id}>{below(one)}</li>
          ))}
        </ul>
      );
    case "table":
      return (
        <table data-node={node.id} className="w-full border-collapse text-left text-sm">
          <thead>
            <tr>
              {node.header.map((title, at) => (
                <th
                  key={placed(at)}
                  scope="col"
                  className="border-b border-hair px-2 py-1 font-medium text-muted"
                >
                  {title}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {node.rows.map((row, at) => (
              <tr key={row.map((one) => one.id).join("/") || `${at}`}>
                {row.map((one) => (
                  <td key={one.id} className="border-b border-hair px-2 py-1 align-top">
                    {below(one)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      );
    case "input":
      return <Field id={node.id} value={node.value} hint={node.hint} inert={inert} onEvent={onEvent} />;
    case "button":
      return <Button id={node.id} label={node.label} inert={inert} onEvent={onEvent} />;
    case "progress":
      return node.done == null ? (
        <progress data-node={node.id} className="w-full" />
      ) : (
        <progress
          data-node={node.id}
          className="w-full"
          max={1}
          value={Math.min(1, Math.max(0, node.done))}
        />
      );
    case "code":
      return (
        <pre
          data-node={node.id}
          data-language={node.language ?? undefined}
          className="overflow-x-auto rounded bg-well p-2 font-mono text-[0.92em]"
        >
          <code>{node.text}</code>
        </pre>
      );
    case "diff":
      return (
        <pre data-node={node.id} className="overflow-x-auto rounded bg-well p-2 font-mono text-[0.92em]">
          {node.text.split("\n").map((line, at) => (
            <div key={placed(at)} data-change={changed(line)} className={changes[changed(line)]}>
              {line || " "}
            </div>
          ))}
        </pre>
      );
    case "markdown":
      return (
        <div data-node={node.id}>
          <Markdown text={node.text} />
        </div>
      );
  }
}

/** The tree of a view, drawn in the DOM. */
export function View({ tree, inert = false, onEvent }: ViewProps) {
  return <Part node={tree} inert={inert} onEvent={onEvent} />;
}

/** What a slot follows: the chain of a dock, the chain of the act of a row, a detail or a mark, or the life. */
export function follows(slot: Slot): { slot: Slot["slot"]; chain?: string; act?: string } {
  if (slot.slot === "dock") return { slot: slot.slot, chain: slot.on };
  if (slot.slot === "row" || slot.slot === "detail" || slot.slot === "mark")
    return { slot: slot.slot, act: slot.act };
  return { slot: slot.slot };
}

/** A view in its slot. The slot names where it stands and what it follows, so the window places it. */
export function ViewSlot({ slot, tree, inert, onEvent, label }: ViewProps & { slot: Slot; label: string }) {
  const where = follows(slot);
  return (
    <section
      aria-label={label}
      data-slot={where.slot}
      data-chain={where.chain}
      data-act={where.act}
      className="min-w-0 text-sm"
    >
      <View tree={tree} inert={inert} onEvent={onEvent} />
    </section>
  );
}
