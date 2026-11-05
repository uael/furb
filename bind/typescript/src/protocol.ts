/* The types of the one protocol of the host, made from host/protocol.schema.json by
 * bind/typescript/script/protocol.ts: run bun run protocol after a change of the protocol. */

/**
 * One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
 * `act`, since `id` is the key of the request.
 */
export type Request = {
  id: number;
} & Request1;
export type Request1 =
  | (Hello & {
      verb: "hello";
    })
  | {
      verb: "list";
    }
  | {
      life: string;
      cwd?: string | null;
      actor?: string | null;
      roster?: string[] | null;
      extensions?: boolean | null;
      config?: string | null;
      claude?: string | null;
      answers?: boolean;
      attach?: number | null;
      verb: "open";
    }
  | {
      life: string;
      from: number;
      verb: "attach";
    }
  | {
      life: string;
      verb: "detach";
    }
  | {
      life: string;
      holder: string;
      verb: "pin";
    }
  | {
      life: string;
      holder: string;
      verb: "unpin";
    }
  | {
      life: string;
      verb: "presence";
    }
  | {
      life: string;
      act: string;
      verb: "get";
    }
  | {
      life: string;
      act: string;
      verb: "peek";
    }
  | {
      life: string;
      on?: string | null;
      of?: string | null;
      verb: "transcript";
    }
  | {
      life: string;
      on?: string | null;
      verb: "turns";
    }
  | {
      life: string;
      on?: string | null;
      verb: "program";
    }
  | {
      life: string;
      verb: "standing";
    }
  | {
      life: string;
      act: string;
      verb: "filled";
    }
  | {
      life: string;
      on?: string | null;
      verb: "cwd";
    }
  | {
      life: string;
      name: string;
      on?: string | null;
      verb: "inspect";
    }
  | {
      life: string;
      verb: "pending";
    }
  | {
      life: string;
      shape: string;
      message: string;
      to?: string | null;
      on?: string | null;
      verb: "prompt";
    }
  | {
      life: string;
      act: string;
      line: string;
      verb: "answer";
    }
  | {
      life: string;
      act: string;
      why?: string | null;
      verb: "pause";
    }
  | {
      life: string;
      act: string;
      verb: "wake";
    }
  | {
      life: string;
      act: string;
      why?: string | null;
      verb: "cancel";
    }
  | {
      life: string;
      label?: string | null;
      source?: string | null;
      on?: string | null;
      verb: "chain";
    }
  | {
      life: string;
      word: string;
      on?: string | null;
      verb: "run";
    }
  | {
      life: string;
      act: string;
      value: string;
      verb: "close";
    }
  | {
      life: string;
      usd?: number | null;
      share?: number | null;
      on?: string | null;
      verb: "grant";
    }
  | {
      life: string;
      command: string;
      stdin?: string | null;
      feed?: boolean;
      on?: string | null;
      verb: "bash";
    }
  | {
      life: string;
      path: string;
      on?: string | null;
      verb: "read";
    }
  | {
      life: string;
      path: string;
      content: string;
      on?: string | null;
      verb: "write";
    }
  | {
      life: string;
      on?: string | null;
      verb: "names";
    }
  | {
      life: string;
      word: string;
      on?: string | null;
      verb: "gate";
    }
  | {
      life: string;
      verb: "stand";
    }
  | {
      life: string;
      about: string;
      verb: "seen";
    }
  | {
      life: string;
      entry: string;
      shape: string;
      message: string;
      on?: string | null;
      verb: "queue";
    }
  | {
      life: string;
      entry: string;
      verb: "unqueue";
    }
  | {
      life: string;
      on?: string | null;
      verb: "queued";
    }
  | {
      life: string;
      notices?: string[];
      verb: "desk";
    }
  | {
      life: string;
      on?: string | null;
      at?: number | null;
      verb: "threads";
    }
  | {
      life: string;
      at: number;
      verb: "as_of";
    }
  | {
      life: string;
      verb: "moments";
    }
  | {
      life: string;
      ask: number;
      turn?: unknown;
      why?: string | null;
      verb: "answered";
    }
  | {
      life: string;
      ask: number;
      text: string;
      thinking: string;
      verb: "wrote";
    }
  | {
      verb: "handover";
    }
  | (Mount & {
      verb: "view.mount";
    })
  | (Tree & {
      verb: "view.tree";
    })
  | (ViewEvent & {
      verb: "view.event";
    })
  | (ViewEffect & {
      verb: "view.effect";
    })
  | (Dispose & {
      verb: "view.dispose";
    });
