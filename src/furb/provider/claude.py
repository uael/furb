"""The claude CLI as a pydantic_ai model: one warm process per conversation, and only ever the delta is sent.

A session is one `claude -p` process holding one conversation, fixed at its model, its effort, its system
prompt and the conversation it answers. It is cooled by killing the process, which the conversation survives on
disk under its own id, and revived by resuming it, so one turn ever writes the messages claude has not already
consumed. A subscription pays for the turns and no API key is read.
"""

import asyncio
import json
import os
import shutil
import time
from base64 import b64encode
from collections.abc import AsyncGenerator, AsyncIterator, Mapping, Sequence
from contextlib import asynccontextmanager, suppress
from dataclasses import dataclass, field
from datetime import UTC, datetime
from decimal import Decimal
from hashlib import sha256
from typing import Literal, NoReturn
from uuid import uuid4

from pydantic import ConfigDict, ValidationError
from pydantic import TypeAdapter as Adapter
from pydantic.dataclasses import dataclass as sealed
from pydantic_ai import RunContext
from pydantic_ai.messages import (
  BinaryContent,
  FinishReason,
  ModelMessage,
  ModelRequest,
  ModelResponse,
  ModelResponseStreamEvent,
  RetryPromptPart,
  SystemPromptPart,
  TextPart,
  ThinkingPart,
  ToolCallPart,
  ToolReturnPart,
  UserPromptPart,
)
from pydantic_ai.models import Model, ModelRequestParameters, StreamedResponse
from pydantic_ai.profiles import ModelProfile, ModelProfileSpec
from pydantic_ai.providers import Provider
from pydantic_ai.settings import ModelSettings, ThinkingLevel
from pydantic_ai.usage import RequestUsage

from furb.engine import OPERATOR, WINDOW, Refused

API = "claude-cli"
BIN = "FURB_CLAUDE_BIN"
HELD = "FURB_CLAUDE_STALL"
MILLION = 1_000_000
FLOOR = (200_000, 32_000)
STALL = 900.0
WARM = 8
KEEP = 64
IDLE = 300.0
ROOM = 32 * 1024 * 1024
TAIL = 2000
DATED = 9
LOOSE = ConfigDict(extra="ignore")
LEVELS = ("low", "medium", "high", "xhigh", "max")
LIVE: list[Session] = []
CLI: list[Cli] = []

LIMITS: Mapping[str, tuple[int, int]] = {
  "fable": (MILLION, 128_000),
  "opus": (MILLION, 128_000),
  "sonnet": (MILLION, 64_000),
  "haiku": (200_000, 64_000),
  "claude-fable-5": (MILLION, 128_000),
  "claude-mythos-5": (MILLION, 128_000),
  "claude-opus-5": (MILLION, 128_000),
  "claude-opus-4-8": (MILLION, 128_000),
  "claude-opus-4-7": (MILLION, 128_000),
  "claude-opus-4-6": (MILLION, 128_000),
  "claude-sonnet-4-6": (MILLION, 64_000),
  "claude-haiku-4-5": (200_000, 64_000),
}
# The CLI's own ladder is low, medium, high, xhigh, max, so every shared name means what it says and only
# `minimal`, which claude has no tier for, has to floor. `max` is claude's alone: a caller that speaks the CLI's
# vocabulary lands on the top tier rather than falling through to whatever claude would have picked by itself.
EFFORT: Mapping[str, str] = {
  "minimal": "low",
  "low": "low",
  "medium": "medium",
  "high": "high",
  "xhigh": "xhigh",
  "max": "max",
}
# What a caller may name, which is the family aliases: a dated name still resolves, but nothing offers one.
FAMILY = tuple(k for k in LIMITS if not k.startswith("claude-"))
ACTOR = "opus/low"
"""ACTOR is the actor a chain stands on when the operator names none, which is a name of the family and an effort."""
STOP: Mapping[str, FinishReason] = {
  "end_turn": "stop",
  "stop_sequence": "stop",
  "max_tokens": "length",
  "refusal": "content_filter",
}
PROFILE = ModelProfile(
  supports_tools=False, supports_json_schema_output=False, supports_json_object_output=False, supports_thinking=True
)


