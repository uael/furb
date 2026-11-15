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
      config?: string | null;
      verb: "list";
    }
  | {
      config?: string | null;
      verb: "projects";
    }
  | {
      cwd: string;
      verb: "files";
    }
  | {
      cwd: string;
      path?: string | null;
      verb: "changes";
    }
  | {
      cwd?: string | null;
      config?: string | null;
      claude?: string | null;
      verb: "models";
    }
  | {
      verb: "cards";
    }
  | {
      life: string;
      verb: "history";
    }
  | {
      life: string;
      verb: "skills";
    }
  | {
      life: string;
      cwd?: string | null;
      actor?: string | null;
      roster?: string[] | null;
      skills?: boolean | null;
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
      cancel?: boolean;
      verb: "end";
    }
  | {
      life: string;
      verb: "presence";
    }
  | {
      life: string;
      draft?: Draft | null;
      verb: "writes";
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
      act: string;
      verb: "ended";
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
      on?: string | null;
      verb: "names";
    }
  | {
      life: string;
      roster: string[];
      verb: "roster";
    }
  | {
      life: string;
      config: Config;
      verb: "config";
    }
  | {
      life: string;
      verb: "cancel_sign_in";
    }
  | {
      life: string;
      about: string;
      verb: "seen";
    }
  | {
      life: string;
      thread: string;
      verb: "resolve";
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
      read?: number | null;
      verb: "threads";
    }
  | {
      life: string;
      hidden?: string[];
      at?: number | null;
      verb: "rail";
    }
  | {
      life: string;
      on?: string | null;
      at?: number | null;
      opened?: string[];
      owed?: string[];
      taken?: string[] | null;
      shown?: Shown | null;
      verb: "ladder";
    }
  | {
      life: string;
      at: number;
      verb: "as_of";
    }
  | {
      life: string;
      thread: string;
      before?: number | null;
      limit: number;
      verb: "thread";
    }
  | {
      life: string;
      /**
       * The chain whose acts give the moments; none gives those of the whole life.
       */
      on?: string | null;
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
  | {
      viewer: string;
      life: string;
      slot: Slot;
      skill: string;
      verb: "view.mount";
    }
  | {
      viewer: string;
      slot: Slot;
      version: number;
      tree: Node;
      verb: "view.tree";
    }
  | {
      viewer: string;
      node: string;
      kind: Gesture;
      value?: string | null;
      verb: "view.event";
    }
  | {
      viewer: string;
      effect: Effect;
      verb: "view.effect";
    }
  | {
      viewer: string;
      verb: "view.dispose";
    };
/**
 * What a peer is: a window of an operator, the process of a skill, or the daemon of a new build,
 * which asks the handover.
 */
export type Kind = "window" | "skill" | "daemon";
/**
 * What a peer may do: read a life, or read and write it. Write holds each right of read.
 */
export type Role = "read" | "write";
/**
 * What a peer asks of the configs of a life, which the World writes, so no window writes a disk.
 */
export type Config =
  | {
      action: "get";
    }
  | {
      key: string[];
      value?: unknown;
      layer?: Layer | null;
      action: "set";
    };
/**
 * A layer of the configs, from the one that wins least to the one that wins most.
 */
export type Layer = "user" | "project" | "local";
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
      operators: Present[];
      frame: "presence";
    }
  | {
      life: string;
      changed: Card[];
      gone: string[];
      frame: "cards";
    }
  | {
      life: string;
      done: number;
      of: number;
      frame: "indexing";
    }
  | {
      life: string;
      thread: string;
      page: Page;
      frame: "thread";
    }
  | {
      life: string;
      stage: Stage;
      done: number;
      of: number;
      frame: "opening";
    }
  | {
      life: string;
      acts: Open[];
      notice: string;
      frame: "stalled";
    }
  | {
      state: string;
      url?: string | null;
      message?: string | null;
      frame: "sign_in";
    }
  | {
      life: string;
      ask: number;
      request: unknown;
      frame: "ask";
    }
  | {
      life: string;
      skills: string[];
      frame: "served";
    }
  | {
      viewer: string;
      life: string;
      slot: Slot;
      skill: string;
      frame: "view.mount";
    }
  | {
      viewer: string;
      slot: Slot;
      version: number;
      tree: Node;
      frame: "view.tree";
    }
  | {
      viewer: string;
      node: string;
      kind: Gesture;
      value?: string | null;
      frame: "view.event";
    }
  | {
      viewer: string;
      effect: Effect;
      frame: "view.effect";
    }
  | {
      viewer: string;
      frame: "view.dispose";
    };
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
        machine: string;
        build: string;
      };
    }
  | {
      answer: "lives";
      data: Listed[];
    }
  | {
      answer: "projects";
      data: Project[];
    }
  | {
      answer: "models";
      data: Model[];
    }
  | {
      answer: "files";
      data: Files;
    }
  | {
      answer: "changes";
      data: Changes;
    }
  | {
      answer: "cards";
      data: Card[];
    }
  | {
      answer: "history";
      data: Sent[];
    }
  | {
      answer: "skills";
      data: Skill[];
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
      data: Present[];
    }
  | {
      answer: "handed";
      data: Handed[];
    }
  | {
      answer: "desk";
      data: Desk;
    }
  | {
      answer: "threads";
      data: Threads;
    }
  | {
      answer: "rail";
      data: ThreadNode[];
    }
  | {
      answer: "ladder";
      data: Rung[];
    }
  | {
      answer: "acts";
      data: Act[];
    }
  | {
      answer: "moments";
      data: number[];
    }
  | {
      answer: "page";
      data: Page;
    };
