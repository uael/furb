"""The claude command line provider, driven against a claude that answers from a script."""

import asyncio
import json
from collections.abc import Sequence
from decimal import Decimal
from pathlib import Path

import pytest
from pydantic_ai.messages import (
  BinaryContent,
  ImageUrl,
  ModelMessage,
  ModelRequest,
  ModelResponse,
  RetryPromptPart,
  SystemPromptPart,
  TextPart,
  ThinkingPart,
  ToolCallPart,
  ToolReturnPart,
  UserPromptPart,
)
from pydantic_ai.models import ModelRequestParameters
from pydantic_ai.tools import ToolDefinition

from furb.engine import OPERATOR, Refused
from furb.provider.claude import (
  ACTOR,
  API,
  FLOOR,
  LEVELS,
  LIVE,
  MILLION,
  Claude,
  Cli,
  Pool,
  Reply,
  Seat,
  Session,
  Settings,
  Spend,
  actors,
  alike,
  based,
  canon,
  cool,
  default,
  effort_of,
  limits,
  listed,
  mine,
  shown,
  spent,
  wrong,
)
from outside.doubles import ask, reply

PARAMS = ModelRequestParameters()


def seat_of(sid: str = "", effort: str | None = None, said: str = "be terse") -> Seat:
  return Seat("haiku", effort, said, sid)


def argv(home: Path, n: int) -> list[str]:
  """What the nth process this test started was asked to be."""
  got = json.loads((home / f"argv-{n}.json").read_text())
  assert isinstance(got, list)
  return [str(x) for x in got]


def heard(home: Path) -> list[str]:
  """Every input line the fake was given, in order, as the text it read out of them."""
  return [str(json.loads(line)["text"]) for line in (home / "heard.jsonl").read_text().splitlines()]


async def bought(model: Claude, messages: Sequence[ModelMessage], settings: Settings | None = None) -> ModelResponse:
  return await model.request(list(messages), settings, PARAMS)


def test_a_model_knows_what_it_can_hold_and_how_much_it_may_write() -> None:
  assert limits("opus") == (MILLION, 128_000)
  assert limits("claude-haiku-4-5") == (200_000, 64_000)
  assert limits("claude-haiku-4-5-20251001") == (200_000, 64_000)
  assert limits("claude-nothing-9") == FLOOR
  assert limits("x") == FLOOR
  assert Claude("opus").window == MILLION
  assert Claude("opus").tokens == 128_000


def test_an_effort_is_spent_in_the_words_the_cli_speaks() -> None:
  assert effort_of("minimal") == "low"
  assert effort_of("medium") == "medium"
  assert effort_of("xhigh") == "xhigh"
  assert effort_of("max") == "max"
  assert effort_of(level=True) is None
  assert effort_of(None) is None
  assert effort_of("whenever") is None


def test_a_request_carries_three_settings_this_provider_reads_for_itself() -> None:
  assert mine(None) == ("", "", "")
  told: Settings = {"claude_session_id": "one", "claude_fork_of": "two", "claude_effort": "max", "temperature": 0.5}
  assert mine(told) == ("one", "two", "max")
  assert mine({"temperature": 0.5}) == ("", "", "")  # a setting this provider does not read for itself


def test_the_count_of_a_turn_holds_the_cache_inside_its_input() -> None:
  empty = spent(None, None)
  assert (empty.input_tokens, empty.output_tokens, empty.cost) == (0, 0, None)
  got = spent(Spend(input_tokens=10, output_tokens=5, cache_read_input_tokens=2, cache_creation_input_tokens=1), 0.25)
  assert (got.input_tokens, got.cache_read_tokens, got.cache_write_tokens) == (13, 2, 1)
  assert got.output_tokens == 5
  assert got.cost == Decimal("0.25")