class Settings(ModelSettings, total=False):
  """What a request carries beyond its messages: which conversation it belongs to, which one it forks, and how hard to think."""

  claude_session_id: str
  claude_fork_of: str
  claude_effort: str


@sealed(frozen=True, slots=True, kw_only=True, config=LOOSE)
class Spend:
  """What a turn cost, in the CLI's own words, where the input count excludes whatever the cache served."""

  input_tokens: int = 0
  output_tokens: int = 0
  cache_creation_input_tokens: int = 0
  cache_read_input_tokens: int = 0


@sealed(frozen=True, slots=True, kw_only=True, config=LOOSE)
class Block:
  """One content block of an assistant message: text, thinking, or a kind this provider does not read."""

  type: str = ""
  text: str = ""
  thinking: str = ""
  signature: str | None = None


@sealed(frozen=True, slots=True, kw_only=True, config=LOOSE)
class Note:
  """The API message an `assistant` event carries, whose usage is a message-start snapshot and not the turn's total."""

  content: tuple[Block, ...] = ()
  usage: Spend | None = None


@sealed(frozen=True, slots=True, kw_only=True, config=LOOSE)
class Said:
  """One line of claude's stream-json output, as much of it as this provider reads."""

  type: str = ""
  session_id: str = ""
  is_error: bool = False
  result: str | None = None
  stop_reason: str | None = None
  total_cost_usd: float | None = None
  message: Note | None = None
  usage: Spend | None = None


SAID = Adapter(Said)


@dataclass(frozen=True, slots=True)
class Seat:
  """What a process is fixed at when it spawns, so no request unlike these can ever be served by it."""

  name: str
  effort: str | None
  said: str
  sid: str


def wrong(name: str, why: str) -> NoReturn:
  """How every turn fails: the model that was asked, and claude's own words for what went wrong."""
  said = f"{name}: {why}"
  raise Refused(said)


def listed(one: Session, *, on: bool) -> None:
  """The one hand that writes the list of live processes.

  It is named for the list it writes, since roster in this engine is the actors the World offers, and each name
  has one meaning.
  """
  if on:
    LIVE.append(one)
  elif one in LIVE:
    LIVE.remove(one)


async def cool() -> None:
  """Kill every process this module started, whichever pool started it. Nothing outlives the life that asked for it."""
  while LIVE:
    await LIVE[-1].stop()


def default() -> Cli:
  """The provider a model gets when it is handed none: one binary and one session budget for the whole process."""
  if not CLI:
    CLI.append(Cli())
  return CLI[0]


def limits(name: str) -> tuple[int, int]:
  """What a claude model can hold and how much it may write, by full name or by the family alias of the CLI."""
  dated = len(name) > DATED and name[-DATED] == "-" and name[1 - DATED :].isdigit()
  return LIMITS.get(name) or LIMITS.get(name[:-DATED] if dated else name) or FLOOR


def actors() -> tuple[tuple[str, tuple[str, ...], int], ...]:
  """The actors the World offers: the operator, and each model of the family at its efforts, with its window."""
  return ((OPERATOR, (), WINDOW), *((name, tuple(LEVELS), limits(name)[0]) for name in FAMILY))


def effort_of(level: ThinkingLevel | str | None) -> str | None:
  """A thinking level as the `--effort` of the CLI, or nothing, which leaves claude the default it would have picked."""
  return EFFORT.get(level) if isinstance(level, str) else None


def mine(settings: ModelSettings | None) -> tuple[str, str, str]:
  """The three settings this provider reads for itself, off the mapping a TypedDict is at runtime."""
  said = dict(settings or {})

  def word(key: str) -> str:
    got = said.get(key)
    return got if isinstance(got, str) else ""

  return word("claude_session_id"), word("claude_fork_of"), word("claude_effort")