/**
 * What a peer is: a window of an operator, the process of a skill, or the daemon of a new build,
 * which asks the handover.
 */
export type Kind = "window" | "skill" | "daemon";
/**
 * Where a window shows a view. The dock follows the chain that it names, and the row, the detail
 * and the mark follow the chain of the act that they name. The sidebar, the footer, a screen and
 * an overlay follow the life.
 */
export type Slot =
  | {
      on: string;
      slot: "dock";
    }
  | {
      slot: "sidebar";
    }
  | {
      act: string;
      slot: "row";
    }
  | {
      act: string;
      slot: "detail";
    }
  | {
      act: string;
      slot: "mark";
    }
  | {
      slot: "footer";
    }
  | {
      slot: "screen";
    }
  | {
      slot: "overlay";
    };
/**
 * One node of a view, under an id that an event names.
 */
export type Node = {
  id: string;
} & Node1;
export type Node1 =
  | {
      axis: Axis;
      gap: number;
      border: boolean;
      children: Node[];
      node: "box";
    }
  | {
      runs: Run[];
      node: "text";
    }
  | {
      items: Node[];
      node: "list";
    }
  | {
      header: string[];
      rows: Node[][];
      node: "table";
    }
  | {
      value: string;
      hint?: string | null;
      /**
       * A node that acts is inert in the window of a peer that only reads: it draws and takes no
       * gesture.
       */
      inert?: boolean;
      node: "input";
    }
  | {
      label: string;
      inert?: boolean;
      node: "button";
    }
  | {
      done?: number | null;
      node: "progress";
    }
  | {
      text: string;
      language?: string | null;
      node: "code";
    }
  | {
      text: string;
      node: "diff";
    }
  | {
      text: string;
      node: "markdown";
    };
/**
 * The way a box lays out its children.
 */
export type Axis = "row" | "column";
/**
 * The tone of a run, which each window gives its own color.
 */
export type Tone = "text" | "muted" | "faint" | "accent" | "success" | "warning" | "danger";
/**
 * What a person does to a node.
 */
export type Gesture = "press" | "change" | "submit";
/**
 * What a skill asks of a window.
 */
export type Effect =
  | {
      url: string;
      effect: "open";
    }
  | {
      text: string;
      effect: "draft";
    }
  | {
      text: string;
      effect: "notice";
    };
/**
 * What the daemon says to a peer.
 */
export type Frame =
  | {
      id: number;
      answer: Answer;
      frame: "reply";
    }
  | {
      id?: number | null;
      error: ProtocolError;
      why: string;
      frame: "refused";
    }
  | {
      life: string;
      seq: number;
      event: Event;
      frame: "event";
    }
  | {
      life: string;
      peers: Peer[];
      frame: "presence";
    }
  | {
      life: string;
      acts: Open[];
      notice: string;
      frame: "stalled";
    }
  | {
      life: string;
      ask: number;
      request: unknown;
      frame: "ask";
    }
  | (Mount & {
      frame: "view.mount";
    })
  | (Tree & {
      frame: "view.tree";
    })
  | (ViewEvent & {
      frame: "view.event";
    })
  | (ViewEffect & {
      frame: "view.effect";
    })
  | (Dispose & {
      frame: "view.dispose";
    });
