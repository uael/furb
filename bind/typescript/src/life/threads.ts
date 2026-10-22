import type { Exit, LiveAct } from "./acts.js";
import { type ActState, actState, valueText } from "./ladder.js";

/** Where a thread hangs: the act that it is about, a place in a file, and a pull request, each when it names one. */
export interface Anchor {
  act?: string;
  file?: string;
  pull?: Pull;
}
/** A pull request that a thread names, by its number and its address. */
export interface Pull {
  number: number;
  url: string;
}
/** One thread of a life: a prompt, with the messages that came to it, its sub-threads, and what its tree holds. */
export interface Thread {
  act: LiveAct;
  /** The chain that the prompt stands on, whose actor serves it. */
  chain: string;
  /** The first line of its message, its shape and its actor. */
  title: string;
  shape: string;
  to: string;
  state: ActState;
  /** How deep it stands in the tree of threads, from 0 at the top. */
  depth: number;
  anchor?: Anchor;
  /** The prompts on the thread, in order: its messages, and the prompts that opened it again. */
  messages: LiveAct[];
  /** The prompts that the rungs of its ladder made, in the order they began. */
  children: Thread[];
  /** How many threads of its tree closed, of all of them, it not among them. */
  closed: number;
  total: number;
  /** The open prompts to the operator of its tree, it among them. */
  owed: string[];
  /** How many open threads of its tree a model works on. */
  working: number;
  /** The pull requests that its tree names, in the order they came. */
  pulls: Pull[];
}

const named = /\b(?:write|read|bash|rung|prompt|chain|wait|cd)\d+\b/g;
const place = /(?<![\w/])((?:[\w.-]+\/)*[\w.-]+\.\w+):(\d+(?:-\d+)?)\b/;
const pulled = /https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/pull\/(\d+)/g;

/** The pull requests that a text names, by their addresses. */
export function pullsOf(text: string): Pull[] {
  return [...text.matchAll(pulled)].map((match) => ({ number: Number(match[1]), url: match[0] }));
}

/** Where a prompt hangs: the first act of the life that its message names, the first place in a file, and the first
 * pull request. */
export function anchorOf(prompt: LiveAct, rows: ReadonlyMap<string, LiveAct>): Anchor | undefined {
  const message = String(prompt.words[1] ?? "");
  const act = [...message.matchAll(named)]
    .map((match) => match[0])
    .find((id) => id !== prompt.id && rows.has(id));
  const file = message.match(place);
  const [pull] = pullsOf(message);
  if (!act && !file && !pull) return undefined;
  return {
    ...(act ? { act } : {}),
    ...(file ? { file: `${file[1]}:${file[2]}` } : {}),
    ...(pull ? { pull } : {}),
  };
}

/** The first prompt of the thread that a prompt stands in: the prompt itself, or the prompt that its thread starts
 * at, walking up the threads. */
export function rootOf(prompt: LiveAct, rows: ReadonlyMap<string, LiveAct>): string {
  let at = prompt;
  for (let step = 0; step < 64; step++) {
    const up = at.thread ? rows.get(at.thread) : undefined;
    if (!up) break;
    at = up;
  }
  return at.id;
}

/** The thread that an act stands in: the first prompt of the thread whose rung is over it, walking up the makers. A
 * prompt that the operator, an ear or a chain made stands in none. */
export function threadOf(act: LiveAct, rows: ReadonlyMap<string, LiveAct>): string | undefined {
  let maker = rows.get(act.by);
  for (let step = 0; maker && step < 64; step++) {
    if (maker.kind === "prompt") return rootOf(maker, rows);
    if (maker.kind !== "rung") return undefined;
    maker = rows.get(maker.by);
  }
  return undefined;
}

/** Whether a prompt is a message of a thread and no thread of its own: a prompt on a thread, which the engine makes a
 * message of that thread or a prompt that opens it again, or a prompt of None that a chain made on the chain, as the
 * acknowledgment of an act that a rung of the operator made, which belongs to the thread of that act. */