def spent(usage: Spend | None, cost: float | None) -> RequestUsage:
  """The counts of claude in the convention of pydantic_ai, where input_tokens holds everything the cache served."""
  got = usage or Spend()
  read, wrote = got.cache_read_input_tokens, got.cache_creation_input_tokens
  return RequestUsage(
    input_tokens=got.input_tokens + read + wrote,
    output_tokens=got.output_tokens,
    cache_read_tokens=read,
    cache_write_tokens=wrote,
    cost=None if cost is None else Decimal(str(cost)),
  )


def canon(message: ModelMessage) -> str:
  """A message as its meaning alone: two structurally equal conversations fingerprint the same, whatever else differs.

  Timestamps, ids, usage and thinking signatures are all left out. A journalled reply carries an emptied thinking part
  where the live stream held one, so thinking counts by presence alone or a replayed turn would match nothing.
  """
  said: list[object] = []
  for part in message.parts:
    if isinstance(part, UserPromptPart):
      told = part.content
      said.append(["u", told if isinstance(told, str) else [b if isinstance(b, str) else shown(b) for b in told]])
    elif isinstance(part, ToolReturnPart):
      said.append(["r", part.tool_name, part.model_response_str()])
    elif isinstance(part, RetryPromptPart):
      said.append(["e", part.model_response()])
    elif isinstance(part, TextPart):
      said.append(["t", part.content])
    elif isinstance(part, ThinkingPart):
      said.append(["k"])
    elif isinstance(part, ToolCallPart):
      said.append(["c", part.tool_name, part.args_as_json_str()])
  return sha256(json.dumps(said, default=repr).encode()).hexdigest()[:16]


def shown(item: object) -> str:
  """One piece of user content that is not plain text, as the string that stands for it in a fingerprint."""
  if isinstance(item, BinaryContent):
    return f"{item.media_type}:{sha256(item.data).hexdigest()[:16]}"
  return repr(item)


def alike(held: Sequence[str], incoming: Sequence[str]) -> int:
  """How far two fingerprint chains agree, which is how much of a conversation a fork of one already holds."""
  n = 0
  while n < min(len(held), len(incoming)) and held[n] == incoming[n]:
    n += 1
  return n


def based(chain: Sequence[str], asked: Sequence[str], incoming: Sequence[str]) -> int | None:
  """Where a request continues a session from: past the reply when the caller echoed it, past the last request when it
  did not, and nowhere at all when it continues neither, which makes it a new conversation on an old key."""
  if not chain and not asked:
    return 0
  for held in (chain, asked):
    if held and len(incoming) >= len(held) and list(incoming[: len(held)]) == list(held):
      return len(held)
  return None


