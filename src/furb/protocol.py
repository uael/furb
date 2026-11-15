"""The types of the one protocol of the host, made from host/protocol.schema.json by xtask/python/protocol.py.

Run `cargo xtask protocol` after a change of the protocol.
"""

from typing import Literal, NotRequired, TypedDict


class Request2(TypedDict):
  """
  The records of the machine: each record of the sessions of each project that
  `workspaces.json` in the folder `config` names, the config folder of the daemon when the peer
  names none, as a window keeps its list there, and each record that a life of the daemon holds.
  """

  id: int
  config: NotRequired[str | None]
  verb: Literal["list"]


class Request3(TypedDict):
  """
  The projects of the records that `list` gives with the same config, as [`Project::of`] folds
  them.
  """

  id: int
  config: NotRequired[str | None]
  verb: Literal["projects"]


class Request4(TypedDict):
  """
  The files of the folder `cwd` and of each folder in it, which a mention of the input names:
  dot files too, less what an ignore file of git leaves out, the folder `.git` and the records
  of the sessions of furb, `.furb/sessions`. A `cwd` that starts with `~` names a folder of the
  home of the user of the machine.
  """

  id: int
  cwd: str
  verb: Literal["files"]


class Request5(TypedDict):
  """
  The changes of the checkout of git that holds the folder `cwd`, against its last commit: its
  branch, each file that differs, by its path from the top of the checkout, with the lines that
  it adds and removes, and the diff of the file at `path` when the peer names one. A folder in
  no checkout has no branch and no change.
  """

  id: int
  cwd: str
  path: NotRequired[str | None]
  verb: Literal["changes"]


class Request6(TypedDict):
  """
  The models that the catalog of the machine offers to a life in the folder `cwd`, with the
  settings of the folder `config` and the claude command line at `claude`, each as `open`
  takes it, with no life: the ones of the daemon when the peer names none.
  """

  id: int
  cwd: NotRequired[str | None]
  config: NotRequired[str | None]
  claude: NotRequired[str | None]
  verb: Literal["models"]


class Request7(TypedDict):
  """
  The cards of the machine: each thread that an operator started in each record that `list`
  gives with no config. The peer then hears a frame `cards` at each change of them, until it
  leaves.
  """

  id: int
  verb: Literal["cards"]


class Request8(TypedDict):
  """
  What the peer sent in the project of a record, the newest first and each text once: what it
  sent to that record, then to each other record of the project, the record that changed last
  first. A record whose index does not agree with it gives nothing until the daemon made its
  index.
  """

  id: int
  life: str
  verb: Literal["history"]


class Request9(TypedDict):
  """
  The skills that the root of a life read last, by a read of `skills://`, in the order of their
  names: those of the record, else those of each other record of its project, the record that
  changed last first, so a new session offers the skills of the last session of its project.
  A record whose index does not agree with it gives none until the daemon made its index.
  """

  id: int
  life: str
  verb: Literal["skills"]


class Request10(TypedDict):
  """
  The life of a record opened, in a folder or in the folder of its record. A life that this
  open makes stands on the actor, the roster and the skills that the peer names, reads the
  settings of the user in the folder `config` and runs the claude command line `claude` when
  the peer names them, as the window of the peer reads them, and the peer answers each request
  of its models in their place when it asks to, as a demo does.
  With `attach`, the peer attaches the life in the same step, from that place in its log, so
  the life has a holder before its grace can pass.
  """

  id: int
  life: str
  cwd: NotRequired[str | None]
  actor: NotRequired[str | None]
  roster: NotRequired[list[str] | None]
  skills: NotRequired[bool | None]
  config: NotRequired[str | None]
  claude: NotRequired[str | None]
  answers: NotRequired[bool]
  attach: NotRequired[int | None]
  verb: Literal["open"]


Request11 = TypedDict("Request11", {"id": int, "life": str, "from": int, "verb": Literal["attach"]})


