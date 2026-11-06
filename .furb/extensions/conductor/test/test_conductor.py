"""The conductor, on the real engine: topics with a chain each, items as prompts in a room by class, one aggregator, fixes of CI before items, the reground of a topic chain, and reviews, audits and QA on a clock."""

import re
from pathlib import Path

import pytest

from conftest import said, settle
from extensions.conftest import Bench
from furb.engine import OPERATOR

HERE = Path(__file__).resolve().parent.parent
SONNET, OPUS, OPUS_M = "claude-cli:sonnet/high", "claude-cli:opus/high", "claude-cli:opus/medium"
ROSTER = [
  [OPERATOR, [], 200000],
  ["claude-cli:sonnet", ["medium", "high"], 1000000],
  ["claude-cli:opus", ["medium", "high"], 1000000],
  ["codex:gpt-6-astra", ["high"], 258400],
]
TEST = '''width.clear()
width.update({"a": 2, "b": 1, "qa": 1, "review": 2})
REPO = cwd()
state.update(review_at=1e12, qa_at=1e12)


def item(key, model="claude-cli:sonnet/high", after=()):
  return {"key": key, "model": model, "brief": "do " + key, "after": list(after)}


def stepped(step, *args):
  def ear(id):
    yield "started", id
    got = yield from step(*args)
    yield "done", id, got

  return act("stepped", "", controlled(ear))
'''


class Round(Bench):
  """The Bench of the conductor and the compaction that it uses, with the roster of the test."""

  def __init__(self, tmp: Path) -> None:
    super().__init__(tmp, [HERE / "../../../extensions/compaction/compaction.py", HERE / "conductor.py"], ROSTER, OPUS)

  async def beat(self) -> None:
    await self.value("await stepped(beat)")

  async def end(self, name: str, value: object) -> None:
    self.engine.close(value, name)
    await settle()

  def asked(self, kind: str = "str") -> list[tuple[str, str, str, str]]:
    """Each prompt to a model: its id, the label of its chain, its model and its message."""
    made = [x for x in said(self.log, "prompt") if x[4] == kind and x[6] != OPERATOR]
    return [(str(x[1]), str(self.engine.get(x[3])[4]), str(x[6]), str(x[5])) for x in made]

  def notices(self) -> list[str]:
    return [str(x[5]) for x in said(self.log, "prompt") if x[3] == self.root and x[4] == "None"]


@pytest.fixture
async def conductor(tmp_path: Path) -> Round:
  made = Round(tmp_path)
  await made.load(TEST)
  return made


async def test_an_item_goes_to_its_model_on_the_one_chain_of_its_topic_within_its_width(conductor: Round) -> None:
  await conductor.word('queue("a").extend([item("one"), item("two"), item("three")])')
  await conductor.beat()
  assert [(label, model) for _, label, model, _ in conductor.asked()] == [("a", SONNET), ("a", SONNET)]
  assert [x["key"] for x in conductor.ns["queues"]["a"]] == ["three"]
  assert conductor.asked()[0][3].startswith("Item one of the topic a. The charter of this chain, scratch://converge/a.md")


async def test_an_item_waits_for_its_after_and_for_a_seat_of_its_class(conductor: Round) -> None:
  await conductor.word('ROOM["opus/high"] = 1\nqueue("a").extend([item("late", after=["first"]), item("x", "claude-cli:opus/high")])\nqueue("b").append(item("y", "claude-cli:opus/high"))')
  await conductor.beat()
  assert [x[3].split(".")[0] for x in conductor.asked()] == ["Item x of the topic a"]
  assert conductor.notices() == ["Conductor:\nb holds 1 queued items and runs none: no item has its after done and a seat of its class free"]
  await conductor.beat()
  assert len(conductor.notices()) == 1