def test_two_conversations_of_the_same_meaning_fingerprint_the_same() -> None:
  assert canon(ask("hello")) == canon(ask("hello"))
  assert canon(ask("hello")) != canon(ask("goodbye"))
  # A journalled reply carries an emptied thinking part where the live stream held one, and still matches.
  assert canon(reply("said")) == canon(
    ModelResponse(parts=[ThinkingPart(content="a whole thought"), TextPart(content="said")])
  )
  assert canon(ModelRequest(parts=[SystemPromptPart(content="a"), UserPromptPart(content="hi")])) == canon(ask("hi"))
  blob = BinaryContent(data=b"\x89PNG", media_type="image/png")
  assert canon(ModelRequest(parts=[UserPromptPart(content=["look", blob])])) == canon(
    ModelRequest(parts=[UserPromptPart(content=["look", blob])])
  )
  assert canon(ModelRequest(parts=[UserPromptPart(content=[ImageUrl(url="http://x/y.png")])])) != canon(ask("hi"))
  assert canon(ModelRequest(parts=[ToolReturnPart(tool_name="t", content="out", tool_call_id="1")])) != canon(
    ask("out")
  )
  assert canon(ModelRequest(parts=[RetryPromptPart(content="again", tool_call_id="1")])) != canon(ask("again"))
  assert canon(ModelResponse(parts=[ToolCallPart(tool_name="t", args={"a": 1}, tool_call_id="1")])) != canon(reply("t"))
  assert shown(blob).startswith("image/png:")
  assert "ImageUrl" in shown(ImageUrl(url="http://x/y.png"))


def test_a_request_continues_the_reply_the_last_ask_or_nothing_at_all() -> None:
  assert alike(["a", "b"], ["a", "b", "c"]) == 2
  assert alike(["a", "b"], ["z"]) == 0
  assert based([], [], ["a"]) == 0
  assert based(["a", "b"], ["a"], ["a", "b", "c"]) == 2
  assert based(["a", "b"], ["a"], ["a", "z"]) == 1
  assert based(["a", "b"], ["a"], ["z"]) is None
  assert based(["a", "b"], ["a", "b"], ["a"]) is None


def test_every_failure_of_a_turn_names_the_model_that_was_asked() -> None:
  with pytest.raises(Refused, match="haiku: something went wrong"):
    wrong("haiku", "something went wrong")


def test_the_provider_is_keyless_and_hands_out_the_pool_as_its_client() -> None:
  cli = Cli()
  assert (cli.name, cli.base_url) == (API, "")
  assert cli.client is cli.pool
  assert default() is default()
  assert Claude("haiku").cli is default()
  own = Cli(Pool(cap=1))
  assert Claude("haiku", own).cli is own
  assert Claude("haiku").system == API
  assert Claude("opus").model_name == "opus"
  assert ACTOR == "opus/low"
  assert [name for name, _, _ in actors()] == [OPERATOR, "fable", "opus", "sonnet", "haiku"]
  assert actors()[2][1] == list(LEVELS)
  assert actors()[2][2] == MILLION
  assert actors()[4][2] == 200_000


def test_the_system_prompt_is_the_instructions_and_every_system_part_after_them() -> None:
  model = Claude("haiku")
  assert model.preface([ask("hi")]) == ""
  told: list[ModelMessage] = [
    ModelRequest(parts=[SystemPromptPart(content="be terse"), UserPromptPart(content="hi")]),
    reply("ok"),
    ModelRequest(parts=[UserPromptPart(content="more")], instructions="say less"),
  ]
  assert model.preface(told) == "say less\n\nbe terse"


