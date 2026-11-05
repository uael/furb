import type { Exit, LiveAct } from "./acts.js";
import { valueText } from "./ladder.js";
import { actState, type State } from "./states.js";

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
  state: State;
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

/** The acts of a life by their names, as a host indexes them. */
export type Rows = Pick<ReadonlyMap<string, LiveAct>, "get">;

/** The first prompt of the thread that a prompt stands in: the prompt itself, or the prompt that its thread starts
 * at, walking up the threads. */
export function rootOf(prompt: LiveAct, rows: Rows): string {
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
export function threadOf(act: LiveAct, rows: Rows): string | undefined {
  let maker = rows.get(act.by);
  for (let step = 0; maker && step < 64; step++) {
    if (maker.kind === "prompt") return rootOf(maker, rows);
    if (maker.kind !== "rung") return undefined;
    maker = rows.get(maker.by);
  }
  return undefined;
}

/** The name of an act, as the engine reads it: the name of an ear or of the operator ends with no number. */
const ACT = /^\w+?\d+$/;

/** What a prompt of a chain is that is no conversation, which the feed of its chain draws as what it is and the list
 * of its threads leaves out: a notice of the round, a prompt of None that a rung or a chain made, which no one answers;
 * what a word of the operator told the chain, a prompt of None that a rung of his made or the acknowledgment of a rung
 * in no thread; a wake, a prompt of None that an ear or a verb of the round made; or a question of another chain to
 * this one, a worker that asks its coordinator, with the chain that asks. The view model of the host says the same in
 * `host/src/view/threads.rs`, which the TUI does not read yet. */
export type Aside =
  | { aside: "notice" }
  | { aside: "rung" }
  | { aside: "wake" }
  | { aside: "question"; from: string };

/** A prompt of a chain that is no thread, with what it is and the prompts on it, in order. */
export interface ChainAside {
  act: LiveAct;
  aside: Aside;
  messages: LiveAct[];
}

/** Whether a chain stands under another, or is it, walking up the makers of the chains. */
function within(chain: string, of: string, rows: Rows): boolean {
  let at = chain;
  for (let step = 0; step < 64; step++) {
    if (at === of) return true;
    const made = rows.get(at);
    const maker = made ? rows.get(made.by) : undefined;
    if (!maker || maker.on === at) return false;
    at = maker.on;
  }
  return false;
}

/** What a prompt to a model is: a message of a thread, as a prompt on a thread, which the engine makes a message of
 * that thread or a prompt that opens it again, or a prompt of None that a chain made as the acknowledgment of an act of
 * that thread; an aside, which message gives as ""; or else a thread of its own, as each prompt that the operator made
 * himself, which message gives as undefined. whole says whether each act that the verdict walked stood in rows: a
 * verdict that an act which has not landed yet would change is no verdict yet. */
function messageOf(act: LiveAct, rows: Rows): { message: string | undefined; aside?: Aside; whole: boolean } {
  let whole = true;
  const walked: Rows = {
    get(id) {
      const got = rows.get(id);
      if (!got && ACT.test(id)) whole = false;
      return got;
    },
  };
  const verdict = (): string | Aside | undefined => {
    if (act.thread) return rootOf(act, walked);
    const maker = walked.get(act.by);
    const none = act.words[0] === "None";
    if (none && maker?.kind === "chain") {
      const about = String(act.words[1] ?? "").match(/^(\w+?\d+) /)?.[1];
      const subject = about ? walked.get(about) : undefined;
      if (!subject) return { aside: "notice" };
      if (subject.kind === "prompt") return threadOf(subject, walked) ?? subject.id;
      return threadOf(subject, walked) ?? { aside: subject.kind === "rung" ? "rung" : "notice" };
    }
    if (byOperator(act)) return undefined;
    if (none && maker?.kind === "rung" && byOperator(maker)) return { aside: "rung" };
    if (none && (maker?.kind === "rung" || maker?.kind === "prompt")) return { aside: "notice" };
    if (none) return { aside: "wake" };
    if (maker && maker.on !== act.on && !within(act.on, maker.on, walked))
      return { aside: "question", from: maker.on };
    return undefined;
  };
  const said = verdict();
  if (typeof said === "object") return { message: "", aside: said, whole };
  return { message: said, whole };
}

/** What a list of the threads of a chain keeps of each prompt that it read: the thread that it is a message of, what
 * it is when it is no conversation, and its title. */
export type PromptRead = { message: string | undefined; title: string; aside?: Aside };

/** The tree of the threads of a life: each prompt under the prompt of the ladder whose rung made it, each with its
 * anchor, its messages, its progress, what waits for the operator in it, and its pull requests. */
export function threads(acts: readonly LiveAct[], owed: ReadonlySet<string>): Thread[] {
  const rows = new Map(acts.map((act) => [act.id, act]));
  const all = new Map<string, Thread>();
  const tops: Thread[] = [];
  const later: [LiveAct, string][] = [];
  for (const act of acts) {
    if (act.kind !== "prompt") continue;
    // A message joins its thread, and an aside is no thread.
    const { message } = messageOf(act, rows);
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
        (!child.act.done && !child.act.paused && !child.act.unfinished && child.to !== "operator" ? 1 : 0) +
        child.working;
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

/** One thread of a chain, as the list of the threads of that chain shows it: its prompt, the first line of its message,
 * its messages, the questions and the notices that it made for the operator, the questions that wait in it, and its
 * state. A thread is resolved when its prompt is over. A message of None on an open thread is over at once, so it
 * holds nothing open, and a prompt of str on a resolved thread opens it again until its model closes that prompt. */
export interface ChainThread {
  act: LiveAct;
  title: string;
  messages: LiveAct[];
  /** The prompts to the operator that the thread made, answered or not, in the order they began. */
  asked: LiveAct[];
  /** The ids of the questions that wait for the operator. */
  asks: string[];
  state: State;
  resolved: boolean;
  /** Who made its prompt: the operator, a rung of a word, or an ear. */
  origin: "operator" | "rung" | "ear";
  /** Whether the operator started it, directly or by a word of their own. */
  yours: boolean;
}

/** The acts whose changes are the work of a thread: its prompt, its messages and its prompts to the operator. */
export function threadWork(thread: Pick<ChainThread, "act" | "messages" | "asked">): LiveAct[] {
  return [thread.act, ...thread.messages, ...thread.asked];
}

/** Whether an act is the operator's own: made by them, or by one of their sessions. */
function byOperator(act: { by: string }): boolean {
  return act.by === "operator" || act.by.startsWith("operator/");
}

/** How a thread concerns the operator: a question waits for them in it, or they started it and it failed or is held. */
export type Concern = "asks" | "failed" | "paused";

/** How a thread concerns the operator, or nothing when it does not, from the state that its node shows. The failure of
 * an agent does not concern them: it rolls into the thread that made it, and only a question escalates. */
export function concerns(
  node: Pick<ThreadNode, "state"> & { thread: Pick<ChainThread, "yours"> },
): Concern | undefined {
  if (node.state === "asks") return "asks";
  if (node.thread.yours && (node.state === "failed" || node.state === "paused")) return node.state;
  return undefined;
}

/** The threads of one chain, in the order they began: each prompt to a model on the chain that is no message of
 * another thread. It asks the operator while a question waits in it. The acts on the chain and the index of every act
 * of a host spare a pass over every act, and what a host keeps of each prompt it read spares a read of it again. What
 * it keeps belongs to the act itself, so an act that changed, an act of another session and an act of another branch,
 * each a new object, are read again, and no prompt keeps what another prompt of the same id was. It keeps no verdict
 * that an act which has not landed yet would change, so the order in which the acts land changes no verdict. */
export function chainThreads(
  on: readonly LiveAct[],
  rows: ReadonlyMap<string, LiveAct>,
  owed: ReadonlySet<string>,
  known: WeakMap<LiveAct, PromptRead> = new WeakMap(),
): ChainThread[] {
  return chainSorted(on, rows, owed, known).threads;
}

/** The asides of one chain, in the order they began: each prompt to a model on the chain that is no conversation, with
 * the prompts on it, and each message of a thread that is no thread of the chain, as a notice. */
export function chainAsides(
  on: readonly LiveAct[],
  rows: ReadonlyMap<string, LiveAct>,
  owed: ReadonlySet<string>,
  known: WeakMap<LiveAct, PromptRead> = new WeakMap(),
): ChainAside[] {
  return chainSorted(on, rows, owed, known).asides;
}

/** The threads and the asides of one chain, in the order they began. */
export function chainSorted(
  on: readonly LiveAct[],
  rows: ReadonlyMap<string, LiveAct>,
  owed: ReadonlySet<string>,
  known: WeakMap<LiveAct, PromptRead> = new WeakMap(),
): { threads: ChainThread[]; asides: ChainAside[] } {
  const out = new Map<string, ChainThread>();
  const asides = new Map<string, ChainAside>();
  const later: [LiveAct, string][] = [];
  const waits: [string, string][] = [];
  const asked: [string, LiveAct][] = [];
  for (const act of on) {
    if (act.kind !== "prompt") continue;
    if (act.words[2] === "operator") {
      const thread = threadOf(act, rows);
      if (thread) asked.push([thread, act]);
      if (thread && owed.has(act.id)) waits.push([thread, act.id]);
      continue;
    }
    let seen = known.get(act);
    if (!seen) {
      const { message, aside, whole } = messageOf(act, rows);
      seen = {
        message,
        ...(aside ? { aside } : {}),
        title:
          message === undefined
            ? (String(act.words[1] ?? "")
                .split("\n")
                .find((line) => line.trim()) ?? "")
            : "",
      };
      // What a prompt is stays as it was said once each act that it names has landed, and only then a host keeps it.
      if (whole) known.set(act, seen);
    }
    if (seen.aside) asides.set(act.id, { act, aside: seen.aside, messages: [] });
    if (seen.message !== undefined) {
      if (seen.message) later.push([act, seen.message]);
      continue;
    }
    const by = byOperator(act);
    const maker = rows.get(act.by);
    const origin = by ? "operator" : maker?.kind === "rung" ? "rung" : "ear";
    out.set(act.id, {
      act,
      title: seen.title,
      messages: [],
      asked: [],
      asks: [],
      state: "done",
      resolved: true,
      origin,
      yours: by || (origin === "rung" && maker !== undefined && byOperator(maker)),
    });
  }
  for (const [act, id] of later) {
    const aside = asides.get(id);
    if (out.has(id)) out.get(id)?.messages.push(act);
    else if (aside) aside.messages.push(act);
    else asides.set(act.id, { act, aside: { aside: "notice" }, messages: [] });
  }
  for (const [id, ask] of asked) out.get(id)?.asked.push(ask);
  for (const [id, ask] of waits) out.get(id)?.asks.push(ask);
  for (const thread of out.values()) {
    const open = [thread.act, ...thread.messages].findLast((act) => !act.done);
    thread.resolved = !open;
    thread.state = thread.asks.length ? "asks" : actState(open ?? thread.act, owed);
  }
  return {
    threads: [...out.values()],
    asides: [...asides.values()].sort((a, b) => a.act.began - b.act.began),
  };
}

/** A thread of a list with the threads that its ladder made, its agents, each nested the same way. state is the state
 * that the thread shows: its own, or asks while a question waits in an agent under it. */
export interface ThreadNode {
  thread: ChainThread;
  agents: ThreadNode[];
  state: State;
}

/** The threads of a list as a tree: each thread under the thread whose ladder made its prompt, as [`threadOf`] walks
 * the makers, and a thread whose maker is no thread of the list at the top, each list in the order that it was given.
 * A question that waits in an agent sets the state of each node over it to asks, so the state of a node is the one
 * roll-up. A failure or an unread in an agent does not roll up. It changes no thread of the list. */
export function nestThreads(list: readonly ChainThread[], rows: Rows): ThreadNode[] {
  const nodes = new Map(
    list.map((thread): [string, ThreadNode] => [thread.act.id, { thread, agents: [], state: thread.state }]),
  );
  const tops: ThreadNode[] = [];
  for (const node of nodes.values()) {
    const parent = nodes.get(threadOf(node.thread.act, rows) ?? "");
    if (parent && parent !== node) parent.agents.push(node);
    else tops.push(node);
  }
  const rolled = (node: ThreadNode): boolean => {
    const below = node.agents.map(rolled).includes(true);
    if (below) node.state = "asks";
    return node.state === "asks";
  };
  for (const top of tops) rolled(top);
  return tops;
}

/** The threads of a tree in the order that the list of the threads shows them: the closed ones first and the open ones
 * last, so that the open ones stand nearest the input, and in each group by the time of their last message, the oldest
 * first. The agents of each thread take the same order. The view model of the host says the same in
 * `host/src/view/threads.rs`, which the TUI does not read yet. */
export function orderThreads(nodes: readonly ThreadNode[]): ThreadNode[] {
  const last = (node: ThreadNode) =>
    Math.max(node.thread.act.began, ...node.thread.messages.map((one) => one.began));
  return nodes
    .map((node) => ({ ...node, agents: orderThreads(node.agents) }))
    .sort((one, other) => Number(threadOpen(one)) - Number(threadOpen(other)) || last(one) - last(other));
}

/** Whether a thread stands open in the list of the threads: it is not resolved, or a question waits in it or in an
 * agent under it. */
export function threadOpen(
  node: Pick<ThreadNode, "state"> & { thread: Pick<ChainThread, "resolved"> },
): boolean {
  return !node.thread.resolved || node.state === "asks";
}

/** What a tree of threads says to the operator, counted by thread: a question counts once, in the thread where it
 * waits, and the failure or the pause of a thread that they started counts in that thread. */
export function attentionOf(tops: readonly ThreadNode[]): Record<Concern, number> {
  const count: Record<Concern, number> = { asks: 0, failed: 0, paused: 0 };
  const visit = (node: ThreadNode): void => {
    const kind = concerns(node);
    if (kind && !(kind === "asks" && node.agents.some((one) => one.state === "asks"))) count[kind] += 1;
    for (const one of node.agents) visit(one);
  };
  for (const top of tops) visit(top);
  return count;
}
