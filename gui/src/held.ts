// The life as the window holds it, and each part of the host taken into it. It reads no DOM, so the tests of the host
// merge the parts as the window merges them.

import type { LiveAct } from "@furb/engine/life";
import type { Life, Part, Project, StreamPart } from "./wire.ts";

/** What a model writes for a rung while its reply runs, as the window holds it. */
export type Streamed = Omit<StreamPart, "id" | "at" | "thinkingAt">;

/** The life as the window holds it: whether the host is connected, the projects, the life that it shows with its
 * acts, merged, and what each model writes now. */
export interface Held {
  connected: boolean;
  generation: number;
  projects: Project[];
  life: Life | null;
  acts: Map<string, LiveAct>;
  streams: Map<string, Streamed>;
  /** The count of the parts that changed the acts, which says to a view that they changed. */
  count: number;
}

export const empty: Held = {
  connected: false,
  generation: -1,
  projects: [],
  life: null,
  acts: new Map(),
  streams: new Map(),
  count: 0,
};

/** The text of a stream with a piece put at its place: a piece that the window holds already changes nothing. */
const placed = (held: string, at: number, piece: string) => held.slice(0, at) + piece;

/** A part of the host, taken into what the window holds. A whole part, or a part of another life, starts from
 * nothing. The acts grow in place, so a part of a large life copies none of them, and the count says that they
 * changed. */
export function merged(held: Held, part: Part): Held {
  const fresh = part.whole || part.generation !== held.generation;
  const acts = fresh ? new Map<string, LiveAct>() : held.acts;
  for (const act of part.acts) acts.set(act.id, act);
  const streams = new Map(fresh ? [] : held.streams);
  for (const piece of part.streams) {
    const was = streams.get(piece.id);
    streams.set(piece.id, {
      chain: piece.chain,
      text: placed(was?.text ?? "", piece.at, piece.text),
      thinking: placed(was?.thinking ?? "", piece.thinkingAt, piece.thinking),
      ...(piece.retry ? { retry: piece.retry } : {}),
    });
  }
  for (const id of part.ended) streams.delete(id);
  return {
    connected: held.connected,
    generation: part.generation,
    projects: part.projects ?? held.projects,
    life: part.life === undefined ? (fresh ? null : held.life) : part.life,
    acts,
    streams,
    count: held.count + (fresh || part.acts.length ? 1 : 0),
  };
}