def test_the_delta_goes_out_as_one_line_whatever_it_holds() -> None:
  session = Session(seat_of())
  blob = BinaryContent(data=b"\x89PNG", media_type="image/png")
  told: list[ModelMessage] = [
    reply("an earlier answer"),
    ModelRequest(parts=[ToolReturnPart(tool_name="t", content="an observation", tool_call_id="1")]),
    ModelRequest(parts=[RetryPromptPart(content="that failed", tool_call_id="1")]),
    ModelRequest(parts=[UserPromptPart(content="an earlier ask")]),
    ModelRequest(parts=[UserPromptPart(content=["the live ask", blob])]),
  ]
  got = json.loads(session.line(told))
  assert got["type"] == "user"
  blocks = got["message"]["content"]
  assert [b["type"] for b in blocks] == ["text", "image"]
  assert "[earlier turn: you]\nan earlier answer" in blocks[0]["text"]
  assert "[earlier observation]\nan observation" in blocks[0]["text"]
  assert "that failed" in blocks[0]["text"]
  assert "an earlier ask" in blocks[0]["text"]
  assert "[earlier turn: user]" not in blocks[0]["text"]
  assert blocks[0]["text"].endswith("the live ask")
  assert blocks[1]["source"] == {"type": "base64", "media_type": "image/png", "data": "iVBORw=="}
  with pytest.raises(Refused, match="never ImageUrl"):
    session.line([ModelRequest(parts=[UserPromptPart(content=[ImageUrl(url="http://x/y.png")])])])
  with pytest.raises(Refused, match="nothing to send"):
    session.line([ModelRequest(parts=[SystemPromptPart(content="be terse")])])


async def test_a_turn_without_a_binary_or_a_process_fails_before_it_is_asked(monkeypatch: pytest.MonkeyPatch) -> None:
  monkeypatch.setenv("FURB_CLAUDE_BIN", "no-claude-of-that-name")
  session = Session(seat_of())
  with pytest.raises(Refused, match="no no-claude-of-that-name on PATH"):
    await session.spawn()
  with pytest.raises(Refused, match="claude is not running"):
    session.write([ask("hi")])
  session.hurt = "it said this on the way out"
  with pytest.raises(Refused, match="it said this on the way out"):
    session.write([ask("hi")])
  await session.stop()  # a session that never stood up still cools without complaint
  assert not session.warm


@pytest.mark.usefixtures("fake")
async def test_two_sweeps_at_once_do_not_trip_over_the_session_the_other_gave_up() -> None:
  """A sweep runs on every route, so two asks arriving together are two sweeps walking one pool. Each waits on
  the process it is giving up, and what it comes back to is a pool the other has been dropping from meanwhile."""
  pool = Pool(cap=1, keep=0, idle=0)
  for name in ("one", "two"):
    held = Session(seat_of(name))
    await held.spawn()
    pool.sessions[held.seat] = held
  await asyncio.gather(pool.sweep(seat_of("elsewhere")), pool.sweep(seat_of("elsewhere")))
  assert pool.sessions == {}


async def test_a_pool_forks_the_conversation_a_request_names_as_its_parent() -> None:
  pool = Pool()
  kin = Session(seat_of("one", "high"))
  kin.asked = ["a", "b"]
  pool.sessions[seat_of("one", "high")] = kin
  assert pool.forked(seat_of("two"), "", ["a", "b"]) is None
  assert pool.forked(seat_of("two"), "nowhere", ["a", "b"]) is None
  assert pool.forked(seat_of("two"), "one", ["z"]) is None
  kid = pool.forked(seat_of("two"), "one", ["a", "b", "c"])
  assert kid is not None
  assert (kid.chain, kid.parent, kid.mode) == (["a", "b"], kin.id, "branch")
  kin.busy = True
  assert pool.forked(seat_of("two"), "one", ["a", "b"]) is None