class Session:
  """One `claude -p` process holding one conversation.

  Fixed for its whole life: the model, the effort, the system prompt and the conversation id. It is cooled by killing
  the process (the conversation survives on disk under that id) and revived by resuming it, so a turn only ever writes
  the messages claude has not already consumed.
  """

  def __init__(self, seat: Seat, parent: str = "") -> None:
    self.seat = seat
    self.id = str(uuid4())
    self.parent = parent
    self.mode: Literal["fresh", "resume", "branch"] = "branch" if parent else "fresh"
    self.chain: list[str] = []
    self.asked: list[str] = []
    self.used = time.monotonic()
    self.busy = False
    self.live = False
    self.again = False
    self.taking = False
    self.done = ""
    self.paid = 0.0
    self.hurt = ""
    self.odd = ""
    self.bin = os.environ.get(BIN) or "claude"
    self.stall = float(os.environ.get(HELD) or STALL)
    self.child: asyncio.subprocess.Process | None = None
    self.ears: asyncio.Task[None] | None = None
    self.eyes: asyncio.Task[None] | None = None
    self.heard: asyncio.Queue[Said] = asyncio.Queue()
    # One process holds one conversation, so it answers one turn at a time. Two callers arriving on the same key is
    # a caller naming one conversation twice, not two conversations: the second waits and then extends the first.
    self.alone = asyncio.Lock()

  @property
  def warm(self) -> bool:
    """Whether a process is standing, which is the only thing a cooling takes away."""
    return self.child is not None

  @asynccontextmanager
  async def turn(self, messages: Sequence[ModelMessage]) -> AsyncGenerator[AsyncIterator[Said]]:
    """Forward what claude has not heard, then hand back the stream it answers with, up to the turn's end.

    The whole of it stands under one lock. A process holds one conversation and answers one input line at a time, so
    a second caller arriving on the same key waits and then extends what the first left, rather than writing its own
    line into the middle of someone else's turn and reading the answer meant for them.
    """
    async with self.alone:
      self.opens()
      incoming = [canon(m) for m in messages]
      base = based(self.chain, self.asked, incoming)
      # An exact duplicate (the retry of a failed turn) has an empty delta, and writing nothing would wait on a
      # boundary that never comes. Re-send the live tail into the same conversation, which pays one repeated ask over
      # a cached prefix; if that retry fails too, the conversation is past saving and the next duplicate starts anew.
      if base is None or (base > 0 and base >= len(incoming) and self.again):
        await self.reset()
        base = 0
      elif base > 0 and base >= len(incoming):
        self.again = True
        base = len(incoming) - 1
      # Nothing an earlier turn left behind can answer this one, and the arming happens before the process starts, so
      # a spawn that dies at once fails this turn rather than leaving it to wait out the stall. Both are why the reset
      # above stands ahead of it: cooling a session is not a turn failing.
      self.heard = asyncio.Queue()
      self.live = True
      try:
        await self.spawn()
        self.write(messages[base:])
        yield self.hear()
      finally:
        # Both positions move past everything claude consumed: `chain` includes the reply a chat-style caller will
        # echo back, `asked` holds the request alone, which is what a compiled-context caller extends. A failed turn
        # keeps neither the reply nor its own retry count.
        self.asked = incoming
        self.chain = [*incoming, self.done] if self.done else incoming
        self.again = False if self.done else self.again
        self.busy = False
        self.live = False
        self.used = time.monotonic()

  def opens(self) -> None:
    """The turn begins: the session is busy, and what the turn before it settled is behind this one."""
    self.busy, self.done, self.used = True, "", time.monotonic()

  async def hear(self) -> AsyncIterator[Said]:
    """Every event of this turn, up to the one that ends it, and never a silence longer than the stall.

    claude does go permanently quiet: on a subscription limit it says one thing, never emits a result, and idles. The
    stall makes that a failed turn the caller can retry instead of a run wedged on a boundary that will not come.
    """
    while True:
      try:
        said = await asyncio.wait_for(self.heard.get(), self.stall)
      except TimeoutError:
        await self.stop()
        odd = f", and the last line this could not read was {self.odd}" if self.odd else ""
        wrong(self.seat.name, f"claude said nothing this could read for {self.stall:.0f}s with a turn in flight{odd}")
      yield said
      if said.type == "result":
        return

  def settled(self, reply: ModelResponse) -> None:
    """The turn ended clean, and claude's own history now holds this reply, so the session's position moves past it."""
    self.done = canon(reply)

  def charge(self, total: float | None) -> float | None:
    """This turn's share of what the process has cost: claude reports a running total, so we keep the watermark."""
    if total is None:
      return None
    paid, self.paid = max(0.0, total - self.paid), total
    return paid

  async def spawn(self) -> None:
    """Stand the process up, resuming the conversation whenever one has already stood here."""
    if self.child is not None:
      return
    seat = self.seat
    args = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"]
    if self.mode == "fresh":
      args += ["--session-id", self.id]
    elif self.mode == "resume":
      args += ["--resume", self.id]
    else:
      args += ["--resume", self.parent, "--fork-session"]
    args += ["--model", seat.name]
    if seat.effort:
      args += ["--effort", seat.effort]
    # A completion engine has no use for tools: `--tools ""` empties the built-in set, current and future, and
    # `--strict-mcp-config` with no config given keeps every other server out, so nothing here can need a permission.
    args += [
      "--system-prompt",
      seat.said,
      "--setting-sources",
      "",
      "--strict-mcp-config",
      "--tools",
      "",
      "--disable-slash-commands",
    ]
    found = shutil.which(self.bin)
    if found is None:
      wrong(seat.name, f"no {self.bin} on PATH")
    self.hurt = ""
    self.taking = self.mode == "branch"
    pipe = asyncio.subprocess.PIPE
    child = await asyncio.create_subprocess_exec(
      found,
      *args,
      stdin=pipe,
      stdout=pipe,
      stderr=pipe,
      limit=ROOM,
      env={
        **os.environ,
        "CLAUDE_CODE_DISABLE_BUNDLED_SKILLS": "1",
        # The harness uses claude as a completion engine, so nonessential traffic is pure tax: the session-title
        # generator alone re-sends the whole conversation to the main model on every turn, uncached and unreported.
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
      },
    )
    self.child = child
    self.paid = 0.0
    self.mode = "resume"
    self.ears = asyncio.ensure_future(self.listen(child))
    self.eyes = asyncio.ensure_future(self.watch(child))
    listed(self, on=True)

  async def listen(self, child: asyncio.subprocess.Process) -> None:
    """Everything the process says, line by line, until it stops saying anything at all."""
    out = child.stdout
    assert out is not None
    while True:
      try:
        raw = await out.readline()
      except ValueError:
        # One line longer than this reads with. What is left of it arrives as the start of the next, so nothing
        # after it can be trusted to be a line at all: the turn ends on the size that beat it, not on the stall.
        self.gone(f"claude said a line past the {ROOM} bytes this reads with")
        with suppress(ProcessLookupError):
          child.kill()
        break
      if not raw:
        break
      try:
        said = SAID.validate_json(raw)
      except ValidationError:
        # A line this cannot read is kept the way stderr is: a turn that then stalls said plenty, and this is it.
        self.odd = raw.decode(errors="replace").strip()[-TAIL:]
        continue
      # A fork is told its own id by the first event it emits; adopting it is what lets this session resume the branch.
      if said.session_id and self.taking:
        self.id, self.taking = said.session_id, False
      if said.type in ("assistant", "result") and self.live:
        self.heard.put_nowait(said)
    # Reaching here means this process is gone, and it is still ours: a cooling cancels this task and waits for it to
    # end before anything else may spawn, so no later child can be standing behind this one's last word.
    code = await child.wait()
    self.child = None
    listed(self, on=False)
    self.gone(
      f"claude exited{f' ({code})' if code else ''}{f': {self.hurt.strip()[-400:]}' if self.hurt.strip() else ''}"
    )

  async def watch(self, child: asyncio.subprocess.Process) -> None:
    """A short tail of what the process complained about, kept for whichever failure comes next."""
    err = child.stderr
    assert err is not None
    while raw := await err.read(4096):
      self.hurt = (self.hurt + raw.decode(errors="replace"))[-TAIL:]

  def gone(self, why: str) -> None:
    """Fail whatever turn is in flight, since the process that owed it an answer cannot give one now."""
    if self.live:
      self.heard.put_nowait(Said(type="result", is_error=True, result=why))

  def write(self, delta: Sequence[ModelMessage]) -> None:
    """Hand the delta to the process, or fail the turn with whatever the dead one left on stderr."""
    child = self.child
    if child is None or child.stdin is None:
      wrong(self.seat.name, f"claude is not running{f': {self.hurt.strip()[-400:]}' if self.hurt.strip() else ''}")
    child.stdin.write(self.line(delta).encode())

  def line(self, delta: Sequence[ModelMessage]) -> str:
    """The delta as one input line: every earlier turn framed as context, and the last ask left as itself.

    One line is load-bearing. `claude -p` answers each user input as its own turn and this call resolves at the first
    boundary, so writing a seed and an ask as two lines would answer the seed and never see the ask.
    """
    blocks: list[dict[str, object]] = []
    text = ""

    def flush() -> None:
      nonlocal text
      if text.strip():
        blocks.append({"type": "text", "text": text.rstrip()})
      text = ""

    def fed(items: Sequence[object]) -> None:
      nonlocal text
      for item in items:
        if isinstance(item, str):
          text += item + "\n\n"
        elif isinstance(item, BinaryContent) and item.is_image:
          flush()
          blocks.append(
            {
              "type": "image",
              "source": {"type": "base64", "media_type": item.media_type, "data": b64encode(item.data).decode()},
            }
          )
        else:
          wrong(
            self.seat.name,
            f"the stream-json input of claude carries text and images alone, never {type(item).__name__}",
          )

    for message in delta:
      if isinstance(message, ModelResponse):
        # claude owns the assistant side of its own history; a crafted seed turn can only arrive as framed context.
        say = "".join(p.content for p in message.parts if isinstance(p, TextPart)).strip()
        text += f"[earlier turn: you]\n{say}\n\n" if say else ""
        continue
      for part in message.parts:
        if isinstance(part, ToolReturnPart):
          text += f"[earlier observation]\n{part.model_response_str()}\n\n"
        elif isinstance(part, RetryPromptPart):
          text += f"{part.model_response()}\n\n"
        elif isinstance(part, UserPromptPart):
          # No role label: a labelled transcript teaches the model a format it then continues past its own turn.
          fed([part.content] if isinstance(part.content, str) else part.content)
    flush()
    if not blocks:
      wrong(self.seat.name, "nothing to send: this request says nothing claude has not already heard")
    return json.dumps({"type": "user", "message": {"role": "user", "content": blocks}}) + "\n"

  async def reset(self) -> None:
    """Give the conversation up entirely: a request that continues nothing gets a session holding nothing."""
    await self.stop()
    self.id = str(uuid4())
    self.chain = []
    self.asked = []
    self.again = False
    self.mode = "fresh"

  async def stop(self) -> None:
    """Cool the session: the process dies, the conversation does not, since claude keeps it under our id."""
    child, self.child = self.child, None
    listed(self, on=False)
    for task in (self.ears, self.eyes):
      if task is not None:
        task.cancel()
        with suppress(asyncio.CancelledError):
          await task
    self.ears = self.eyes = None
    if child is not None:
      with suppress(ProcessLookupError):
        child.kill()
      await child.wait()
    self.gone("the session was stopped")


