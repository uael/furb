// What the host of a life and the window say to each other over their socket. The host sends parts: what changed in
// the life that the window shows, and in the list of the projects. The window sends calls, each of which the host
// answers once.

import type { Ask, LiveAct, Measure } from "@furb/engine/life";

/** One record of a project, as the picker lists it. */
export interface Place {
  path: string;
  name: string;
  /** What the life on the record does, as the TUI says it, and why it did not open, when it did not. */
  status: string;
  error?: string;
  modified?: number;
  size?: number;
}

/** One project that the TUI and the app know, with its records, the newest first. */
export interface Project {
  directory: string;
  name: string;
  records: Place[];
}

/** The life that the window shows, as the host holds it, which a part sends whole when one of its words changed. */
export interface Life {
  /** The record of the life, and the chain that every record stands on. */
  record: string;
  root: string;
  /** The project of the life, and its name. */
  directory: string;
  place: string;
  /** The name of the session, as the TUI names it. */
  name: string;
  /** The chain that the composer sends to, and the actor of that chain. */
  selected: string;
  actor: string;
  /** What the life does, in the words of the TUI, and whether the host still reads it. */
  status: string;
  loading: boolean;
  /** The last notice and the last error of the session, which the TUI shows in its bar. */
  notice: string;
  error: string;
  /** Why the record is open to read: a life on it was refused, as on a record that another engine made. Nothing
   * for a life. */
  reading?: { why: string; made: string | null; engine: string; logged: boolean };
  /** Whether the record is a recording, which shows its log and runs nothing. */
  recorded: boolean;
  /** How many acts that an earlier life left wait for a resume. */
  pending: number;
  /** The dollars that the life spent. */
  cost: number;
  measures: Record<string, Measure>;
  /** What waits for the operator, the oldest first, and the question that the composer answers on the chain shown. */
  asks: Ask[];
  aimed?: string;
}

/** What a model writes for a rung while its reply runs: the text and the thought that came after the place that the
 * window holds, and the retry that waits, if one does. */
export interface StreamPart {
  id: string;
  chain: string;
  at: number;
  text: string;
  thinkingAt: number;
  thinking: string;
  retry?: { attempt: number; most: number; until: number; why: string };
}

/** What changed, as the host sends it. A part of another generation is of another life, and starts from nothing. */
export interface Part {
  generation: number;
  /** Whether the part holds the whole life, as the first part after the window connects does. */
  whole: boolean;
  /** The projects, when they changed. */
  projects?: Project[];
  /** The life, when it changed, and null when no life is open. */
  life?: Life | null;
  /** The acts that changed, in the order of the life. */
  acts: LiveAct[];
  streams: StreamPart[];
  /** The rungs whose streams ended. */
  ended: string[];
}

/** A word of the host that is no answer of a call: a part, a cancel that waits for the operator to confirm it since it
 * ends running work, or a message that the operator took back, which returns to the composer of its chain. */
export type Said =
  | { part: Part }
  | { confirm: { act: string; acts: string[]; chains: string } }
  | { compose: { chain: string; text: string } };

/** What a rewind to an act gives back of the files: each file that a write changed after it, and how many commands
 * ran after it, whose changes stay. */
export interface Restore {
  files: { path: string; made: boolean }[];
  commands: number;
}

/** The answer of a call, by the number of the call. */
export type Answer = { id: number; value?: unknown; error?: string };