async def test_a_pool_branches_off_the_sibling_sitting_at_the_branch_point() -> None:
  pool = Pool()
  told: list[ModelMessage] = [ask("a"), ask("b"), ask("c")]
  short, long, late = Session(seat_of("one")), Session(seat_of("two")), Session(seat_of("four"))
  short.chain, long.chain, late.chain = ["a"], ["a", "b"], ["a"]
  pool.sessions[seat_of("one")], pool.sessions[seat_of("two")] = short, long
  pool.sessions[seat_of("four")] = late  # a match that is no better than the one already found
  kid = pool.tipped(seat_of("three"), told, ["a", "b", "c"])
  assert kid is not None
  assert (kid.chain, kid.parent) == (["a", "b"], long.id)
  assert pool.tipped(seat_of("three"), told, ["z", "b", "c"]) is None  # a prefix of nothing here
  del pool.sessions[seat_of("two")]
  del pool.sessions[seat_of("four")]  # leaving only the sibling one message in, so the reply below sits past its tip
  assert pool.tipped(seat_of("three"), [ask("a"), reply("b"), ask("c")], ["a", "b", "c"]) is None  # a historical edit
  assert pool.tipped(Seat("opus", None, "be terse", "three"), told, ["a", "b", "c"]) is None
  assert pool.tipped(Seat("haiku", None, "other words", "three"), told, ["a", "b", "c"]) is None
  assert pool.tipped(seat_of("three"), [], []) is None  # every chain here is longer than the ask
  short.busy = True
  assert pool.tipped(seat_of("three"), told, ["a", "b", "c"]) is None


async def test_a_pool_drops_the_coldest_dead_conversation_once_it_holds_too_many() -> None:
  pool = Pool(keep=1)
  for n in range(3):
    session = Session(seat_of(str(n)))
    session.used = float(n)
    pool.sessions[seat_of(str(n))] = session
  await pool.sweep(seat_of("2"))
  assert list(pool.sessions) == [seat_of("2")]


async def test_a_pool_cools_what_has_sat_idle_and_what_stands_over_the_cap(fake: Path) -> None:
  assert fake.is_dir()
  pool = Pool(cap=1, idle=0.0)
  live: list[Session] = []
  for n in range(3):
    session = Session(seat_of(str(n)))
    pool.sessions[seat_of(str(n))] = session
    await session.spawn()
    live.append(session)
  assert pool.hot() == 3
  await pool.sweep(seat_of("2"))
  assert [s.warm for s in live] == [False, False, True]
  await pool.close()
  assert pool.sessions == {}
  assert LIVE == []


async def test_a_turn_buys_a_completion_from_the_binary_it_spawns(fake: Path) -> None:
  model = Claude("haiku")
  told: list[ModelMessage] = [
    ModelRequest(parts=[SystemPromptPart(content="be terse"), UserPromptPart(content="say alpha")])
  ]
  got = await bought(model, told, {"claude_effort": "max"})
  assert got.text == "heard: say alpha"
  assert [p.part_kind for p in got.parts] == ["thinking", "text"]
  assert (got.finish_reason, got.provider_name, got.model_name) == ("stop", API, "haiku")
  assert (got.usage.input_tokens, got.usage.output_tokens, got.usage.cost) == (13, 5, Decimal("0.5"))
  said = argv(fake, 0)
  assert said[:6] == ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"]
  assert said[said.index("--model") + 1] == "haiku"
  assert said[said.index("--effort") + 1] == "max"
  assert said[said.index("--system-prompt") + 1] == "be terse"
  assert said[said.index("--tools") + 1] == ""
  assert "--strict-mcp-config" in said
  assert "--disable-slash-commands" in said
  assert said[said.index("--session-id") + 1] == got.provider_response_id
  assert heard(fake) == ["say alpha"]


async def test_the_second_turn_writes_only_what_claude_has_not_already_heard(fake: Path) -> None:
  model = Claude("haiku")
  told: list[ModelMessage] = [ask("say alpha")]
  one = await bought(model, told)
  session = next(iter(model.cli.pool.sessions.values()))
  child = session.child
  told += [one, ask("say beta")]
  two = await bought(model, told)
  assert two.text == "heard: say beta"
  assert session.child is child  # the same process, so nothing before this turn was uploaded again
  assert heard(fake) == ["say alpha", "say beta"]
  assert len(session.chain) == 4
  assert two.usage.cost == Decimal("0.5")  # the running total is 1.0, and half of it was paid for last turn