class Pool:
  """Every conversation this process holds, and the budget they share.

  A session is keyed by everything fixed at spawn, so the same key reuses the warm process and forwards only the
  delta. Over the warm cap the least recently used idle session is cooled, and over the tracked cap the coldest dead
  entry is dropped, which is what bounds a run that mints a fresh conversation for every sub-agent it spawns.
  """

  def __init__(self, cap: int = WARM, keep: int = KEEP, idle: float = IDLE) -> None:
    self.sessions: dict[Seat, Session] = {}
    self.cap = cap
    self.keep = keep
    self.idle = idle

  async def route(self, seat: Seat, fork: str, messages: Sequence[ModelMessage]) -> Session:
    """The session holding this conversation: the one keyed to it, a fork of the one it grew out of, or a new one."""
    got = self.sessions.get(seat)
    if got is None:
      incoming = [canon(m) for m in messages]
      got = self.forked(seat, fork, incoming) or self.tipped(seat, messages, incoming) or Session(seat)
      self.sessions[seat] = got
    await self.sweep(seat)
    return got

  def forked(self, seat: Seat, fork: str, incoming: Sequence[str]) -> Session | None:
    """A fork of the conversation this request names as its parent, which already holds everything the two share.

    A same-task hop names what it continues while interleaving messages of its own, so its tip can never match. The
    forked process holds the parent transcript, the shared prefix reads from the provider's cache, and only the
    divergence is ever uploaded.
    """
    if not fork:
      return None
    kin = self.sessions.get(Seat(seat.name, seat.effort, seat.said, fork)) or next(
      (s for k, s in self.sessions.items() if k.name == seat.name and k.said == seat.said and k.sid == fork), None
    )
    if kin is None or kin.busy:
      return None
    # The fork holds the parent's whole transcript, the parent's own replies included, so what the two share is
    # measured against whichever of its positions runs longer: the reply-inclusive one when the caller echoed it back.
    shared = max(alike(kin.chain, incoming), alike(kin.asked, incoming))
    if not shared:
      return None
    kid = Session(seat, parent=kin.id)
    kid.chain = list(incoming[:shared])
    return kid

  def tipped(self, seat: Seat, messages: Sequence[ModelMessage], incoming: Sequence[str]) -> Session | None:
    """The idle sibling sitting exactly at this request's branch point, so the fork inherits its whole prefix.

    Effort is deliberately not compared: the branch point is the conversation, and the fork spawns at this request's
    own effort. A historical edit, whose tail carries an assistant turn, is not forkable without per-turn checkpoints,
    so it falls through to a session of its own.
    """
    best: Session | None = None
    for k, s in self.sessions.items():
      if s.busy or k.name != seat.name or k.said != seat.said or not s.chain or len(s.chain) > len(incoming):
        continue
      if list(incoming[: len(s.chain)]) != s.chain or any(
        isinstance(m, ModelResponse) for m in messages[len(s.chain) :]
      ):
        continue
      if best is None or len(s.chain) > len(best.chain):
        best = s
    if best is None:
      return None
    kid = Session(seat, parent=best.id)
    kid.chain = list(best.chain)
    return kid

  async def sweep(self, keep: Seat) -> None:
    """Give up what this turn does not need: whatever has sat idle, then the coldest process over the warm cap, then
    the coldest dead entry over the tracked cap. Called on every route, so an idle pool sheds at the next ask."""
    now = time.monotonic()

    def older(held: tuple[Seat, Session]) -> float:
      return held[1].used

    def resting(*, warm: bool) -> list[tuple[Seat, Session]]:
      # The session itself, never its key: each waits on a process, and a sweep that raced this one to the same
      # one has already taken that key out of the pool by the time this comes back to look the session up.
      held = ((k, s) for k, s in self.sessions.items() if k != keep and s.warm is warm and not s.busy)
      return sorted(held, key=older)

    for _, said in resting(warm=True):
      if now - said.used > self.idle or self.hot() >= self.cap:
        await said.stop()
    for seat, said in resting(warm=False):
      if len(self.sessions) <= self.keep:
        break
      await said.stop()
      self.sessions.pop(seat, None)

  def hot(self) -> int:
    """How many processes are standing, which is what the warm cap counts."""
    return sum(1 for s in self.sessions.values() if s.warm)

  async def close(self) -> None:
    """Give up every conversation at once: for shutdown, and for a test that must leave nothing behind."""
    for s in list(self.sessions.values()):
      await s.stop()
    self.sessions.clear()