/**
 * What a verb comes to.
 */
export type Answer =
  | {
      answer: "done";
    }
  | {
      answer: "welcome";
      data: {
        protocol: number;
        peer: Peer;
      };
    }
  | {
      answer: "lives";
      data: Listed[];
    }
  | {
      answer: "act";
      data: string;
    }
  | {
      answer: "value";
      data: unknown;
    }
  | {
      answer: "values";
      data: unknown[];
    }
  | {
      answer: "inspected";
      data: {
        kind: string;
        repr: string;
        value: unknown;
      };
    }
  | {
      answer: "presence";
      data: Peer[];
    }
  | {
      answer: "handed";
      data: Handed[];
    };
/**
 * What a peer may do: read a life, or read and write it. Write holds each right of read.
 */
export type Role = "read" | "write";
/**
 * Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
 */
export type ProtocolError =
  | {
      ours: number;
      theirs: number;
      error: "protocol";
    }
  | {
      error: "first";
    }
  | {
      error: "again";
    }
  | {
      needs: Role;
      has: Role;
      error: "role";
    }
  | {
      why: string;
      error: "unread";
    }
  | {
      bound: number;
      error: "large";
    }
  | {
      why: string;
      error: "refused";
    }
  | {
      error: "stopped";
    }
  | {
      life: string;
      error: "closed";
    }
  | {
      life: string;
      error: "relative";
    }
  | {
      life: string;
      error: "unattached";
    }
  | {
      life: string;
      holder: string;
      error: "unpinned";
    }
  | {
      life: string;
      ask: number;
      error: "unasked";
    }
  | {
      life: string;
      ask: number;
      error: "foreign";
    }
  | {
      life: string;
      error: "answerless";
    }
  | {
      needs: Kind;
      has: Kind;
      error: "speaker";
    }
  | {
      error: "nameless";
    }
  | {
      life: string;
      skill: string;
      error: "no_skill";
    }
  | {
      viewer: string;
      error: "unmounted";
    }
  | {
      viewer: string;
      error: "mounted";
    };
/**
 * What a life tells its clients, in the order of its log.
 */
export type Event =
  | {
      event: "fact";
      data: unknown;
    }
  | {
      event: "act";
      data: Act;
    }
  | {
      event: "text";
      data: {
        rung: string;
        chain: string;
        text: string;
        thinking: string;
      };
    }
  | {
      event: "queued";
      data: {
        chain: string;
        waiting: FollowUp[];
      };
    }
  | {
      event: "notice";
      data: string;
    }
  | {
      event: "changed";
      data: string;
    }
  | {
      event: "reborn";
      data: string | null;
    }
  | {
      event: "ended";
      data: string | null;
    };
/**
 * Where an act stands: asked, started by the ear that owns it, or done, with what it came to.
 */
export type Phase =
  | {
      phase: "asked";
    }
  | {
      phase: "started";
    }
  | {
      phase: "done";
      value: unknown;
    };
/**
 * How the word of a rung ran: it runs or waits for its word, it ran, a fault or a cancel ended it,
 * or the gate refused it, each with what a person reads.
 */
export type RunState =
  | {
      run: "running";
    }
  | {
      run: "done";
    }
  | {
      run: "failed";
      reason: string;
    }
  | {
      run: "cancelled";
      reason: string;
    }
  | {
      run: "refused";
      reason: string;
    };
/**
 * The state of work: it asks the operator, it ended while the operator looked away, it failed, a
 * pause holds it, its model thinks or writes its word, a word or a command of it runs, its word
 * awaits a prompt on another chain, it ended well, a cancel ended it, or it rests. The order is
 * the order of the roll-up. Only a window says unread, since only it knows what its operator saw.
 */
export type State =
  | "asks"
  | "unread"
  | "failed"
  | "paused"
  | "thinks"
  | "runs"
  | "delegates"
  | "done"
  | "cancelled"
  | "idle";