async def test_a_request_that_continues_nothing_gets_a_conversation_that_holds_nothing(fake: Path) -> None:
  model = Claude("haiku")
  await bought(model, [ask("say alpha")])
  session = next(iter(model.cli.pool.sessions.values()))
  was = session.id
  await bought(model, [ask("something else entirely")])
  assert session.id != was
  assert argv(fake, 1)[argv(fake, 1).index("--session-id") + 1] == session.id
  assert heard(fake) == ["say alpha", "something else entirely"]


async def test_a_duplicate_is_re_sent_once_into_the_conversation_and_then_given_up(fake: Path) -> None:
  model = Claude("haiku")
  told: list[ModelMessage] = [ask("say alpha")]
  await bought(model, told)
  session = next(iter(model.cli.pool.sessions.values()))
  was = session.id
  got = await bought(model, told)  # the same ask again: the live tail goes back into the same conversation
  assert got.text == "heard: say alpha"
  assert session.id == was
  assert heard(fake) == ["say alpha", "say alpha"]
  session.again = True  # a retry that failed once already: the conversation is past saving
  await bought(model, told)
  assert session.id != was


async def test_a_cooled_conversation_is_revived_by_resuming_it(fake: Path) -> None:
  model = Claude("haiku")
  told: list[ModelMessage] = [ask("say alpha")]
  one = await bought(model, told)
  session = next(iter(model.cli.pool.sessions.values()))
  await session.stop()
  assert not session.warm
  told += [one, ask("say beta")]
  await bought(model, told)
  assert argv(fake, 1)[argv(fake, 1).index("--resume") + 1] == session.id
  assert heard(fake) == ["say alpha", "say beta"]


async def test_a_fork_takes_the_parent_transcript_and_the_new_name_it_is_given(fake: Path) -> None:
  model = Claude("haiku")
  told: list[ModelMessage] = [ask("say alpha")]
  one = await bought(model, told, {"claude_session_id": "parent"})
  session = model.cli.pool.sessions[seat_of("parent", said="")]
  told += [one, ask("say beta")]
  two = await bought(model, told, {"claude_session_id": "child", "claude_fork_of": "parent"})
  assert two.text == "heard: say beta"
  said = argv(fake, 1)
  assert said[said.index("--resume") + 1] == session.id
  assert "--fork-session" in said
  kid = model.cli.pool.sessions[seat_of("child", said="")]
  assert kid.id == f"{session.id}-forked"  # the fork is told its own name by the first thing it says
  assert kid.mode == "resume"
  assert heard(fake) == ["say alpha", "say beta"]


@pytest.mark.usefixtures("fake")
async def test_a_silence_ends_the_turn_it_was_holding(monkeypatch: pytest.MonkeyPatch) -> None:
  monkeypatch.setenv("FURB_FAKE_MODE", "hush")
  monkeypatch.setenv("FURB_CLAUDE_STALL", "1.0")
  model = Claude("haiku")
  with pytest.raises(Refused, match="said nothing this could read for 1s with a turn in flight"):
    await bought(model, [ask("say alpha")])
  assert LIVE == []  # the process it was holding was killed with the turn


@pytest.mark.usefixtures("fake")
async def test_a_line_too_long_to_read_ends_the_turn_it_arrived_in(monkeypatch: pytest.MonkeyPatch) -> None:
  """The reader holds one line at a time, so a line past that is one it can neither finish nor pick up after:
  what follows arrives as the tail of something it never saw the start of. Left to the stall it reads as a
  process that went quiet, which is the one thing it did not do."""
  monkeypatch.setattr("furb.provider.claude.ROOM", 4096)
  monkeypatch.setenv("FURB_FAKE_MODE", "flood")
  model = Claude("haiku")
  with pytest.raises(Refused, match="past the 4096 bytes"):
    await bought(model, [ask("say alpha")])