async def test_an_ended_item_tells_the_root_and_frees_its_place_for_the_next(conductor: Round) -> None:
  await conductor.word('queue("b").extend([item("one"), item("two", after=["one"])])')
  await conductor.beat()
  (first, *_), = conductor.asked()
  await conductor.end(first, "done one")
  await conductor.beat()
  assert f"b: the item one ({first}, {SONNET}) ended: done one" in conductor.notices()[-1]
  assert [x[3].split(".")[0] for x in conductor.asked()] == ["Item one of the topic b", "Item two of the topic b"]


async def test_a_fix_of_ci_goes_before_the_items_of_its_topic_and_waits_for_a_seat_of_the_fixer(conductor: Round) -> None:
  await conductor.word('ROOM["sonnet/high"] = 0\nqueue("b").append(item("one", "claude-cli:opus/medium"))\nfix("b", "red at abc")')
  assert conductor.ns["fixwait"] == {"b": "red at abc"}
  await conductor.beat()
  assert conductor.asked() == []
  await conductor.word('ROOM["sonnet/high"] = 1')
  await conductor.beat()
  ((fixed, label, model, message),) = conductor.asked()
  assert (label, model, message.endswith("The brief of the aggregator:\n\nred at abc")) == ("b", SONNET, True)
  await conductor.end(fixed, "fixed at def")
  await conductor.beat()
  assert f"b ended the fix {fixed} of CI: fixed at def" in conductor.notices()[-1]
  assert conductor.asked()[-1][3].startswith("Item one of the topic b")


async def test_a_cycle_of_the_aggregator_starts_on_a_new_print_and_sends_each_failure_to_its_topic(conductor: Round) -> None:
  conductor.say(r"pr list --head w/converge", [{"number": 7}])
  conductor.say(r"pr checks 7", [{"bucket": "pass"}, {"bucket": "fail"}])
  await conductor.beat()
  ((cycle, label, model, message),) = conductor.asked("tuple[dict[str, str], str]")
  assert (label, model, message.startswith("One cycle of the aggregator.")) == ("converge", OPUS_M, True)
  assert "pr 7\nfail 1, pass 1" in conductor.ns["state"]["print"]
  await conductor.end(cycle, ({"a": "test x fails"}, "merged b"))
  conductor.sand.now += 300
  await conductor.beat()
  assert conductor.asked()[-1][1:3] == ("a", SONNET)
  assert conductor.notices()[-1].endswith(f"cycle {cycle}: merged b Sent to a.")
  assert len(conductor.asked("tuple[dict[str, str], str]")) == 1
  conductor.say(r"pr checks 7", [{"bucket": "pending"}])
  conductor.sand.now += 300
  await conductor.beat()
  assert len(conductor.asked("tuple[dict[str, str], str]")) == 1
  conductor.sand.now += 900
  await conductor.beat()
  assert len(conductor.asked("tuple[dict[str, str], str]")) == 2


async def test_qa_runs_again_only_on_a_new_head_after_qa_every(conductor: Round) -> None:
  await conductor.word('state.update(head="p1", qa_at=-1e9, last=1e12)')
  await conductor.beat()
  qa = [x for x in conductor.asked() if x[1] == "qa"]
  assert [x[3].split(".")[0] for x in qa] == ["Item qa-run-1 of the topic qa"]
  await conductor.end(qa[0][0], "clean")
  conductor.sand.now += 900
  await conductor.word('state.update(head="p1")')
  await conductor.beat()
  assert len([x for x in conductor.asked() if x[1] == "qa"]) == 1
  await conductor.word('state.update(head="p2")')
  await conductor.beat()
  assert len([x for x in conductor.asked() if x[1] == "qa"]) == 2


async def test_reviews_and_audits_on_astra_come_each_hour_and_once_for_each_kind(conductor: Round) -> None:
  await conductor.word('state.update(review_at=-1e9, last=1e12)\nqueue("review").clear()\nwidth["review"] = 0')
  await conductor.beat()
  keys = [x["key"] for x in conductor.ns["queues"]["review"]]
  assert keys == ["words-audit-1", "align-audit-1", "perf-1", "review-a-1", "review-b-1"]
  assert {x["model"] for x in conductor.ns["queues"]["review"]} == {"codex:gpt-6-astra/high"}
  assert "## Values bound now" in conductor.sand.files["scratch://converge/desk.md"]
  conductor.sand.now += 1800
  await conductor.beat()
  assert len(conductor.ns["queues"]["review"]) == 5
  conductor.sand.now += 3600
  await conductor.beat()
  assert len(conductor.ns["queues"]["review"]) == 5


