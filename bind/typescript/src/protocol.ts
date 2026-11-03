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
      verb: "cancel";
    }
  | {
      life: string;
      label?: string | null;
      on?: string | null;
      verb: "chain";
    }
  | {
      life: string;
      word: string;
      on?: string | null;
      verb: "run";
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
 * What a peer is: a window of an operator, or the process of a skill.
 */
export type Kind = "window" | "skill";
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
      node: "input";
    }
  | {
      label: string;
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
      event: "notice";
      data: string;
    }
  | {
      event: "changed";
      data: string;
    }
  | {
      event: "ended";
      data: string | null;
    };
/**
 * Where an act stands: asked, started by the ear that owns it, or done, with what it came to.
 */
export type State =
  | {
      state: "asked";
    }
  | {
      state: "started";
    }
  | {
      state: "done";
      value: unknown;
    };

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
 * One act of a life: its kind, its name, who made it, what it is on, the chain it is in, its words
 * after these, and its state.
 */
export interface Act {
  kind: string;
  id: string;
  by: string;
  on: string;
  chain: string;
  words: unknown[];
  state: State;
}

export const PROTOCOL = 1;