function messageOf(act: LiveAct, rows: ReadonlyMap<string, LiveAct>): string | undefined {
  if (act.thread) return rootOf(act, rows);
  if (act.words[0] !== "None" || rows.get(act.by)?.kind !== "chain") return undefined;
  const about = String(act.words[1] ?? "").match(/^(\w+?\d+) /)?.[1];
  const subject = about ? rows.get(about) : undefined;
  return subject
    ? subject.kind === "prompt"
      ? (threadOf(subject, rows) ?? subject.id)
      : threadOf(subject, rows)
    : "";
}

/** The tree of the threads of a life: each prompt under the prompt of the ladder whose rung made it, each with its
 * anchor, its messages, its progress, what waits for the operator in it, and its pull requests. */
export function threads(acts: readonly LiveAct[], owed: ReadonlySet<string>): Thread[] {
  const rows = new Map(acts.map((act) => [act.id, act]));
  const all = new Map<string, Thread>();
  const tops: Thread[] = [];
  const later: [LiveAct, string][] = [];
  for (const act of acts) {
    if (act.kind !== "prompt") continue;
    const message = messageOf(act, rows);
    if (message !== undefined) {
      if (message) later.push([act, message]);
      continue;
    }
    const [shape = "", text = "", to = ""] = act.words.map(String);
    const thread: Thread = {
      act,
      chain: act.on,
      title: text.split("\n").find((line) => line.trim()) ?? "",
      shape,
      to,
      state: actState(act, owed),
      depth: 0,
      anchor: anchorOf(act, rows),
      messages: [],
      children: [],
      closed: 0,
      total: 0,
      owed: [],
      working: 0,
      pulls: [],
    };
    all.set(act.id, thread);
    const parent = all.get(threadOf(act, rows) ?? "");
    if (parent) parent.children.push(thread);
    else tops.push(thread);
  }
  for (const [act, id] of later) all.get(id)?.messages.push(act);
  // The commands of each ladder print the pull requests that they open.
  const printed = new Map<string, Pull[]>();
  for (const act of acts) {
    if (act.kind !== "bash" || !act.done) continue;
    const exit = act.value as Exit | undefined;
    const found = pullsOf(`${exit?.stdout?.content ?? ""}${exit?.stderr?.content ?? ""}`);
    const id = threadOf(act, rows);
    if (found.length && id) printed.set(id, [...(printed.get(id) ?? []), ...found]);
  }
  const fill = (thread: Thread, depth: number): void => {
    thread.depth = depth;
    const own = [
      ...(thread.act.done ? pullsOf(valueText(thread.act.value)) : []),
      ...(printed.get(thread.act.id) ?? []),
    ];
    const pulls = new Map(own.map((pull) => [pull.url, pull]));
    if (thread.state === "asks") thread.owed.push(thread.act.id);
    for (const child of thread.children) {
      fill(child, depth + 1);
      thread.total += 1 + child.total;
      thread.closed += (child.act.done ? 1 : 0) + child.closed;
      thread.owed.push(...child.owed);
      thread.working +=
        (!child.act.done && !child.act.paused && child.to !== "operator" ? 1 : 0) + child.working;
      for (const pull of child.pulls) pulls.set(pull.url, pull);
    }
    thread.pulls = [...pulls.values()];
  };
  for (const top of tops) fill(top, 0);
  return tops;
}

/** The threads of a tree, one under the other in the order of the tree, less the sub-threads of each folded one. */
export function outline(tops: readonly Thread[], folds: ReadonlySet<string>): Thread[] {
  const out: Thread[] = [];
  const visit = (thread: Thread): void => {
    out.push(thread);
    if (!folds.has(thread.act.id)) for (const child of thread.children) visit(child);
  };
  for (const top of tops) visit(top);
  return out;
}