async def test_queue_rows_puts_each_row_of_a_known_topic_and_class_once(conductor: Round) -> None:
  rows = "- k1 | a | sonnet/high | x.py | Fix x.\nk2 | b | opus/medium | y.py | Fix y.\nk3 | qa | sonnet/high | z | no\nk4 | nowhere | sonnet/high | z | no\nk1 | a | sonnet/high | x.py | again\nshort | row"
  got = await conductor.value(f"queue_rows({rows!r}, 'review-a-1')")
  assert got == ["a:k1", "b:k2"]
  (one,) = conductor.ns["queues"]["a"]
  assert (one["model"], one["brief"]) == (SONNET, "A finding of review-a-1: Fix x. Paths: x.py. Judge it against w/converge first, then fix it at its cause as one commit, with a test or a scenario that fails before the fix. Judge the row against main first: it is a claim of an audit.")
  await conductor.word('queue_rows("k5 | a | opus/high | w | Fix w.", "r", before="k1")')
  assert [x["key"] for x in conductor.ns["queues"]["a"]] == ["k5", "k1"]


async def test_a_topic_chain_past_reground_keeps_its_reads_then_a_model_chooses_what_to_keep(conductor: Round) -> None:
  await conductor.word('chains["a"] = str(chain("a"))')
  topic = conductor.ns["chains"]["a"]
  conductor.sand.cost = (600000, 0, 0, 0, 0.0)
  conductor.sand.script[topic] = ['read("notes.md")', 'x = 2']
  (conductor.tmp / "notes.md").write_text("one\n", encoding="utf-8")
  conductor.engine.prompt(str, "work", to=SONNET, on=topic)
  await settle()
  await conductor.word('state["last"] = 1e12')
  await conductor.beat()
  assert "a (" + topic + ") read 60% of its window, so it was regrounded on its reads." in conductor.notices()[-1]
  await conductor.beat()
  assert conductor.ns["ground"]["a"].startswith("g:")
  await conductor.word('ground["a"] = "g:an-earlier-reply"')
  await conductor.beat()
  chose = conductor.ns["ground"]["a"].removeprefix("c:")
  assert conductor.engine.get(chose)[0] == "prompt"
  await conductor.end(chose, ([], "nothing more to keep"))
  await conductor.beat()
  assert "still read 60% of its window after its reground, so a cheap model chose the acts to keep (0)." in conductor.notices()[-1]


async def test_news_is_done_with_the_next_prompt_of_the_root(conductor: Round) -> None:
  await conductor.word("heard = news(600)")
  told = conductor.engine.prompt(str, "Next.", on=conductor.root)
  await settle()
  assert conductor.engine.peek(conductor.ns["heard"]) == told


async def test_words_writes_each_message_of_the_owner_in_order(conductor: Round) -> None:
  told = conductor.engine.prompt(str, "Do it.", on=conductor.root)
  await conductor.word("words()")
  assert f"## {told}, a message of the owner\nDo it." in conductor.sand.files["scratch://converge/words-life.md"]


async def test_conduct_beats_until_it_is_closed(conductor: Round) -> None:
  await conductor.word('queue("b").append(item("one"))\nheld = conduct()')
  assert [x[3].split(".")[0] for x in conductor.asked()] == ["Item one of the topic b"]
  assert re.fullmatch(r"conduct\d+", conductor.ns["state"]["id"])
  await conductor.word("close(None, held)")
  assert conductor.engine.over(conductor.ns["held"])