/**
 * Both ways of the protocol, which its schema holds.
 */
export interface Protocol {
  request: Request;
  frame: Frame;
}
/**
 * The first frame of a peer: the protocol that it speaks, its kind, and the name of a skill.
 */
export interface Hello {
  protocol: number;
  kind: Kind;
  name?: string | null;
}
/**
 * A view that a window mounts in a slot of a life, which the process of a skill serves.
 */
export interface Mount {
  viewer: string;
  life: string;
  slot: Slot;
  skill: string;
}
/**
 * The tree of a view, which takes the place of each tree of a lower version.
 */
export interface Tree {
  viewer: string;
  slot: Slot;
  version: number;
  tree: Node;
}
/**
 * A run of a text, in a tone.
 */
export interface Run {
  text: string;
  tone?: Tone & string;
}
/**
 * What a person did to a node of a view, with the value of an input.
 */
export interface ViewEvent {
  viewer: string;
  node: string;
  kind: Gesture;
  value?: string | null;
}
/**
 * What a skill asks of the one window of a view.
 */
export interface ViewEffect {
  viewer: string;
  effect: Effect;
}
/**
 * The end of a view in its window.
 */
export interface Dispose {
  viewer: string;
}
/**
 * A peer as the daemon knows it after its hello: the login that its transport proved, its role,
 * its kind, and the name of a skill.
 */
export interface Peer {
  login: string;
  role: Role;
  kind: Kind;
  name?: string | null;
}
/**
 * A record of the machine, and whether a life of the daemon holds it now.
 */
export interface Listed {
  life: string;
  open: boolean;
}
/**
 * A life that a daemon gave at its handover: its record, its folder, and the holder of each pin.
 */
export interface Handed {
  life: string;
  cwd: string;
  pins: string[];
}
/**
 * One act of a life with its meaning, as it stands after the last fact about it.
 */
export interface Act {
  kind: string;
  id: string;
  by: string;
  /**
   * What the act is on: a chain, or the prompt of a thread for a prompt on a thread.
   */
  on: string;
  /**
   * The chain that the act stands in.
   */
  chain: string;
  /**
   * The words of the act after its kind, its name, its maker and what it is on.
   */
  words: unknown[];
  phase: Phase;
  /**
   * Whether the console of the operator took the act: a question to him, or a notice.
   */
  operator: boolean;
  /**
   * The pause that holds the act while it lives.
   */
  pause?: Pause | null;
  /**
   * How the word of a rung ran, for a rung alone.
   */
  run?: RunState | null;
  /**
   * The word of a rung, once its chain took it to run or to refuse.
   */
  word?: string | null;
  /**
   * How many facts the life had said when the act was made, which is its place in time.
   */
  began: number;
  /**
   * How many facts the life had said when the act was done.
   */
  ended?: number | null;
  /**
   * What the models cost: the answers that wrote the word of a rung, or each answer on a chain.
   */
  spend?: Spend | null;
  /**
   * The state of the act, or of the work of a chain for a chain.
   */
  state: State;
}
/**
 * A pause, as its fact holds it: who said it, and the cause that it gave.
 */
export interface Pause {
  by: string;
  why?: unknown;
}
/**
 * What the answers of models cost: their dollars, how many they are, and the actor of the last.
 */
export interface Spend {
  dollars: number;
  answers: number;
  actor?: string | null;
}
/**
 * One follow-up that waits on a chain: its entry, which the window that queued it chose, the shape
 * that its prompt wants, its message, and why its send failed, when it did. A follow-up that
 * failed holds the queue of its chain until the operator takes it out.
 */
export interface FollowUp {
  entry: string;
  chain: string;
  shape: string;
  message: string;
  failed?: string | null;
}
/**
 * An act that stays open in a life that stalled: its name, its kind, and the chain that it stands
 * in.
 */
export interface Open {
  act: string;
  kind: string;
  chain: string;
}

export const PROTOCOL = 1;