@pytest.mark.usefixtures("fake")
async def test_a_silence_full_of_lines_it_could_not_read_says_the_last_of_them(monkeypatch: pytest.MonkeyPatch) -> None:
  """A stream this cannot read stalls exactly like a process that went quiet, and the two are not the same failure.
  Told only of the silence, whoever reads the message goes looking at the network for a fault in this parser."""
  monkeypatch.setenv("FURB_FAKE_MODE", "deaf")
  monkeypatch.setenv("FURB_CLAUDE_STALL", "1.0")
  model = Claude("haiku")
  with pytest.raises(Refused, match="could not read was this is not a line the stream ever wrote"):
    await bought(model, [ask("say alpha")])


async def test_a_process_that_dies_fails_the_turn_with_what_it_said_on_the_way_out(
  monkeypatch: pytest.MonkeyPatch, fake: Path
) -> None:
  assert fake.is_dir()
  monkeypatch.setenv("FURB_FAKE_MODE", "boom")
  model = Claude("haiku")
  with pytest.raises(Refused, match="claude exited \\(3\\): the fake refused to run"):
    await bought(model, [ask("say alpha")])
  session = next(iter(model.cli.pool.sessions.values()))
  assert not session.warm
  assert session.chain == session.asked  # a failed turn keeps no reply


async def test_a_process_that_stops_mid_turn_fails_it_too(monkeypatch: pytest.MonkeyPatch, fake: Path) -> None:
  assert fake.is_dir()
  monkeypatch.setenv("FURB_FAKE_MODE", "gone")
  model = Claude("haiku")
  with pytest.raises(Refused, match="claude exited"):
    await bought(model, [ask("say alpha")])


async def test_an_error_the_stream_reports_is_the_turn_failing(monkeypatch: pytest.MonkeyPatch, fake: Path) -> None:
  assert fake.is_dir()
  monkeypatch.setenv("FURB_FAKE_MODE", "sour")
  model = Claude("haiku")
  with pytest.raises(Refused, match="the fake reports an error"):
    await bought(model, [ask("say alpha")])


async def test_a_line_the_stream_never_wrote_is_stepped_over(monkeypatch: pytest.MonkeyPatch, fake: Path) -> None:
  assert fake.is_dir()
  monkeypatch.setenv("FURB_FAKE_MODE", "junk")
  model = Claude("haiku")
  got = await bought(model, [ask("say alpha")])
  assert got.text == "heard: say alpha"


async def test_a_turn_that_narrated_nothing_still_says_what_it_ended_with(
  monkeypatch: pytest.MonkeyPatch, fake: Path
) -> None:
  assert fake.is_dir()
  monkeypatch.setenv("FURB_FAKE_MODE", "bare")
  model = Claude("haiku")
  got = await bought(model, [ask("say alpha")])
  assert got.text == "said nothing, ended anyway"
  assert got.finish_reason == "length"
  assert (got.usage.input_tokens, got.usage.cost) == (0, None)


async def test_a_stream_hands_every_word_over_as_it_lands(fake: Path) -> None:
  assert fake.is_dir()
  model = Claude("haiku")
  async with model.request_stream([ask("say alpha")], None, PARAMS) as stream:
    assert isinstance(stream, Reply)
    assert (stream.model_name, stream.provider_name, stream.provider_url) == ("haiku", API, "")
    assert stream.timestamp.tzinfo is not None
    kinds = [event.event_kind async for event in stream]
    got = stream.get()
    await stream.close_stream()
  assert got.text == "heard: say alpha"
  assert kinds[0] == "part_start"
  assert LIVE == []