async def test_a_new_conduct_ends_the_old_one_so_one_beat_starts_a_cycle_and_the_workers_live_on(conductor: Round) -> None:
  conductor.say(r"pr list --head w/converge", [{"number": 7}])
  conductor.say(r"pr checks 7", [{"bucket": "pending"}])
  conductor.sand.hold = r"pr list --head"
  await conductor.word('queue("b").append(item("one"))\nfirst = conduct()')
  ((worker, *_),) = conductor.asked()
  conductor.say(r"pr checks 7", [{"bucket": "fail"}])
  await conductor.word("second = conduct()")
  conductor.sand.release()
  await settle()
  ((cycle, *_),) = conductor.asked("tuple[dict[str, str], str]")
  first, second = conductor.ns["first"], conductor.ns["second"]
  assert conductor.engine.over(first) and not conductor.engine.over(second)
  assert conductor.ns["state"]["id"] == second
  assert conductor.ns["state"]["cycle"] == cycle
  assert "fail 1" in conductor.ns["state"]["print"]
  assert not conductor.engine.over(worker)


async def test_a_failure_of_a_topic_whose_fix_runs_waits_for_the_next_fix_of_that_topic(conductor: Round) -> None:
  await conductor.word('state["last"] = 1e12\nfirst = fix("a", "test x fails at h1")')
  await conductor.word('state["cycle"] = str(prompt(tuple[dict[str, str], str], "c", to="claude-cli:opus/medium", on=chain_of("converge")))')
  await conductor.end(conductor.ns["state"]["cycle"], ({"a": "test y fails at h2"}, "merged b"))
  await conductor.beat()
  assert conductor.ns["fixwait"] == {"a": "test y fails at h2"}
  assert conductor.notices()[-1].endswith("merged b Kept for the next fix of a.")
  await conductor.end(conductor.ns["first"], "fixed x")
  await conductor.beat()
  assert conductor.asked()[-1][3].endswith("The brief of the aggregator:\n\ntest y fails at h2")
  assert conductor.ns["fixwait"] == {}


async def test_qa_runs_in_its_own_tree_at_the_published_head_and_a_cycle_merges_beside_it(conductor: Round) -> None:
  conductor.say(r"pr list --head w/converge", [{"number": 7}])
  conductor.say(r"pr checks 7", [{"bucket": "pending"}])
  conductor.say(r"pr view 7", {"headRefOid": "h2"})
  await conductor.word('state.update(head="h1", qa_at=-1e9, cycle_at=1e12, last=-1e9)')
  await conductor.word('state["cycle"] = str(prompt(tuple[dict[str, str], str], "c", to="claude-cli:opus/medium", on=chain_of("converge")))')
  await conductor.beat()
  assert [x for x in conductor.asked() if x[1] == "qa"] == []
  await conductor.end(conductor.ns["state"]["cycle"], ({}, "pushed h2"))
  await conductor.beat()
  assert conductor.ns["state"]["head"] == "h2"
  await conductor.beat()
  (qa,) = [x for x in conductor.asked() if x[1] == "qa"]
  assert "The head to test: h2." in qa[3]
  assert "script/worktree.sh ~/src/github.com/uael/furb-qa q/converge h2" in qa[3]
  conductor.say(r"pr checks 7", [{"bucket": "fail"}])
  conductor.sand.now += 1000
  await conductor.beat()
  cycles = conductor.asked("tuple[dict[str, str], str]")
  assert len(cycles) == 2
  conductor.say(r"pr view 7", {"headRefOid": "h3"})
  await conductor.end(cycles[-1][0], ({}, "pushed h3"))
  conductor.sand.now += 1000
  await conductor.beat()
  assert conductor.ns["state"]["head"] == "h3"
  await conductor.end(qa[0], "clean h2")
  if conductor.ns["state"]["cycle"]:
    await conductor.end(conductor.ns["state"]["cycle"], ({}, "pushed h3"))
  conductor.sand.now += 1000
  await conductor.beat()
  await conductor.beat()
  runs = [x for x in conductor.asked() if x[1] == "qa"]
  assert len(runs) == 2
  assert "The head to test: h3." in runs[-1][3]
  assert "q/converge h3" in runs[-1][3]