class Cli(Provider[object]):
  """The claude binary as a provider: keyless, since the CLI bills a subscription, and the pool is the client it hands out."""

  def __init__(self, pool: Pool | None = None) -> None:
    self.pool = Pool() if pool is None else pool

  @property
  def name(self) -> str:
    """The name of this provider, which every model of it says as its system."""
    return API

  @property
  def base_url(self) -> str:
    """The url of this provider, which is none, since it spawns a binary and calls nothing."""
    return ""

  @property
  def client(self) -> object:
    """The client this provider hands out, which is its pool of conversations."""
    return self.pool


@dataclass
class Reply(StreamedResponse):
  """One turn of claude's stream, as the events pydantic_ai reads it through."""

  session: Session
  heard: AsyncIterator[Said]
  stamp: datetime = field(default_factory=lambda: datetime.now(tz=UTC))

  async def _get_event_iterator(self) -> AsyncIterator[ModelResponseStreamEvent]:
    seen = 0
    async for said in self.heard:
      if said.type == "assistant" and said.message is not None:
        # A message-start snapshot: the input and cache counts are real, the output count is not, and the result event
        # replaces the whole of it with the turn's true total.
        self._usage = spent(said.message.usage, None) if said.message.usage is not None else self._usage
        for block in said.message.content:
          seen += 1
          if block.type == "text":
            for event in self._parts_manager.handle_text_delta(vendor_part_id=seen, content=block.text):
              yield event
          elif block.type == "thinking":
            for event in self._parts_manager.handle_thinking_delta(
              vendor_part_id=seen, content=block.thinking, signature=block.signature, provider_name=API
            ):
              yield event
      elif said.type == "result":
        self._usage = spent(said.usage, self.session.charge(said.total_cost_usd))
        self.provider_response_id = said.session_id or None
        if said.is_error:
          wrong(self.model_name, said.result or "claude reported an error")
        # The result text is a fallback for the rare turn that ends without having narrated anything at all.
        if not self._parts_manager.get_parts() and said.result:
          for event in self._parts_manager.handle_text_delta(vendor_part_id=seen, content=said.result):
            yield event
        self.finish_reason = STOP.get(said.stop_reason or "", "stop")
        self.session.settled(self.get())

  async def close_stream(self) -> None:
    """Killing the process is the only way to stop claude mid-turn, and its conversation survives the killing."""
    await self.session.stop()

  @property
  def model_name(self) -> str:
    """The model this turn was bought from."""
    return self.session.seat.name

  @property
  def provider_name(self) -> str:
    """The provider this turn was bought from."""
    return API

  @property
  def provider_url(self) -> str:
    """The url of the provider, which is none."""
    return ""

  @property
  def timestamp(self) -> datetime:
    """When this turn began."""
    return self.stamp