async def test_a_run_is_a_conversation_of_its_own_unless_the_caller_names_one(fake: Path) -> None:
  assert fake.is_dir()
  model = Claude("haiku")
  told: list[ModelMessage] = [ask("say alpha")]
  await bought(model, told)
  await bought(model, told, {"claude_session_id": "named"})
  assert sorted(k.sid for k in model.cli.pool.sessions) == ["", "named"]


async def test_a_model_refuses_what_it_cannot_bridge_before_it_spawns_anything() -> None:
  model = Claude("haiku")
  tooled = ModelRequestParameters(function_tools=[ToolDefinition(name="t", description="", parameters_json_schema={})])
  with pytest.raises(Refused, match="bridges no tools"):
    await model.request([ask("hi")], None, tooled)
  bad: Settings = {"claude_effort": "whenever"}
  with pytest.raises(Refused, match="never whenever"):
    await model.request([ask("hi")], bad, PARAMS)
  assert LIVE == []


async def test_nothing_this_module_started_outlives_the_run_that_asked_for_it(fake: Path) -> None:
  assert fake.is_dir()
  session = Session(seat_of())
  await session.spawn()
  assert len(LIVE) == 1
  assert LIVE[0] is session
  assert session.warm
  listed(session, on=False)
  listed(session, on=False)  # a second forgetting is not an error
  assert LIVE == []
  listed(session, on=True)
  await cool()
  assert LIVE == []
  assert not session.warm


def test_a_settled_turn_moves_the_session_past_its_own_reply() -> None:
  session = Session(seat_of())
  session.settled(reply("said"))
  assert session.done == canon(reply("said"))
  assert session.charge(None) is None
  assert session.charge(1.0) == 1.0
  assert session.charge(1.5) == 0.5
  assert session.charge(0.5) == 0.0  # a fresh process restarts the count, and no turn is ever paid backwards


async def test_two_turns_arriving_at_once_are_served_one_after_the_other(fake: Path) -> None:
  """The engine runs repls side by side, and every one of them lands on the same key unless the caller says otherwise."""
  model = Claude("haiku")
  one, two = [ask("say alpha")], [ask("say beta")]
  first, second = await asyncio.gather(bought(model, one), bought(model, two))
  assert first.text == "heard: say alpha"  # neither turn is answered with the other's reply
  assert second.text == "heard: say beta"
  assert len(model.cli.pool.sessions) == 1
  assert heard(fake) == ["say alpha", "say beta"]


async def test_a_conversation_of_its_own_keeps_two_holes_out_of_one_process(fake: Path) -> None:
  model = Claude("haiku")
  mine: Settings = {"claude_session_id": "3"}
  theirs: Settings = {"claude_session_id": "106"}
  one, two = [ask("say alpha")], [ask("say beta")]
  first, second = await asyncio.gather(bought(model, one, mine), bought(model, two, theirs))
  assert first.text == "heard: say alpha"
  assert second.text == "heard: say beta"
  assert sorted(k.sid for k in model.cli.pool.sessions) == ["106", "3"]
  assert len({s.id for s in model.cli.pool.sessions.values()}) == 2
  assert sorted(heard(fake)) == ["say alpha", "say beta"]


async def test_a_second_turn_cannot_take_the_process_the_first_is_waiting_on(
  fake: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
  """Two repls open side by side land on one key, and the later one must not write into the turn the earlier one holds."""
  monkeypatch.setenv("FURB_FAKE_MODE", "slow")
  monkeypatch.setenv("FURB_CLAUDE_STALL", "20.0")
  model = Claude("haiku")
  told: list[ModelMessage] = [ask("say alpha")]
  told += [await bought(model, told)]
  beta, gamma = await asyncio.gather(bought(model, [*told, ask("say beta")]), bought(model, [*told, ask("say gamma")]))
  assert beta.text == "heard: say beta"  # the first turn is answered with its own reply, and waits on no dead queue
  assert (gamma.text or "").endswith(
    "say gamma"
  )  # the second diverges, so it re-sends its whole conversation into a new one
  assert heard(fake)[1] == "say beta"
