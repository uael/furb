import { type LiveAct, operated } from "./acts.js";
import { actState, type State } from "./states.js";

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
 * `src/view/threads.rs`, which the TUI does not read yet. */
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
    if (operated(act.by)) return undefined;
    if (none && maker?.kind === "rung" && operated(maker.by)) return { aside: "rung" };
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

/** A run of notes: the asides of a chain that are no question and follow one another in its feed with no card of
 * another kind between them, which a window draws as one block. The view model of the host says the same in
 * `chain_notes` of `src/view/threads.rs`. */
export interface NoteRun {
  prompts: LiveAct[];
  count: number;
}

/** The runs of notes of one chain, from its acts in the order they began, and its threads and asides as chainSorted
 * gives them. Only a step of the top of the chain draws a card: an act that a rung of the chain made stands in the
 * program of that rung, a message stands in its thread or its note, and a grant, a chain and a rung that the chain
 * wrote to retell its source draw none. Each other act ends the run that it follows, as a thread, a question, a command
 * and a word of the operator do. */
export function chainNotes(
  on: readonly LiveAct[],
  rows: Rows,
  sorted: { threads: readonly ChainThread[]; asides: readonly ChainAside[] },
): NoteRun[] {
  const notes = new Set(
    sorted.asides.filter((one) => one.aside.aside !== "question").map((one) => one.act.id),
  );
  const inside = new Set(
    [
      ...sorted.threads.flatMap((thread) => [...thread.messages, ...thread.asked]),
      ...sorted.asides.flatMap((one) => one.messages),
    ].map((act) => act.id),
  );
  const runs: NoteRun[] = [];
  let open = false;
  for (const act of on) {
    const maker = rows.get(act.by);
    const below = maker !== undefined && maker.on === act.on && maker.kind !== "chain";
    const retold = act.kind === "rung" && maker?.kind === "chain";
    if (below || retold || act.kind === "chain" || act.kind === "grant" || inside.has(act.id)) continue;
    if (!notes.has(act.id)) {
      open = false;
      continue;
    }
    if (!open) runs.push({ prompts: [], count: 0 });
    open = true;
    const run = runs[runs.length - 1] as NoteRun;
    run.prompts.push(act);
    run.count++;
  }
  return runs;
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
    const by = operated(act.by);
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
      yours: by || (origin === "rung" && maker !== undefined && operated(maker.by)),
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
 * `src/view/threads.rs`, which the TUI does not read yet. */
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