class Claude(Model[object]):
  """Completions bought from the `claude` CLI, where a subscription pays and no API key is ever read.

  The system prompt is not the whole prompt: the CLI adds its own ambient context to every session it starts, so two
  requests are the same request here when their messages, model, effort and system prompt agree.
  """

  def __init__(
    self,
    name: str = "sonnet",
    cli: Cli | None = None,
    settings: ModelSettings | None = None,
    profile: ModelProfileSpec | None = None,
  ) -> None:
    self.name = name
    self.window, self.tokens = limits(name)
    self.cli = default() if cli is None else cli
    self._provider = self.cli
    super().__init__(settings=settings, profile=PROFILE if profile is None else profile)

  @property
  def model_name(self) -> str:
    """The model a request of this one is bought from."""
    return self.name

  @property
  def system(self) -> str:
    """The provider this model belongs to."""
    return API

  def preface(self, messages: Sequence[ModelMessage]) -> str:
    """The system prompt a process spawns with: the run's instructions, then every system part the history carries."""
    told = [
      p.content for m in messages if isinstance(m, ModelRequest) for p in m.parts if isinstance(p, SystemPromptPart)
    ]
    last = next((m.instructions for m in reversed(messages) if isinstance(m, ModelRequest) and m.instructions), "")
    return "\n\n".join([last, *told] if last else told)

  async def request(
    self,
    messages: list[ModelMessage],
    model_settings: ModelSettings | None,
    model_request_parameters: ModelRequestParameters,
  ) -> ModelResponse:
    """One turn, whole: the stream is drained and what it came to is the response."""
    async with self.request_stream(messages, model_settings, model_request_parameters) as reply:
      async for _ in reply:
        pass
      return reply.get()

  @asynccontextmanager
  async def request_stream(
    self,
    messages: list[ModelMessage],
    model_settings: ModelSettings | None,
    model_request_parameters: ModelRequestParameters,
    run_context: RunContext[object] | None = None,
  ) -> AsyncGenerator[StreamedResponse]:
    """One turn as a stream, on the session that holds this conversation."""
    settings, params = self.prepare_request(model_settings, model_request_parameters)
    if params.function_tools or params.output_tools or params.native_tools:
      wrong(self.name, "this provider bridges no tools: it buys completions, and the CLI it spawns is given none")
    sid, fork, effort = mine(settings)
    if effort and effort not in LEVELS:
      wrong(self.name, f"claude spends one of {', '.join(LEVELS)} on a turn, never {effort}")
    # A run is a conversation unless the caller names one, so parallel agent runs land on processes of their own.
    seat = Seat(
      self.name,
      effort or effort_of(params.thinking),
      self.preface(messages),
      sid or (run_context.run_id or "" if run_context else ""),
    )
    session = await self.cli.pool.route(seat, fork, messages)
    async with session.turn(messages) as heard:
      yield Reply(model_request_parameters=params, session=session, heard=heard)