/**
 * How a file differs from the last commit.
 */
export type Change = ("added" | "modified" | "deleted" | "renamed") | "untracked";
/**
 * How the thread of a card stands. It asks one member by his login, or each operator. A pause
 * names who said it. A thread whose model closed its prompt is resolved, and one that a cancel
 * ended is stopped.
 */
export type Standing =
  | {
      of?: string | null;
      is: "asks";
    }
  | {
      is: "failed";
    }
  | {
      by: string;
      is: "paused";
    }
  | {
      is: "thinks";
    }
  | {
      is: "runs";
    }
  | {
      is: "delegates";
    }
  | {
      is: "stopped";
    }
  | {
      is: "rests";
    }
  | {
      is: "resolved";
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
 * Who made the prompt of a thread: the operator, a rung of a word, or an ear.
 */
export type Origin = "operator" | "rung" | "ear";
/**
 * What a prompt of a chain is that is no conversation, which the feed of its chain draws as what
 * it is and the list of its threads leaves out.
 */
export type Aside =
  | {
      aside: "notice";
    }
  | {
      aside: "rung";
    }
  | {
      aside: "wake";
    }
  | {
      aside: "question";
      from: string;
    }
  | {
      aside: "agent";
    };
/**
 * One line of a ladder, in the order that the window draws it.
 */
export type Rung =
  | {
      id: string;
      depth: number;
      type: "step";
    }
  | {
      rungs: string[];
      depth: number;
      type: "ear";
    }
  | {
      head: string;
      depth: number;
      count: number;
      of: string;
      key: string;
      heads: string[];
      type: "more";
    };
/**
 * Who made an act, as a window names it: the operator, a member who answers as him by his login,
 * a model with its effort when its actor names one, a skill, or the engine itself.
 */
export type Maker =
  | {
      role: "operator";
    }
  | {
      operator: string;
      role: "member";
    }
  | {
      model: string;
      effort?: string | null;
      role: "model";
    }
  | {
      role: "skill";
    }
  | {
      role: "engine";
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
 * How the word of a rung ran: it runs or waits for its word, it ran, or a fault or a cancel ended
 * it, each with what a person reads.
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
    };
/**
 * How a line stands: it ran, or will not run; it runs, with the work that it waits for and the
 * millisecond of the epoch at which the host saw it begin, so a time of a second shows after a
 * whole second; it asks the operator; its model is asked again after a call that failed; it
 * failed, with the reason that a person reads; or a stop ended it, at the millisecond of the epoch
 * at which the host saw the stop, which a host that opened the life after it does not know.
 */
export type Mark =
  | {
      is: "ran";
    }
  | {
      work: Work;
      began?: number | null;
      is: "runs";
    }
  | {
      is: "asks";
    }
  | (Retrying & {
      is: "retries";
    })
  | {
      why: string;
      is: "failed";
    }
  | {
      at?: number | null;
      is: "stopped";
    };
/**
 * What a line that runs waits for: its model that writes it, a command or a wait, or an agent.
 */
export type Work = "writes" | "command" | "agent";
/**
 * What an act that a line made came to: it works, it failed, or it is done with a value, as a
 * literal of python cut to `LITERAL` characters.
 */
export type Made =
  | {
      made: "works";
    }
  | {
      made: "failed";
    }
  | {
      made: "done";
      value: string;
    };
/**
 * What a said is. A question holds the shape that its answer takes, which `choices`, `ticks` and
 * `hint` of the view read, and what answered it once it is closed: it is open until then.
 */
export type Saying =
  | ("answer" | "notice")
  | {
      question: {
        shape: string;
        answered?: Answered | null;
      };
    };
/**
 * What a question came to: a value, as a person reads it, or a refusal with its reason.
 */
export type Reply =
  | {
      is: "value";
      text: string;
    }
  | {
      is: "refused";
      text: string;
    };
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
      error: "shown";
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
      life: string;
      error: "answering";
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
      event: "retried";
      data: {
        rung: string;
        chain: string;
        retry: Retrying;
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
 * A stage of the open of a life, in the order that an open passes them. An open skips a stage that
 * it has no work for, and a stage never comes again in one open. Each stage counts its work in a
 * unit of its own, and a stage that counts nothing tells 0 of 0.
 */
export type Stage = "recall" | "find" | "cross" | "read" | "seed" | "boot" | "log";

/**
 * Both ways of the protocol, which its schema holds.
 */
export interface Protocol {
  request: Request;
  frame: Frame;
}
/**
 * The first frame of a peer: the protocol that it speaks, its kind, the name of a skill, and the
 * role that it asks. The daemon gives the peer the lesser of that role and the role that its
 * transport proved, so a peer asks `read` to be a reader on any transport. With no role, the peer
 * has the role that its transport proved.
 */
export interface Hello {
  protocol: number;
  kind: Kind;
  name?: string | null;
  role?: Role | null;
}
/**
 * Where a person writes a draft: in a thread, or for a new thread when it names none.
 */
export interface Draft {
  on?: string | null;
}
/**
 * How many of the last prompts of a thread, and of the last rungs of each ladder, show.
 */
export interface Shown {
  prompts: number;
  rungs: number;
}
/**
 * A run of a text, in a tone.
 */
export interface Run {
  text: string;
  tone?: Tone & string;
}
/**
 * A peer as the daemon knows it after its hello: its operator, the login that its transport
 * proved, its role, its kind, and the name of a skill. No window is named apart from its
 * operator, so two windows of one operator are one operator.
 */
export interface Peer {
  operator: string;
  role: Role;
  kind: Kind;
  name?: string | null;
}
/**
 * A record of the machine: the main checkout of the project whose sessions hold it, when the
 * list names one, else the checkout whose `.furb/sessions` holds a record that a life holds,
 * the second of the epoch of the last write of the record, when the disk holds it,
 * whether a life of the daemon holds it now, whether that life lives and does not
 * end, whether a window holds that life, and the name of its session while a life holds it: the
 * one that the operator said last on the root, which wins, else the one that the skill naming
 * gave. An empty name takes the default name again.
 */
export interface Listed {
  life: string;
  project?: string | null;
  used?: number | null;
  held: boolean;
  open: boolean;
  /**
   * Whether a window of any connection holds the life now: a pin holds it with no window.
   */
  watched?: boolean;
  /**
   * The holder of each pin of the life, so a life that only a pin holds stands in the background,
   * and a life with no holder, which ends at its grace, does not.
   */
  pins?: string[];
  name?: string | null;
}
/**
 * A project of the machine: its main checkout, the count of the records of its sessions, and the
 * second of the epoch of the last write of one of them.
 */
export interface Project {
  folder: string;
  sessions: number;
  used?: number | null;
}
/**
 * A model that the catalog of a machine offers: the name that a stand takes, the levels of effort
 * that it takes, the level that a life takes when its actor names none, the tokens of its window,
 * and whether it sees images.
 */
export interface Model {
  name: string;
  efforts: string[];
  effort?: string | null;
  window: number;
  images: boolean;
}
/**
 * The closed set of the nodes of a view, which each window draws in its own way.
 * The files of a folder: the path of each from the folder, in the order of a walk by name, and
 * whether more files stand there than the daemon gives.
 */
export interface Files {
  paths: string[];
  more: boolean;
}
/**
 * The changes of a checkout of git against its last commit: its branch, none when its head names
 * no branch, each file that differs, and the diff of the file that the peer named, as git writes
 * a unified diff, a file that git does not track yet added whole.
 */
export interface Changes {
  branch?: string | null;
  files: Changed[];
  diff?: string | null;
}
/**
 * A file of a checkout that differs from the last commit: its path from the top of the checkout,
 * the lines that it adds and removes, and how it differs.
 */
export interface Changed {
  path: string;
  added: number;
  removed: number;
  state: Change;
}
/**
 * A card of the machine: one thread that an operator started, in the record of its session. It
 * holds the name of the session when it has one, the main checkout of the project whose sessions
 * hold the record, when the list of the machine names one, else the checkout whose
 * `.furb/sessions` holds a record that a life holds, whether a life holds the record now,
 * each operator whose window holds that life, and the second of the epoch since which the thread
 * stands as it does. A record that has no index of its threads yet gives no card until
 * the daemon made its index.
 */
export interface Card {
  life: string;
  session?: string | null;
  project?: string | null;
  /**
   * Why no life of the machine opens the record of the card, whose thread then only reads.
   */
  refused?: string | null;
  held: boolean;
  viewers: string[];
  /**
   * The holder of each pin of the life, as `list` gives it.
   */
  pins: string[];
  since: number;
  prompt: string;
  title: string;
  standing: Standing;
  model?: string | null;
  agents?: number;
  readers?: string[];
}
/**
 * What an operator sent to a life: a message, or a word of Python, with the site that sent it,
 * `operator` or `operator/<login>`.
 */
export interface Sent {
  by: string;
  text: string;
  python?: boolean;
}
/**
 * A skill that the root of a life was told of, by the last read of `skills://` there: its name,
 * and what it is for.
 */
export interface Skill {
  name: string;
  description: string;
}
/**
 * An operator whose window holds a life now: the operator, the greatest role of his windows,
 * and the draft that he writes, when one of his windows writes one.
 */
export interface Present {
  operator: string;
  role: Role;
  writes?: Draft | null;
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
 * What waits for the operator in a life: each ask, the oldest first, and how many of them are
 * questions and notices.
 */
export interface Desk {
  asks: Ask[];
  questions: number;
  notices: number;
}
/**
 * A prompt to the operator that waits for him. A question waits for an answer of its shape. A
 * notice is a prompt of None, which the console closed as it took it, and which waits only until
 * the operator dismisses it.
 */
export interface Ask {
  id: string;
  /**
   * The chain that the prompt stands in.
   */
  chain: string;
  /**
   * Who made the prompt: a rung, an ear, or the operator.
   */
  by: string;
  shape: string;
  message: string;
  notice: boolean;
  /**
   * The lines that answer a choice, one for each value that the shape allows.
   */
  choices: string[];
  /**
   * The values of a list of a Literal, which the operator ticks.
   */
  ticks: Tick[];
  /**
   * What the line of the operator is, in words, when its shape is more than text or a choice.
   */
  hint: string;
  began: number;
}
/**
 * A value of a list of a Literal: as the operator reads it, and as the line of the list writes it.
 */
export interface Tick {
  shown: string;
  json: string;
}
/**
 * The threads of one chain as a window draws them: the tree of its threads, its asides, the runs
 * of its notes, what the tree says to the operator, and the state of the chain.
 */
export interface Threads {
  threads: ThreadNode[];
  asides: ChainAside[];
  notes: NoteRun[];
  attention: Attention;
  state: State;
}
/**
 * A thread with the threads that its ladder made, its agents. Its state is its own, or asks while
 * a question waits in an agent under it: a failure or an unread in an agent does not roll up.
 */
export interface ThreadNode {
  thread: ChainThread;
  agents: ThreadNode[];
  state: State;
}
/**
 * One thread of a chain: its prompt, its name, its messages, the prompts to the operator that it
 * made and the questions of them that wait, and its state. A thread is over once its prompt
 * and each of its messages is done, and a message to a thread that is over opens it again until
 * its model closes the message.
 */
export interface ChainThread {
  prompt: string;
  /**
   * The name of the thread: the label of its chain for the first thread that no operator made on
   * a chain under the root, the name of the session for the first thread of the operator on the
   * root, and else the first line of its message that is not empty.
   */
  title: string;
  messages: string[];
  asked: string[];
  asks: string[];
  state: State;
  over: boolean;
  origin: Origin;
  /**
   * Whether the operator started it, himself or by a word of his own.
   */
  yours: boolean;
}
/**
 * A prompt of a chain that is no thread, with what it is and the prompts on it, in order.
 */
export interface ChainAside {
  prompt: string;
  aside: Aside;
  messages: string[];
}
/**
 * A run of notes: the asides of a chain that are no question and follow one another in its feed
 * with no card of another kind between them, which a window draws as one block. `unread` counts
 * the notes that began or ended after the place up to which the operator read the chain.
 */
export interface NoteRun {
  prompts: string[];
  count: number;
  unread: number;
}
/**
 * What a tree of threads says to the operator, counted by thread.
 */
export interface Attention {
  asks: number;
  failed: number;
  paused: number;
  unread: number;
}
/**
 * One act of a life with its meaning, as it stands after the last fact about it.
 */
export interface Act {
  kind: string;
  id: string;
  by: string;
  /**
   * Who made the act: for a rung that waits for the word of an actor, that actor; for any
   * other act, the role of the act or the person that made it.
   */
  maker?: Maker;
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
   * Whether a reply of a rung runs, while its model writes its word. A rung with no word and no
   * reply that runs waits for its turn.
   */
  asking?: boolean;
  /**
   * The messages of the operator that a chain with a source took from it, in their order: each
   * prompt of the operator that the prefix of the chain carries, with the id it has on the source.
   */
  taken?: string[];
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
   * Who closed the act, as a window names who made an act, once a close of it was said.
   */
  closer?: Maker | null;
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
 * What the answers of models cost: their dollars, how many they are, and the actor of the last
 * and what it read.
 */
export interface Spend {
  dollars: number;
  answers: number;
  actor?: string | null;
  /**
   * The account of the last answer, none when that answer named none.
   */
  account?: string | null;
  /**
   * What the last answer read.
   */
  read?: Read | null;
}
/**
 * What an answer of a model read: the tokens of its whole prompt, and of them those that it read
 * from the cache and those that it wrote to it.
 */
export interface Read {
  tokens: number;
  cached: number;
  written: number;
}
/**
 * Some exchanges of a thread, the oldest first, with how many exchanges stand before them, the
 * agents of the whole thread, in the order the thread made them, and the context of its chain.
 */
export interface Page {
  exchanges: Exchange[];
  above: number;
  agents: Agent[];
  context?: Context | null;
}
/**
 * One message of a thread with all that answers it: its place in the thread, 0 for the prompt of
 * the thread, the act of the message, who said it and its text, whether it is a word of Python
 * that an operator ran on the thread, whose block holds its code and which has no text, whether
 * the message still runs, the block of code that answers it once a model answers, and what the
 * answer says to the operator, in the order it was said. A message that the operator said on a
 * thread that works is done at once, since its model reads it at its next step, so only one that
 * asks still runs. A message is the prompt of the thread, what an operator said on it, or a word
 * that he ran on it, and what answers it is what came after it and before the next message.
 */
export interface Exchange {
  at: number;
  message: string;
  by: Maker;
  text: string;
  python?: boolean;
  runs?: boolean;
  block?: Block | null;
  said: Said[];
}
/**
 * The code of one answer: the model that wrote its last word, each line of its words, whether more
 * comes since the answer works, and the thought of its model.
 */
export interface Block {
  maker: Maker;
  lines: Line[];
  more: boolean;
  thought?: Thought | null;
}
/**
 * One line of a block: its number in the thread, its text, in which each quote stands as the
 * binding that it makes and a string of many lines as one line with its escapes, its mark, what
 * the last command that it awaited wrote, and what each act that it made came to, in the order it
 * made them: one for each pass of a loop, the last of which is the value that it bound.
 */
export interface Line {
  number: number;
  text: string;
  mark: Mark;
  output?: Output | null;
  made?: Made[];
}
/**
 * A call of a model that failed for a cause that passes, so its provider asks the model again:
 * the attempt that comes, the most attempts of the call, the seconds that the provider waits
 * before that attempt and the second of the epoch at which it comes, and why the attempt before
 * it failed. What the model wrote before is void.
 */
export interface Retrying {
  attempt: number;
  most: number;
  wait: number;
  at: number;
  why: string;
}
/**
 * What a command that a line awaited wrote, as it writes it and once it ended: its last lines,
 * those of its stdout then those of its stderr, each as a terminal shows it, how many lines came
 * before them, and the seconds that it took once it ended, counted from milliseconds, which a
 * host that opened the life after the command began does not know.
 */
export interface Output {
  tail: string[];
  more: number;
  took?: number | null;
}
/**
 * What a model thought before it wrote: the millisecond of the epoch at which it began to think,
 * as the host saw it begin, and in how many seconds it thought, which is none while it thinks.
 */
export interface Thought {
  text: string;
  began: number;
  took?: number | null;
}
/**
 * What an answer says to the operator: the value of the close of its message, a notice, or a
 * question, with its act and its text.
 */
export interface Said {
  act: string;
  kind: Saying;
  text: string;
}
/**
 * What closed a question: who closed it, and what it came to.
 */
export interface Answered {
  by: Maker;
  reply: Reply;
}
/**
 * An agent of a thread: the thread that a word of the thread started on another chain, as a card,
 * with the millisecond of the epoch at which the host saw it begin, the seconds that it took once
 * it ended, counted from milliseconds as the time of a thought, and the line of its last word
 * where it stands: the line that runs or asks, else the last one, and the number in the thread
 * of the line of the word that made it, none when no line of a word of the thread made it. A host
 * that opened the life after the agent began saw it begin at that open.
 */
export interface Agent {
  prompt: string;
  title: string;
  standing: Standing;
  model?: string | null;
  agents?: number;
  readers?: string[];
  began?: number | null;
  took?: number | null;
  line?: Line | null;
  at?: number | null;
}
/**
 * The context of a chain at its last answer: what that answer read, the window that the roster of
 * the standing offers its actor, none when the roster holds no such actor, and the dollars of each
 * answer on the chain.
 */
export interface Context {
  read: Read;
  window?: number | null;
  dollars: number;
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

export type Mount = Omit<Extract<Request1, { verb: "view.mount" }>, "verb">;
export type Tree = Omit<Extract<Request1, { verb: "view.tree" }>, "verb">;
export type ViewEvent = Omit<Extract<Request1, { verb: "view.event" }>, "verb">;
export type ViewEffect = Omit<Extract<Request1, { verb: "view.effect" }>, "verb">;
export type Dispose = Omit<Extract<Request1, { verb: "view.dispose" }>, "verb">;

export const PROTOCOL = 47;