class Request12(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  verb: Literal["detach"]


class Request13(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  holder: str
  verb: Literal["pin"]


class Request14(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  holder: str
  verb: Literal["unpin"]


class Request15(TypedDict):
  """
  The end of a life now, whatever pins hold it, as the grace ends a life with no holder: it
  writes its image, each of its windows hears its end, and its record opens again at its tip,
  where the life takes its open work again, so an end is a pause. With `cancel`, the end is a
  kill: the operator first cancels each open work of the life, so the next life takes none.
  It is refused while a window of another peer holds the life, so no view ends a life that
  another view shows.
  """

  id: int
  life: str
  cancel: NotRequired[bool]
  verb: Literal["end"]


class Request16(TypedDict):
  """
  Each operator whose window holds a life now.
  """

  id: int
  life: str
  verb: Literal["presence"]


class Request18(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  act: str
  verb: Literal["get"]


class Request19(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  act: str
  verb: Literal["peek"]


class Request20(TypedDict):
  """
  An act as the life folds it once it is done: at once when it is done, else when its done
  comes. The window that ran a word asks it of the rung of that word, and asks it again after a
  reconnect, since the record keeps the end.
  """

  id: int
  life: str
  act: str
  verb: Literal["ended"]


class Request21(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  of: NotRequired[str | None]
  verb: Literal["transcript"]


class Request22(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  verb: Literal["turns"]


class Request23(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  verb: Literal["program"]


class Request24(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  verb: Literal["standing"]


class Request25(TypedDict):
  """
  The share of the window of its model that the prompt of an answer filled, as `filled` of
  the engine gives it from the reply that took that answer, and nothing before an answer.
  """

  id: int
  life: str
  act: str
  verb: Literal["filled"]


class Request26(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  verb: Literal["cwd"]


class Request27(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  name: str
  on: NotRequired[str | None]
  verb: Literal["inspect"]


class Request28(TypedDict):
  """
  The prompts of a life that wait for an operator.
  """

  id: int
  life: str
  verb: Literal["pending"]


class Request29(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  shape: str
  message: str
  to: NotRequired[str | None]
  on: NotRequired[str | None]
  verb: Literal["prompt"]


class Request30(TypedDict):
  """
  The close of a prompt to the operator, with a line that the operator wrote.
  """

  id: int
  life: str
  act: str
  line: str
  verb: Literal["answer"]


class Request31(TypedDict):
  """
  A pause of an act or of a chain, with the cause that the operator gives.
  """

  id: int
  life: str
  act: str
  why: NotRequired[str | None]
  verb: Literal["pause"]


class Request32(TypedDict):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  life: str
  act: str
  verb: Literal["wake"]


class Request33(TypedDict):
  """
  A cancel of an act or of a chain, with the cause that the operator gives.
  """

  id: int
  life: str
  act: str
  why: NotRequired[str | None]
  verb: Literal["cancel"]


class Request34(TypedDict):
  """
  A word that runs on a chain as a turn of the operator: each act of a person goes this way, as
  the word that `Door` gives, so the transcript shows it as it shows the act of a model.
  """

  id: int
  life: str
  word: str
  on: NotRequired[str | None]
  verb: Literal["run"]


class Request35(TypedDict):
  """
  A close of an act with a value, written as the record writes a value: `2.0` stays a float,
  and `{"is": "Refused", "args": ["why"]}` is a refusal.
  """

  id: int
  life: str
  act: str
  value: str
  verb: Literal["close"]


class Request36(TypedDict):
  """
  The names that the module of a chain binds.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  verb: Literal["names"]


class Request37(TypedDict):
  """
  The models that a life offers, in place of the roster of the life, which no word can say,
  since the roster is the World's: the life then stands on them, as after a change of its
  configs, and each later birth of it keeps them.
  """

  id: int
  life: str
  roster: list[str]
  verb: Literal["roster"]


class Request39(TypedDict):
  """
  A cancel of the sign-in of codex that the daemon runs for the life, which refuses the config
  that asked it. It writes no config.
  """

  id: int
  life: str
  verb: Literal["cancel_sign_in"]


class Request40(TypedDict):
  """
  The fact that the operator read a chain or a thread up to now, which each window and each
  later life reads in the record.
  """

  id: int
  life: str
  about: str
  verb: Literal["seen"]


class Request41(TypedDict):
  """
  The resolve of a thread of the operator: a close of each act that keeps it open, the last
  first, with the text `resolved` or with None, as its shape takes. A message on the thread
  opens it again.
  """

  id: int
  life: str
  thread: str
  verb: Literal["resolve"]


class Request42(TypedDict):
  """
  A follow-up that the operator queued on a chain, under an entry that the window chose. The
  life sends it as a prompt of its shape and its message once the chain rests, and a second
  queue of the same entry, from any window, changes nothing.
  """

  id: int
  life: str
  entry: str
  shape: str
  message: str
  on: NotRequired[str | None]
  verb: Literal["queue"]


class Request43(TypedDict):
  """
  A follow-up taken out of its queue before the life sent it.
  """

  id: int
  life: str
  entry: str
  verb: Literal["unqueue"]


class Request44(TypedDict):
  """
  The follow-ups that wait on a chain, in order.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  verb: Literal["queued"]


class Request45(TypedDict):
  """
  What waits for the operator in a life, with each notice that the window still shows.
  """

  id: int
  life: str
  notices: NotRequired[list[str]]
  verb: Literal["desk"]


class Request46(TypedDict):
  """
  The threads of a chain with their meaning, now or as they stood just after a moment. `read`
  is the place up to which the window read the chain, from which the runs of notes count the
  ones not yet read; none reads nothing.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  at: NotRequired[int | None]
  read: NotRequired[int | None]
  verb: Literal["threads"]


class Request47(TypedDict):
  """
  The threads of every chain of a life but those it names, as one tree in the order of the
  list of the threads, now or as they stood just after a moment.
  """

  id: int
  life: str
  hidden: NotRequired[list[str]]
  at: NotRequired[int | None]
  verb: Literal["rail"]


class Request49(TypedDict):
  """
  The acts of a life as they stood just after a moment, each with its meaning then.
  """

  id: int
  life: str
  at: int
  verb: Literal["as_of"]


class Request50(TypedDict):
  """
  A page of a thread of a life, its prompt: the exchanges before the one at `before`, or
  before the end, at most `limit` of them, the newest page first. The page of the end also
  makes the peer hear a frame `thread` at each change of that thread, until it reads the end
  of another thread or leaves: a peer reads one thread at a time.
  """

  id: int
  life: str
  thread: str
  before: NotRequired[int | None]
  limit: int
  verb: Literal["thread"]


class Request51(TypedDict):
  """
  Each moment of a life at which an act began or ended, in order.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  """
    The chain whose acts give the moments; none gives those of the whole life.
    """
  verb: Literal["moments"]


class Request52(TypedDict):
  """
  The turn that the peer gave for a request of a model of a life that it answers, written as
  the record writes a value, or why it gives none.
  """

  id: int
  life: str
  ask: int
  turn: NotRequired[object]
  why: NotRequired[str | None]
  verb: Literal["answered"]


class Request53(TypedDict):
  """
  What the peer added to the text and to the thought of the turn of a request, as it writes.
  """

  id: int
  life: str
  ask: int
  text: str
  thinking: str
  verb: Literal["wrote"]


class Request54(TypedDict):
  """
  The place of the daemon given to the daemon of a new build: each life written to its
  checkpoint and ended, and the lives with their pins in the answer, once the daemon stopped to
  listen.
  """

  id: int
  verb: Literal["handover"]


class Request59(TypedDict):
  """
  The end of a view in its window.
  """

  id: int
  viewer: str
  verb: Literal["view.dispose"]


type Kind = Literal["window", "skill", "daemon"]
"""
What a peer is: a window of an operator, the process of a skill, or the daemon of a new build,
which asks the handover.
"""


type Role = Literal["read", "write"]
"""
What a peer may do: read a life, or read and write it. Write holds each right of read.
"""


class Hello(TypedDict):
  """
  The first frame of a peer: the protocol that it speaks, its kind, the name of a skill, and the
  role that it asks. The daemon gives the peer the lesser of that role and the role that its
  transport proved, so a peer asks `read` to be a reader on any transport. With no role, the peer
  has the role that its transport proved.
  """

  protocol: int
  kind: Kind
  name: NotRequired[str | None]
  role: NotRequired[Role | None]


class Draft(TypedDict):
  """
  Where a person writes a draft: in a thread, or for a new thread when it names none.
  """

  on: NotRequired[str | None]


class Config1(TypedDict):
  """
  Each key of the configs with the value that wins for it and its layer.
  """

  action: Literal["get"]


type Layer = Literal["user", "project", "local"]
"""
A layer of the configs, from the one that wins least to the one that wins most.
"""


class Shown(TypedDict):
  """
  How many of the last prompts of a thread, and of the last rungs of each ladder, show.
  """

  prompts: int
  rungs: int


class Slot1(TypedDict):
  """
  Where a window shows a view. The dock follows the chain that it names, and the row, the detail
  and the mark follow the chain of the act that they name. The sidebar, the footer, a screen and
  an overlay follow the life.
  """

  on: str
  slot: Literal["dock"]


class Slot2(TypedDict):
  """
  A section of the sidebar.
  """

  slot: Literal["sidebar"]


class Slot3(TypedDict):
  """
  The row of one act, of a kind that the skill draws.
  """

  act: str
  slot: Literal["row"]


class Slot4(TypedDict):
  """
  The detail of one act, of a kind that the skill draws.
  """

  act: str
  slot: Literal["detail"]


class Slot5(TypedDict):
  """
  A mark in the feed, after one act, of a kind that the skill marks.
  """

  act: str
  slot: Literal["mark"]


class Slot6(TypedDict):
  """
  Where a window shows a view. The dock follows the chain that it names, and the row, the detail
  and the mark follow the chain of the act that they name. The sidebar, the footer, a screen and
  an overlay follow the life.
  """

  slot: Literal["footer"]


class Slot7(TypedDict):
  """
  Where a window shows a view. The dock follows the chain that it names, and the row, the detail
  and the mark follow the chain of the act that they name. The sidebar, the footer, a screen and
  an overlay follow the life.
  """

  slot: Literal["screen"]


class Slot8(TypedDict):
  """
  Where a window shows a view. The dock follows the chain that it names, and the row, the detail
  and the mark follow the chain of the act that they name. The sidebar, the footer, a screen and
  an overlay follow the life.
  """

  slot: Literal["overlay"]


type Slot = Slot1 | Slot2 | Slot3 | Slot4 | Slot5 | Slot6 | Slot7 | Slot8
"""
Where a window shows a view. The dock follows the chain that it names, and the row, the detail
and the mark follow the chain of the act that they name. The sidebar, the footer, a screen and
an overlay follow the life.
"""


class Node5(TypedDict):
  """
  One node of a view, under an id that an event names.
  """

  id: str
  value: str
  hint: NotRequired[str | None]
  inert: NotRequired[bool]
  """
    A node that acts is inert in the window of a peer that only reads: it draws and takes no
    gesture.
    """
  node: Literal["input"]


class Node6(TypedDict):
  """
  One node of a view, under an id that an event names.
  """

  id: str
  label: str
  inert: NotRequired[bool]
  node: Literal["button"]


class Node7(TypedDict):
  """
  The share of the work that is done, from 0 to 1, or none while it is not known.
  """

  id: str
  done: NotRequired[float | None]
  node: Literal["progress"]


class Node8(TypedDict):
  """
  One node of a view, under an id that an event names.
  """

  id: str
  text: str
  language: NotRequired[str | None]
  node: Literal["code"]


class Node9(TypedDict):
  """
  A diff in the unified form.
  """

  id: str
  text: str
  node: Literal["diff"]


class Node10(TypedDict):
  """
  One node of a view, under an id that an event names.
  """

  id: str
  text: str
  node: Literal["markdown"]


type Axis = Literal["row", "column"]
"""
The way a box lays out its children.
"""


type Tone = Literal["text", "muted", "faint", "accent", "success", "warning", "danger"]
"""
The tone of a run, which each window gives its own color.
"""


type Gesture = Literal["press", "change", "submit"]
"""
What a person does to a node.
"""


class Effect1(TypedDict):
  """
  What a skill asks of a window.
  """

  url: str
  effect: Literal["open"]


class Effect2(TypedDict):
  """
  What a skill asks of a window.
  """

  text: str
  effect: Literal["draft"]


class Effect3(TypedDict):
  """
  What a skill asks of a window.
  """

  text: str
  effect: Literal["notice"]


type Effect = Effect1 | Effect2 | Effect3
"""
What a skill asks of a window.
"""


class Frame6(TypedDict):
  """
  How far the daemon stands in the index of the cards of a record that no life holds, pushed to
  each peer that asked the cards while the daemon reads the record: the thousandths of the read
  done, of 1000, which never go back. The frame with `done` equal to `of` comes before the frame
  `cards` of the record.
  """

  life: str
  done: int
  of: int
  frame: Literal["indexing"]


class Frame10(TypedDict):
  """
  The state of the sign-in of codex that this peer asked with a config of its provider, pushed
  at each change of the state, so no window asks it again: the word of the state, the browser
  address while it waits, and what to tell the operator when it needs him or failed.
  """

  state: str
  url: NotRequired[str | None]
  message: NotRequired[str | None]
  frame: Literal["sign_in"]


class Frame11(TypedDict):
  """
  A request of a model of a life that the peer answers: its number, and the request with the
  actor, the chain, the messages and the settings.
  """

  life: str
  ask: int
  request: object
  frame: Literal["ask"]


class Frame12(TypedDict):
  """
  The skills attached to a life, each by its name, which a window mounts a view of: pushed to
  each window of the life when it attaches, and after each attach and each end of a skill.
  """

  life: str
  skills: list[str]
  frame: Literal["served"]


class Frame13(TypedDict):
  """
  A view that a window mounts in a slot of a life, which the process of a skill serves.
  """

  viewer: str
  life: str
  slot: Slot
  skill: str
  frame: Literal["view.mount"]


class Frame15(TypedDict):
  """
  What a person did to a node of a view, with the value of an input.
  """

  viewer: str
  node: str
  kind: Gesture
  value: NotRequired[str | None]
  frame: Literal["view.event"]


class Frame16(TypedDict):
  """
  What a skill asks of the one window of a view.
  """

  viewer: str
  effect: Effect
  frame: Literal["view.effect"]


class Frame17(TypedDict):
  """
  The end of a view in its window.
  """

  viewer: str
  frame: Literal["view.dispose"]


class Answer1(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["done"]


class Answer11(TypedDict):
  """
  The name of the act that the verb made.
  """

  answer: Literal["act"]
  data: str


class Answer12(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["value"]
  data: object


class Answer13(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["values"]
  data: list[object]


class Data1(TypedDict):
  kind: str
  repr: str
  value: object


class Answer14(TypedDict):
  """
  A name of a module: the name of its type, its representation, and its value as JSON.
  """

  answer: Literal["inspected"]
  data: Data1


class Answer22(TypedDict):
  """
  Each moment at which an act began or ended.
  """

  answer: Literal["moments"]
  data: list[int]


class Peer(TypedDict):
  """
  A peer as the daemon knows it after its hello: its operator, the login that its transport
  proved, its role, its kind, and the name of a skill. No window is named apart from its
  operator, so two windows of one operator are one operator.
  """

  operator: str
  role: Role
  kind: Kind
  name: NotRequired[str | None]


class Listed(TypedDict):
  """
  A record of the machine: the main checkout of the project whose sessions hold it, when the
  list names one, else the checkout whose `.furb/sessions` holds a record that a life holds,
  the second of the epoch of the last write of the record, when the disk holds it,
  whether a life of the daemon holds it now, whether that life lives and does not
  end, whether a window holds that life, and the name of its session while a life holds it: the
  one that the operator said last on the root, which wins, else the one that the skill naming
  gave. An empty name takes the default name again.
  """

  life: str
  project: NotRequired[str | None]
  used: NotRequired[int | None]
  held: bool
  open: bool
  watched: NotRequired[bool]
  """
    Whether a window of any connection holds the life now: a pin holds it with no window.
    """
  pins: NotRequired[list[str]]
  """
    The holder of each pin of the life, so a life that only a pin holds stands in the background,
    and a life with no holder, which ends at its grace, does not.
    """
  name: NotRequired[str | None]


class Project(TypedDict):
  """
  A project of the machine: its main checkout, the count of the records of its sessions, and the
  second of the epoch of the last write of one of them.
  """

  folder: str
  sessions: int
  used: NotRequired[int | None]


class Model(TypedDict):
  """
  A model that the catalog of a machine offers: the name that a stand takes, the levels of effort
  that it takes, the level that a life takes when its actor names none, the tokens of its window,
  and whether it sees images.
  """

  name: str
  efforts: list[str]
  effort: NotRequired[str | None]
  window: int
  images: bool


class Files(TypedDict):
  """
  The closed set of the nodes of a view, which each window draws in its own way.
  The files of a folder: the path of each from the folder, in the order of a walk by name, and
  whether more files stand there than the daemon gives.
  """

  paths: list[str]
  more: bool


type Change = Literal["added", "modified", "deleted", "renamed"] | Literal["untracked"]
"""
How a file differs from the last commit.
"""


Standing1 = TypedDict("Standing1", {"of": NotRequired[str | None], "is": Literal["asks"]})


Standing2 = TypedDict("Standing2", {"is": Literal["failed"]})


Standing3 = TypedDict("Standing3", {"by": str, "is": Literal["paused"]})


Standing4 = TypedDict("Standing4", {"is": Literal["thinks"]})


Standing5 = TypedDict("Standing5", {"is": Literal["runs"]})


Standing6 = TypedDict("Standing6", {"is": Literal["delegates"]})


Standing7 = TypedDict("Standing7", {"is": Literal["stopped"]})


Standing8 = TypedDict("Standing8", {"is": Literal["rests"]})


Standing9 = TypedDict("Standing9", {"is": Literal["resolved"]})


type Standing = (
  Standing1 | Standing2 | Standing3 | Standing4 | Standing5 | Standing6 | Standing7 | Standing8 | Standing9
)
"""
How the thread of a card stands. It asks one member by his login, or each operator. A pause
names who said it. A thread whose model closed its prompt is resolved, and one that a cancel
ended is stopped.
"""


class Sent(TypedDict):
  """
  What an operator sent to a life: a message, or a word of Python, with the site that sent it,
  `operator` or `operator/<login>`.
  """

  by: str
  text: str
  python: NotRequired[bool]


class Skill(TypedDict):
  """
  A skill that the root of a life was told of, by the last read of `skills://` there: its name,
  and what it is for.
  """

  name: str
  description: str


class Present(TypedDict):
  """
  An operator whose window holds a life now: the operator, the greatest role of his windows,
  and the draft that he writes, when one of his windows writes one.
  """

  operator: str
  role: Role
  writes: NotRequired[Draft | None]


class Handed(TypedDict):
  """
  A life that a daemon gave at its handover: its record, its folder, and the holder of each pin.
  """

  life: str
  cwd: str
  pins: list[str]


class Tick(TypedDict):
  """
  A value of a list of a Literal: as the operator reads it, and as the line of the list writes it.
  """

  shown: str
  json: str


type State = Literal["asks", "unread", "failed", "paused", "thinks", "runs", "delegates", "done", "cancelled", "idle"]
"""
The state of work: it asks the operator, it ended while the operator looked away, it failed, a
pause holds it, its model thinks or writes its word, a word or a command of it runs, its word
awaits a prompt on another chain, it ended well, a cancel ended it, or it rests. The order is
the order of the roll-up. Only a window says unread, since only it knows what its operator saw.
"""


type Origin = Literal["operator", "rung", "ear"]
"""
Who made the prompt of a thread: the operator, a rung of a word, or an ear.
"""


class Aside1(TypedDict):
  """
  A notice of the round: a prompt of None that a rung or a chain made, which no one answers, as
  the acknowledgment of an act that stands in no thread of the chain, a stall that the watch of
  the daemon tells, or the question of the act that names the session.
  """

  aside: Literal["notice"]


class Aside2(TypedDict):
  """
  What a word of the operator told the chain: a prompt of None that a rung of his made, or the
  acknowledgment of a rung that stands in no thread.
  """

  aside: Literal["rung"]


class Aside3(TypedDict):
  """
  A wake of the chain: a prompt of None that an ear or a verb of the round made.
  """

  aside: Literal["wake"]


Aside4 = TypedDict("Aside4", {"aside": Literal["question"], "from": str})


class Aside5(TypedDict):
  """
  The work of an agent: a prompt that a rung made on a chain under its own, when that rung
  stands in no thread, as a command of the operator runs its word. It is no thread of the
  operator and never takes an unread mark of his.
  """

  aside: Literal["agent"]


type Aside = Aside1 | Aside2 | Aside3 | Aside4 | Aside5
"""
What a prompt of a chain is that is no conversation, which the feed of its chain draws as what
it is and the list of its threads leaves out.
"""


class NoteRun(TypedDict):
  """
  A run of notes: the asides of a chain that are no question and follow one another in its feed
  with no card of another kind between them, which a window draws as one block. `unread` counts
  the notes that began or ended after the place up to which the operator read the chain.
  """

  prompts: list[str]
  count: int
  unread: int


class Attention(TypedDict):
  """
  What a tree of threads says to the operator, counted by thread.
  """

  asks: int
  failed: int
  paused: int
  unread: int


class Rung1(TypedDict):
  """
  A step, with the steps under it after it, each one level deeper.
  """

  id: str
  depth: int
  type: Literal["step"]


class Rung2(TypedDict):
  """
  The rungs in a row that one maker wrote on the chain with no word of its own, as the words of
  the skills and the setup of a chain.
  """

  rungs: list[str]
  depth: int
  type: Literal["ear"]


class Rung3(TypedDict):
  """
  A fold of `count` earlier steps of a level, from the step `head`, with the kinds of the steps
  that it holds in the order they first come. `key` opens it: prompts: or rungs: and the act
  that the steps stand under, or the chain at the top.
  """

  head: str
  depth: int
  count: int
  of: str
  key: str
  heads: list[str]
  type: Literal["more"]


type Rung = Rung1 | Rung2 | Rung3
"""
One line of a ladder, in the order that the window draws it.
"""


class Maker1(TypedDict):
  """
  Who made an act, as a window names it: the operator, a member who answers as him by his login,
  a model with its effort when its actor names one, a skill, or the engine itself.
  """

  role: Literal["operator"]


class Maker2(TypedDict):
  """
  Who made an act, as a window names it: the operator, a member who answers as him by his login,
  a model with its effort when its actor names one, a skill, or the engine itself.
  """

  operator: str
  role: Literal["member"]


class Maker3(TypedDict):
  """
  Who made an act, as a window names it: the operator, a member who answers as him by his login,
  a model with its effort when its actor names one, a skill, or the engine itself.
  """

  model: str
  effort: NotRequired[str | None]
  role: Literal["model"]


class Maker4(TypedDict):
  """
  Who made an act, as a window names it: the operator, a member who answers as him by his login,
  a model with its effort when its actor names one, a skill, or the engine itself.
  """

  role: Literal["skill"]


class Maker5(TypedDict):
  """
  Who made an act, as a window names it: the operator, a member who answers as him by his login,
  a model with its effort when its actor names one, a skill, or the engine itself.
  """

  role: Literal["engine"]


type Maker = Maker1 | Maker2 | Maker3 | Maker4 | Maker5
"""
Who made an act, as a window names it: the operator, a member who answers as him by his login,
a model with its effort when its actor names one, a skill, or the engine itself.
"""


class Phase1(TypedDict):
  """
  Where an act stands: asked, started by the ear that owns it, or done, with what it came to.
  """

  phase: Literal["asked"]


class Phase2(TypedDict):
  """
  Where an act stands: asked, started by the ear that owns it, or done, with what it came to.
  """

  phase: Literal["started"]


class Phase3(TypedDict):
  """
  Where an act stands: asked, started by the ear that owns it, or done, with what it came to.
  """

  phase: Literal["done"]
  value: object


type Phase = Phase1 | Phase2 | Phase3
"""
Where an act stands: asked, started by the ear that owns it, or done, with what it came to.
"""


class Pause(TypedDict):
  """
  A pause, as its fact holds it: who said it, and the cause that it gave.
  """

  by: str
  why: NotRequired[object]


class RunState1(TypedDict):
  """
  How the word of a rung ran: it runs or waits for its word, it ran, or a fault or a cancel ended
  it, each with what a person reads.
  """

  run: Literal["running"]


class RunState2(TypedDict):
  """
  How the word of a rung ran: it runs or waits for its word, it ran, or a fault or a cancel ended
  it, each with what a person reads.
  """

  run: Literal["done"]


class RunState3(TypedDict):
  """
  How the word of a rung ran: it runs or waits for its word, it ran, or a fault or a cancel ended
  it, each with what a person reads.
  """

  run: Literal["failed"]
  reason: str


class RunState4(TypedDict):
  """
  How the word of a rung ran: it runs or waits for its word, it ran, or a fault or a cancel ended
  it, each with what a person reads.
  """

  run: Literal["cancelled"]
  reason: str


type RunState = RunState1 | RunState2 | RunState3 | RunState4
"""
How the word of a rung ran: it runs or waits for its word, it ran, or a fault or a cancel ended
it, each with what a person reads.
"""


class Read(TypedDict):
  """
  What an answer of a model read: the tokens of its whole prompt, and of them those that it read
  from the cache and those that it wrote to it.
  """

  tokens: int
  cached: int
  written: int


Mark1 = TypedDict("Mark1", {"is": Literal["ran"]})


Mark3 = TypedDict("Mark3", {"is": Literal["asks"]})


Mark5 = TypedDict("Mark5", {"why": str, "is": Literal["failed"]})


Mark6 = TypedDict("Mark6", {"at": NotRequired[int | None], "is": Literal["stopped"]})


type Work = Literal["writes", "command", "agent"]
"""
What a line that runs waits for: its model that writes it, a command or a wait, or an agent.
"""


class Retrying(TypedDict):
  """
  A call of a model that failed for a cause that passes, so its provider asks the model again:
  the attempt that comes, the most attempts of the call, the seconds that the provider waits
  before that attempt and the second of the epoch at which it comes, and why the attempt before
  it failed. What the model wrote before is void.
  """

  attempt: int
  most: int
  wait: int
  at: int
  why: str


class Output(TypedDict):
  """
  What a command that a line awaited wrote, as it writes it and once it ended: its last lines,
  those of its stdout then those of its stderr, each as a terminal shows it, how many lines came
  before them, and the seconds that it took once it ended, counted from milliseconds, which a
  host that opened the life after the command began does not know.
  """

  tail: list[str]
  more: int
  took: NotRequired[int | None]


class Made1(TypedDict):
  """
  What an act that a line made came to: it works, it failed, or it is done with a value, as a
  literal of python cut to `LITERAL` characters.
  """

  made: Literal["works"]


class Made2(TypedDict):
  """
  What an act that a line made came to: it works, it failed, or it is done with a value, as a
  literal of python cut to `LITERAL` characters.
  """

  made: Literal["failed"]


class Made3(TypedDict):
  """
  What an act that a line made came to: it works, it failed, or it is done with a value, as a
  literal of python cut to `LITERAL` characters.
  """

  made: Literal["done"]
  value: str


type Made = Made1 | Made2 | Made3
"""
What an act that a line made came to: it works, it failed, or it is done with a value, as a
literal of python cut to `LITERAL` characters.
"""


class Thought(TypedDict):
  """
  What a model thought before it wrote: the millisecond of the epoch at which it began to think,
  as the host saw it begin, and in how many seconds it thought, which is none while it thinks.
  """

  text: str
  began: int
  took: NotRequired[int | None]


Reply1 = TypedDict("Reply1", {"is": Literal["value"], "text": str})


Reply2 = TypedDict("Reply2", {"is": Literal["refused"], "text": str})


type Reply = Reply1 | Reply2
"""
What a question came to: a value, as a person reads it, or a refusal with its reason.
"""


class Context(TypedDict):
  """
  The context of a chain at its last answer: what that answer read, the window that the roster of
  the standing offers its actor, none when the roster holds no such actor, and the dollars of each
  answer on the chain.
  """

  read: Read
  window: NotRequired[int | None]
  dollars: float


class ProtocolError1(TypedDict):
  """
  The hello names another protocol, and the daemon closes the peer.
  """

  ours: int
  theirs: int
  error: Literal["protocol"]


class ProtocolError2(TypedDict):
  """
  The first frame of the peer was no hello, and the daemon closes the peer.
  """

  error: Literal["first"]


class ProtocolError3(TypedDict):
  """
  A second hello of the same peer.
  """

  error: Literal["again"]


class ProtocolError4(TypedDict):
  """
  Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
  """

  needs: Role
  has: Role
  error: Literal["role"]


class ProtocolError5(TypedDict):
  """
  A frame that is no frame of the protocol, with what the reader found.
  """

  why: str
  error: Literal["unread"]


class ProtocolError6(TypedDict):
  """
  A frame past the bound, and the daemon closes the peer.
  """

  bound: int
  error: Literal["large"]


class ProtocolError7(TypedDict):
  """
  The daemon or the life refused the verb, with why.
  """

  why: str
  error: Literal["refused"]


class ProtocolError8(TypedDict):
  """
  The daemon gave its place to the daemon of a new build, or stops.
  """

  error: Literal["stopped"]


class ProtocolError9(TypedDict):
  """
  Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
  """

  life: str
  error: Literal["closed"]


class ProtocolError10(TypedDict):
  """
  A life is named by the absolute path of its record, the same for each peer.
  """

  life: str
  error: Literal["relative"]


class ProtocolError11(TypedDict):
  """
  Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
  """

  life: str
  error: Literal["unattached"]


class ProtocolError12(TypedDict):
  """
  A peer may not end a life that the window of another peer shows.
  """

  life: str
  error: Literal["shown"]


class ProtocolError13(TypedDict):
  """
  Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
  """

  life: str
  holder: str
  error: Literal["unpinned"]


class ProtocolError14(TypedDict):
  """
  A turn or a write for a request of a model that the life does not wait for.
  """

  life: str
  ask: int
  error: Literal["unasked"]


class ProtocolError15(TypedDict):
  """
  A turn or a write for a request of a model that another peer answers: only the peer that
  opened the life with `answers` speaks for its requests.
  """

  life: str
  ask: int
  error: Literal["foreign"]


class ProtocolError16(TypedDict):
  """
  A turn for a life whose models no peer answers.
  """

  life: str
  error: Literal["answerless"]


class ProtocolError17(TypedDict):
  """
  An open with `answers` of a life whose models a peer that lives answers already.
  """

  life: str
  error: Literal["answering"]


class ProtocolError18(TypedDict):
  """
  A frame of a view that only a window says, or only a skill, and the peer is of another kind.
  """

  needs: Kind
  has: Kind
  error: Literal["speaker"]


class ProtocolError19(TypedDict):
  """
  A skill says its name at its hello, since a window mounts the views of a skill by it.
  """

  error: Literal["nameless"]


class ProtocolError20(TypedDict):
  """
  Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
  """

  life: str
  skill: str
  error: Literal["no_skill"]


class ProtocolError21(TypedDict):
  """
  A view belongs to the window that mounted it and to the skill that serves it, and to no other
  peer.
  """

  viewer: str
  error: Literal["unmounted"]


class ProtocolError22(TypedDict):
  """
  A viewer names one view of the daemon.
  """

  viewer: str
  error: Literal["mounted"]


type ProtocolError = (
  ProtocolError1
  | ProtocolError2
  | ProtocolError3
  | ProtocolError4
  | ProtocolError5
  | ProtocolError6
  | ProtocolError7
  | ProtocolError8
  | ProtocolError9
  | ProtocolError10
  | ProtocolError11
  | ProtocolError12
  | ProtocolError13
  | ProtocolError14
  | ProtocolError15
  | ProtocolError16
  | ProtocolError17
  | ProtocolError18
  | ProtocolError19
  | ProtocolError20
  | ProtocolError21
  | ProtocolError22
)
"""
Why the daemon refuses a frame of a peer. It crosses the protocol as data, beside its text.
"""


class Event1(TypedDict):
  """
  One fact that the life said, as JSON reads it: each fact of the record that the life said
  again as it opened, then each fact that it said after.
  """

  event: Literal["fact"]
  data: object


class Data2(TypedDict):
  rung: str
  chain: str
  text: str
  thinking: str


class Event3(TypedDict):
  """
  What a model added to its text and to its thought, as it writes the reply of a rung on a
  chain. It is no fact, and the turn of the reply holds all of it.
  """

  event: Literal["text"]
  data: Data2


class Data3(TypedDict):
  rung: str
  chain: str
  retry: Retrying


class Event4(TypedDict):
  """
  A call of a model that failed for a cause that passes, as it writes the reply of a rung on a
  chain, and the retry of its provider after a wait. What the model wrote for the rung before
  it is void. It is no fact.
  """

  event: Literal["retried"]
  data: Data3


class Event6(TypedDict):
  """
  A report of the life that no model reads, one line.
  """

  event: Literal["notice"]
  data: str


class Event7(TypedDict):
  """
  A chain whose queries give another answer now, once each fact that changed it was told.
  """

  event: Literal["changed"]
  data: str


class Event8(TypedDict):
  """
  The life told the past of its record and is born on it, with no fault: each event after it
  is of the birth, which tells each chain, its turns and each open prompt again as it stood.
  """

  event: Literal["reborn"]
  data: str | None


class Event9(TypedDict):
  """
  The end of the life, with the fault that ended it, or none when a client ended it. It is the
  last event.
  """

  event: Literal["ended"]
  data: str | None


class FollowUp(TypedDict):
  """
  One follow-up that waits on a chain: its entry, which the window that queued it chose, the shape
  that its prompt wants, its message, and why its send failed, when it did. A follow-up that
  failed holds the queue of its chain until the operator takes it out.
  """

  entry: str
  chain: str
  shape: str
  message: str
  failed: NotRequired[str | None]


type Stage = Literal["recall", "find", "cross", "read", "seed", "boot", "log"]
"""
A stage of the open of a life, in the order that an open passes them. An open skips a stage that
it has no work for, and a stage never comes again in one open. Each stage counts its work in a
unit of its own, and a stage that counts nothing tells 0 of 0.
"""


class Open(TypedDict):
  """
  An act that stays open in a life that stalled: its name, its kind, and the chain that it stands
  in.
  """

  act: str
  kind: str
  chain: str


class Request1(Hello):
  """
  One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
  `act`, since `id` is the key of the request.
  """

  id: int
  verb: Literal["hello"]


class Request17(TypedDict):
  """
  The draft that the person of the window writes now, or none once it is empty, which each
  window of the life hears in the presence. It is no fact, and it goes with the window.
  """

  id: int
  life: str
  draft: NotRequired[Draft | None]
  verb: Literal["writes"]


class Request48(TypedDict):
  """
  The ladder of a chain, now or as it stood just after a moment: the steps that show and the
  folds of the others, with the folds that the window opened, the prompts that the operator
  owes, the only acts at its top when `taken` names them, and how many last steps show.
  """

  id: int
  life: str
  on: NotRequired[str | None]
  at: NotRequired[int | None]
  opened: NotRequired[list[str]]
  owed: NotRequired[list[str]]
  taken: NotRequired[list[str] | None]
  shown: NotRequired[Shown | None]
  verb: Literal["ladder"]


class Request55(TypedDict):
  """
  A view that a window mounts in a slot of a life, which the process of a skill serves.
  """

  id: int
  viewer: str
  life: str
  slot: Slot
  skill: str
  verb: Literal["view.mount"]


class Request57(TypedDict):
  """
  What a person did to a node of a view, with the value of an input.
  """

  id: int
  viewer: str
  node: str
  kind: Gesture
  value: NotRequired[str | None]
  verb: Literal["view.event"]


class Request58(TypedDict):
  """
  What a skill asks of the one window of a view.
  """

  id: int
  viewer: str
  effect: Effect
  verb: Literal["view.effect"]


class Config2(TypedDict):
  """
  A key of a layer, the project when none is named, set to a value, or taken out with no
  value, then a stand of the life on the configs. A change that the catalog refuses puts its
  config back as it was, and the life does not stand.
  """

  key: list[str]
  value: NotRequired[object]
  layer: NotRequired[Layer | None]
  action: Literal["set"]


type Config = Config1 | Config2
"""
What a peer asks of the configs of a life, which the World writes, so no window writes a disk.
"""


class Run(TypedDict):
  """
  A run of a text, in a tone.
  """

  text: str
  tone: NotRequired[Tone]


class Frame2(TypedDict):
  """
  A refusal of the request of an id, or of a frame with no id that could be read, with the
  text that a person reads.
  """

  id: NotRequired[int | None]
  error: ProtocolError
  why: str
  frame: Literal["refused"]


class Frame4(TypedDict):
  """
  Each operator that a window of a life holds now, pushed to each of its windows after each
  attach, detach and draft.
  """

  life: str
  operators: list[Present]
  frame: Literal["presence"]


class Frame8(TypedDict):
  """
  How far the open of a life stands, pushed to each of its windows while the life opens: the
  stage, its work done and its work known, each in the unit of the stage, and both 0 for a
  stage that counts nothing. The stages of one open come in their order, the numbers of a stage
  never go back, and `done` never passes `of`. A window hears a frame at once for each stage
  that the open reaches, and at most four each second for a stage that goes on. The frame of
  `log` with `done` equal to `of` ends the open, and comes before the first event that the open
  tells. It is no event of the log and no fact of the record.
  """

  life: str
  stage: Stage
  done: int
  of: int
  frame: Literal["opening"]


class Frame9(TypedDict):
  """
  A life that holds open acts and said no fact for the bound of the daemon, pushed once to each
  of its windows by the watch of the daemon, which no word of the life runs: each open act, and
  the notice that a window shows as it stands.
  """

  life: str
  acts: list[Open]
  notice: str
  frame: Literal["stalled"]


class Data(TypedDict):
  protocol: int
  peer: Peer
  machine: str
  build: str


class Answer2(TypedDict):
  """
  The answer to a hello: the protocol of the daemon, the peer as the daemon knows it, the name
  of the machine of the daemon, and its build.
  """

  answer: Literal["welcome"]
  data: Data


class Answer3(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["lives"]
  data: list[Listed]


class Answer4(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["projects"]
  data: list[Project]


class Answer5(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["models"]
  data: list[Model]


class Answer6(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["files"]
  data: Files


class Answer9(TypedDict):
  """
  What the peer sent, the newest first.
  """

  answer: Literal["history"]
  data: list[Sent]


class Answer10(TypedDict):
  """
  The skills of a life, each with what it is for.
  """

  answer: Literal["skills"]
  data: list[Skill]


class Answer15(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["presence"]
  data: list[Present]


class Answer16(TypedDict):
  """
  The lives that a daemon gave at its handover.
  """

  answer: Literal["handed"]
  data: list[Handed]


class Answer20(TypedDict):
  """
  The ladder of a chain.
  """

  answer: Literal["ladder"]
  data: list[Rung]


class Changed(TypedDict):
  """
  A file of a checkout that differs from the last commit: its path from the top of the checkout,
  the lines that it adds and removes, and how it differs.
  """

  path: str
  added: int
  removed: int
  state: Change


class Card(TypedDict):
  """
  A card of the machine: one thread that an operator started, in the record of its session. It
  holds the name of the session when it has one, the main checkout of the project whose sessions
  hold the record, when the list of the machine names one, else the checkout whose
  `.furb/sessions` holds a record that a life holds, whether a life holds the record now,
  each operator whose window holds that life, and the second of the epoch since which the thread
  stands as it does. A record that has no index of its threads yet gives no card until
  the daemon made its index.
  """

  life: str
  session: NotRequired[str | None]
  project: NotRequired[str | None]
  refused: NotRequired[str | None]
  """
    Why no life of the machine opens the record of the card, whose thread then only reads.
    """
  held: bool
  viewers: list[str]
  pins: list[str]
  """
    The holder of each pin of the life, as `list` gives it.
    """
  since: int
  prompt: str
  title: str
  standing: Standing
  model: NotRequired[str | None]
  agents: NotRequired[int]
  readers: NotRequired[list[str]]


class Ask(TypedDict):
  """
  A prompt to the operator that waits for him. A question waits for an answer of its shape. A
  notice is a prompt of None, which the console closed as it took it, and which waits only until
  the operator dismisses it.
  """

  id: str
  chain: str
  """
    The chain that the prompt stands in.
    """
  by: str
  """
    Who made the prompt: a rung, an ear, or the operator.
    """
  shape: str
  message: str
  notice: bool
  choices: list[str]
  """
    The lines that answer a choice, one for each value that the shape allows.
    """
  ticks: list[Tick]
  """
    The values of a list of a Literal, which the operator ticks.
    """
  hint: str
  """
    What the line of the operator is, in words, when its shape is more than text or a choice.
    """
  began: int


class ChainThread(TypedDict):
  """
  One thread of a chain: its prompt, its name, its messages, the prompts to the operator that it
  made and the questions of them that wait, and its state. A thread is over once its prompt
  and each of its messages is done, and a message to a thread that is over opens it again until
  its model closes the message.
  """

  prompt: str
  title: str
  """
    The name of the thread: the label of its chain for the first thread that no operator made on
    a chain under the root, the name of the session for the first thread of the operator on the
    root, and else the first line of its message that is not empty.
    """
  messages: list[str]
  asked: list[str]
  asks: list[str]
  state: State
  over: bool
  origin: Origin
  yours: bool
  """
    Whether the operator started it, himself or by a word of his own.
    """


class ChainAside(TypedDict):
  """
  A prompt of a chain that is no thread, with what it is and the prompts on it, in order.
  """

  prompt: str
  aside: Aside
  messages: list[str]


class Spend(TypedDict):
  """
  What the answers of models cost: their dollars, how many they are, and the actor of the last
  and what it read.
  """

  dollars: float
  answers: int
  actor: NotRequired[str | None]
  account: NotRequired[str | None]
  """
    The account of the last answer, none when that answer named none.
    """
  read: NotRequired[Read | None]
  """
    What the last answer read.
    """


Mark2 = TypedDict("Mark2", {"work": Work, "began": NotRequired[int | None], "is": Literal["runs"]})


Mark4 = TypedDict("Mark4", {"attempt": int, "most": int, "wait": int, "at": int, "why": str, "is": Literal["retries"]})


type Mark = Mark1 | Mark2 | Mark3 | Mark4 | Mark5 | Mark6
"""
How a line stands: it ran, or will not run; it runs, with the work that it waits for and the
millisecond of the epoch at which the host saw it begin, so a time of a second shows after a
whole second; it asks the operator; its model is asked again after a call that failed; it
failed, with the reason that a person reads; or a stop ended it, at the millisecond of the epoch
at which the host saw the stop, which a host that opened the life after it does not know.
"""


class Answered(TypedDict):
  """
  What closed a question: who closed it, and what it came to.
  """

  by: Maker
  reply: Reply


class Data4(TypedDict):
  chain: str
  waiting: list[FollowUp]


class Event5(TypedDict):
  """
  The follow-ups that wait on a chain, in order, each time that a fact of the operator or a send
  of the life changed them.
  """

  event: Literal["queued"]
  data: Data4


class Request38(TypedDict):
  """
  The configs of the folder of the life, in their layers: what each key holds, a change of a
  key, or a stand on them as the disk holds them now.
  """

  id: int
  life: str
  config: Config
  verb: Literal["config"]


class Node2(TypedDict):
  """
  One node of a view, under an id that an event names.
  """

  id: str
  runs: list[Run]
  node: Literal["text"]


class Frame5(TypedDict):
  """
  A change of the cards of one record, pushed to each peer that asked the cards: each card that
  is new or that changed, and the prompt of each card that the record holds no more.
  """

  life: str
  changed: list[Card]
  gone: list[str]
  frame: Literal["cards"]


class Answer8(TypedDict):
  """
  The cards of the machine.
  """

  answer: Literal["cards"]
  data: list[Card]


class Changes(TypedDict):
  """
  The changes of a checkout of git against its last commit: its branch, none when its head names
  no branch, each file that differs, and the diff of the file that the peer named, as git writes
  a unified diff, a file that git does not track yet added whole.
  """

  branch: NotRequired[str | None]
  files: list[Changed]
  diff: NotRequired[str | None]


class Desk(TypedDict):
  """
  What waits for the operator in a life: each ask, the oldest first, and how many of them are
  questions and notices.
  """

  asks: list[Ask]
  questions: int
  notices: int


class ThreadNode(TypedDict):
  """
  A thread with the threads that its ladder made, its agents. Its state is its own, or asks while
  a question waits in an agent under it: a failure or an unread in an agent does not roll up.
  """

  thread: ChainThread
  agents: list[ThreadNode]
  state: State


class Act(TypedDict):
  """
  One act of a life with its meaning, as it stands after the last fact about it.
  """

  kind: str
  id: str
  by: str
  maker: NotRequired[Maker]
  """
    Who made the act: for a rung that waits for the word of an actor, that actor; for any
    other act, the role of the act or the person that made it.
    """
  on: str
  """
    What the act is on: a chain, or the prompt of a thread for a prompt on a thread.
    """
  chain: str
  """
    The chain that the act stands in.
    """
  words: list[object]
  """
    The words of the act after its kind, its name, its maker and what it is on.
    """
  phase: Phase
  operator: bool
  """
    Whether the console of the operator took the act: a question to him, or a notice.
    """
  pause: NotRequired[Pause | None]
  """
    The pause that holds the act while it lives.
    """
  run: NotRequired[RunState | None]
  """
    How the word of a rung ran, for a rung alone.
    """
  word: NotRequired[str | None]
  """
    The word of a rung, once its chain took it to run or to refuse.
    """
  asking: NotRequired[bool]
  """
    Whether a reply of a rung runs, while its model writes its word. A rung with no word and no
    reply that runs waits for its turn.
    """
  taken: NotRequired[list[str]]
  """
    The messages of the operator that a chain with a source took from it, in their order: each
    prompt of the operator that the prefix of the chain carries, with the id it has on the source.
    """
  began: int
  """
    How many facts the life had said when the act was made, which is its place in time.
    """
  ended: NotRequired[int | None]
  """
    How many facts the life had said when the act was done.
    """
  spend: NotRequired[Spend | None]
  """
    What the models cost: the answers that wrote the word of a rung, or each answer on a chain.
    """
  closer: NotRequired[Maker | None]
  """
    Who closed the act, as a window names who made an act, once a close of it was said.
    """
  state: State
  """
    The state of the act, or of the work of a chain for a chain.
    """


class Line(TypedDict):
  """
  One line of a block: its number in the thread, its text, in which each quote stands as the
  binding that it makes and a string of many lines as one line with its escapes, its mark, what
  the last command that it awaited wrote, and what each act that it made came to, in the order it
  made them: one for each pass of a loop, the last of which is the value that it bound.
  """

  number: int
  text: str
  mark: Mark
  output: NotRequired[Output | None]
  made: NotRequired[list[Made]]


class Question(TypedDict):
  shape: str
  answered: NotRequired[Answered | None]


class Saying1(TypedDict):
  """
  What a said is. A question holds the shape that its answer takes, which `choices`, `ticks` and
  `hint` of the view read, and what answered it once it is closed: it is open until then.
  """

  question: Question


type Saying = Literal["answer", "notice"] | Saying1
"""
What a said is. A question holds the shape that its answer takes, which `choices`, `ticks` and
`hint` of the view read, and what answered it once it is closed: it is open until then.
"""


class Agent(TypedDict):
  """
  An agent of a thread: the thread that a word of the thread started on another chain, as a card,
  with the millisecond of the epoch at which the host saw it begin, the seconds that it took once
  it ended, counted from milliseconds as the time of a thought, and the line of its last word
  where it stands: the line that runs or asks, else the last one, and the number in the thread
  of the line of the word that made it, none when no line of a word of the thread made it. A host
  that opened the life after the agent began saw it begin at that open.
  """

  prompt: str
  title: str
  standing: Standing
  model: NotRequired[str | None]
  agents: NotRequired[int]
  readers: NotRequired[list[str]]
  began: NotRequired[int | None]
  took: NotRequired[int | None]
  line: NotRequired[Line | None]
  at: NotRequired[int | None]


class Event2(TypedDict):
  """
  An act with its meaning as it stands after a fact about it, which takes the place of each
  earlier event of the same act.
  """

  event: Literal["act"]
  data: Act


type Event = Event1 | Event2 | Event3 | Event4 | Event5 | Event6 | Event7 | Event8 | Event9
"""
What a life tells its clients, in the order of its log.
"""


class Frame3(TypedDict):
  """
  One event of a life that the peer attached, under its place in the log of the life.
  """

  life: str
  seq: int
  event: Event
  frame: Literal["event"]


class Answer7(TypedDict):
  """
  What a verb comes to.
  """

  answer: Literal["changes"]
  data: Changes


class Answer17(TypedDict):
  """
  What waits for the operator.
  """

  answer: Literal["desk"]
  data: Desk


class Answer19(TypedDict):
  """
  The threads of every chain of a life, as one tree.
  """

  answer: Literal["rail"]
  data: list[ThreadNode]


class Answer21(TypedDict):
  """
  The acts as they stood at a moment, or the one act that `ended` names.
  """

  answer: Literal["acts"]
  data: list[Act]


class Threads(TypedDict):
  """
  The threads of one chain as a window draws them: the tree of its threads, its asides, the runs
  of its notes, what the tree says to the operator, and the state of the chain.
  """

  threads: list[ThreadNode]
  asides: list[ChainAside]
  notes: list[NoteRun]
  attention: Attention
  state: State


class Block(TypedDict):
  """
  The code of one answer: the model that wrote its last word, each line of its words, whether more
  comes since the answer works, and the thought of its model.
  """

  maker: Maker
  lines: list[Line]
  more: bool
  thought: NotRequired[Thought | None]


class Said(TypedDict):
  """
  What an answer says to the operator: the value of the close of its message, a notice, or a
  question, with its act and its text.
  """

  act: str
  kind: Saying
  text: str


class Answer18(TypedDict):
  """
  The threads of a chain.
  """

  answer: Literal["threads"]
  data: Threads


class Exchange(TypedDict):
  """
  One message of a thread with all that answers it: its place in the thread, 0 for the prompt of
  the thread, the act of the message, who said it and its text, whether it is a word of Python
  that an operator ran on the thread, whose block holds its code and which has no text, whether
  the message still runs, the block of code that answers it once a model answers, and what the
  answer says to the operator, in the order it was said. A message that the operator said on a
  thread that works is done at once, since its model reads it at its next step, so only one that
  asks still runs. A message is the prompt of the thread, what an operator said on it, or a word
  that he ran on it, and what answers it is what came after it and before the next message.
  """

  at: int
  message: str
  by: Maker
  text: str
  python: NotRequired[bool]
  runs: NotRequired[bool]
  block: NotRequired[Block | None]
  said: list[Said]


class Page(TypedDict):
  """
  Some exchanges of a thread, the oldest first, with how many exchanges stand before them, the
  agents of the whole thread, in the order the thread made them, and the context of its chain.
  """

  exchanges: list[Exchange]
  above: int
  agents: list[Agent]
  context: NotRequired[Context | None]


class Frame7(TypedDict):
  """
  A change of the thread that the peer reads: each exchange that is new or that changed, how
  many exchanges stand before the first of the page of the end now, and the agents of the
  thread.
  """

  life: str
  thread: str
  page: Page
  frame: Literal["thread"]


class Answer23(TypedDict):
  """
  A page of a thread.
  """

  answer: Literal["page"]
  data: Page


type Answer = (
  Answer1
  | Answer2
  | Answer3
  | Answer4
  | Answer5
  | Answer6
  | Answer7
  | Answer8
  | Answer9
  | Answer10
  | Answer11
  | Answer12
  | Answer13
  | Answer14
  | Answer15
  | Answer16
  | Answer17
  | Answer18
  | Answer19
  | Answer20
  | Answer21
  | Answer22
  | Answer23
)
"""
What a verb comes to.
"""


class Frame1(TypedDict):
  """
  The answer to the request of an id.
  """

  id: int
  answer: Answer
  frame: Literal["reply"]


class Protocol(TypedDict):
  """
  Both ways of the protocol, which its schema holds.
  """

  request: Request
  frame: Frame


class Request56(TypedDict):
  """
  The tree of a view, which takes the place of each tree of a lower version.
  """

  id: int
  viewer: str
  slot: Slot
  version: int
  tree: Node
  verb: Literal["view.tree"]


type Request = (
  Request1
  | Request2
  | Request3
  | Request4
  | Request5
  | Request6
  | Request7
  | Request8
  | Request9
  | Request10
  | Request11
  | Request12
  | Request13
  | Request14
  | Request15
  | Request16
  | Request17
  | Request18
  | Request19
  | Request20
  | Request21
  | Request22
  | Request23
  | Request24
  | Request25
  | Request26
  | Request27
  | Request28
  | Request29
  | Request30
  | Request31
  | Request32
  | Request33
  | Request34
  | Request35
  | Request36
  | Request37
  | Request38
  | Request39
  | Request40
  | Request41
  | Request42
  | Request43
  | Request44
  | Request45
  | Request46
  | Request47
  | Request48
  | Request49
  | Request50
  | Request51
  | Request52
  | Request53
  | Request54
  | Request55
  | Request56
  | Request57
  | Request58
  | Request59
)
"""
One request of a peer: an id that the reply gives back, and the verb. A verb names an act as
`act`, since `id` is the key of the request.
"""


class Node1(TypedDict):
  """
  One node of a view, under an id that an event names.
  """

  id: str
  axis: Axis
  gap: int
  border: bool
  children: list[Node]
  node: Literal["box"]


class Node3(TypedDict):
  """
  One node of a view, under an id that an event names.
  """

  id: str
  items: list[Node]
  node: Literal["list"]


class Node4(TypedDict):
  """
  One node of a view, under an id that an event names.
  """

  id: str
  header: list[str]
  rows: list[list[Node]]
  node: Literal["table"]


type Node = Node1 | Node2 | Node3 | Node4 | Node5 | Node6 | Node7 | Node8 | Node9 | Node10
"""
One node of a view, under an id that an event names.
"""


class Frame14(TypedDict):
  """
  The tree of a view, which takes the place of each tree of a lower version.
  """

  viewer: str
  slot: Slot
  version: int
  tree: Node
  frame: Literal["view.tree"]


type Frame = (
  Frame1
  | Frame2
  | Frame3
  | Frame4
  | Frame5
  | Frame6
  | Frame7
  | Frame8
  | Frame9
  | Frame10
  | Frame11
  | Frame12
  | Frame13
  | Frame14
  | Frame15
  | Frame16
  | Frame17
)
"""
What the daemon says to a peer.
"""

type Mount = Frame13
type ViewEvent = Frame15
type Dispose = Frame17

PROTOCOL = 47
