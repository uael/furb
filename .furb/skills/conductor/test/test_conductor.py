"""The conductor, on the real engine: one test for each rule of the round."""

from pathlib import Path

import pytest

from conftest import said, settle
from skills.conftest import Bench
from furb.engine import OPERATOR

HERE = Path(__file__).resolve().parent.parent
SONNET, SONNET_M, OPUS, OPUS_M = "claude-cli:sonnet/high", "claude-cli:sonnet/medium", "claude-cli:opus/high", "claude-cli:opus/medium"
ROSTER = [
  [OPERATOR, [], 200000],
  ["claude-cli:sonnet", ["medium", "high", "xhigh"], 1000000],
  ["claude-cli:opus", ["medium", "high", "xhigh"], 1000000],
  ["codex:gpt-6-astra", ["high"], 258400],
]
TEST = """TOPICS[:] = ["a", "b", "tidy", "review", "qa"]
for t in TOPICS:
  desk_queue.setdefault(t, [])
  desk_live.setdefault(t, {})
desk_width.clear()
desk_width.update({"a": 2, "b": 1, "tidy": 1, "review": 2, "qa": 1})
CODE = ("a", "b")
FINGER = "cat finger.txt 2>/dev/null; true"
CI_PROBE = "cat ci.txt 2>/dev/null; true"
DF = "cat df.txt 2>/dev/null; true"
MEM = "cat mem.txt 2>/dev/null; exit $(cat code.txt 2>/dev/null || echo 0)"
desk_time.update(review_at=1e12, ci_at=1e12, disk_at=1e12)
SIDE = []


def item(key, model="claude-cli:sonnet/high", after=(), p=None):
  return {"key": key, "model": model, "brief": (f"P{p}. " if p is not None else "") + "do " + key, "after": list(after)}


def stepped(step, *args):
  def ear(id):
    yield "started", id
    got = yield from step(*args)
    yield "done", id, got

  return act("stepped", "", controlled(ear))
"""


class Round(Bench):
  """The Bench of the conductor and the compaction that it uses, with the roster of the test."""

  def __init__(self, tmp: Path) -> None:
    super().__init__(tmp, [HERE / "../../../.agents/skills/compaction/compaction.py", HERE / "conductor.py"], ROSTER, OPUS)

  async def beat(self) -> None:
    await self.value("await stepped(desk_beat, 300.0, 900.0)")

  async def end(self, name: str, value: object) -> None:
    self.engine.close(value, name)
    await settle()

  def file(self, name: str, text: str) -> None:
    (self.tmp / name).write_text(text, encoding="utf-8")

  def asked(self, kind: str = "str") -> list[tuple[str, str, str, str]]:
    """Each prompt to a model: its id, the label of its chain, its model and its message."""
    made = [x for x in said(self.log, "prompt") if x[4] == kind and x[6] != OPERATOR]
    return [(str(x[1]), str(self.engine.get(x[3])[4]), str(x[6]), str(x[5])) for x in made]

  def keys(self, label: str = "") -> list[str]:
    return [x[3].split(" of the topic")[0].removeprefix("Item ") for x in self.asked() if x[3].startswith("Item ") and (x[1] == label or not label)]

  def notes(self, on: str) -> list[str]:
    return [str(x[5]) for x in said(self.log, "prompt") if x[3] == on and x[4] == "None"]

  def queue(self, t: str) -> list[str]:
    return [str(x["key"]) for x in self.ns["desk_queue"][t]]


@pytest.fixture
async def conductor(tmp_path: Path) -> Round:
  made = Round(tmp_path)
  await made.load(TEST)
  return made


async def test_an_item_goes_to_its_model_on_the_chain_of_its_topic_within_its_width(conductor: Round) -> None:
  await conductor.word('desk_queue["a"].extend([item("one"), item("two"), item("three")])')
  await conductor.beat()
  assert [(label, model) for _, label, model, _ in conductor.asked()] == [("a", SONNET), ("a", SONNET)]
  assert conductor.queue("a") == ["three"]
  assert conductor.asked()[0][3].startswith("Item one of the topic a. Follow scratch://converge/method.md")


async def test_the_hardest_ready_item_of_any_topic_goes_first_a_hot_fix_before_it_and_polish_last(conductor: Round) -> None:
  await conductor.word('ROOM_CAP["sonnet/high"] = 1\nPOLISH_KEYS.append("shine")\ndesk_queue["a"].extend([item("shine"), item("plain"), item("hard", p=1)])\ndesk_queue["b"].append(item("hot", p=0))')
  assert await conductor.value("[priority(x) for x in desk_queue['a']]") == [6, 4, 1]
  for _ in range(4):
    await conductor.beat()
    (live,) = [p for t in ("a", "b") for p in conductor.ns["desk_live"][t]]
    await conductor.end(live, "done")
  assert conductor.keys() == ["hot", "hard", "plain", "shine"]


async def test_an_item_starts_once_each_key_of_its_after_list_is_merged_and_engine_changes_run_in_their_order(conductor: Round) -> None:
  await conductor.word('SERIAL2[:] = ["e1", "e2"]\ndesk_queue["b"].append(item("one"))\ndesk_queue["a"].extend([item("two", after=["one"]), item("e2"), item("e1", p=5)])')
  await conductor.beat()
  assert conductor.keys() == ["one", "e1"]
  (one,) = conductor.ns["desk_live"]["b"]
  await conductor.end(one, "done one")
  (e1,) = [p for p in conductor.ns["desk_live"]["a"]]
  await conductor.end(e1, "done e1")
  await conductor.beat()
  assert conductor.keys() == ["one", "e1"]
  await conductor.word("desk_time['merged_upto'] = clock()")
  await conductor.beat()
  assert sorted(conductor.keys()[2:]) == ["e2", "two"]


async def test_a_medium_item_takes_a_medium_seat_and_moves_up_only_when_none_is_free_and_no_item_of_the_class_above_waits(conductor: Round) -> None:
  await conductor.word('ROOM_CAP.update({"sonnet/medium": 1, "sonnet/high": 1})\ndesk_width["a"] = 3\ndesk_queue["a"].extend([item("m1", SONNET_M), item("m2", SONNET_M), item("h1", after=["later"])])')
  await conductor.beat()
  assert [(k, x[2]) for k, x in zip(conductor.keys(), conductor.asked(), strict=True)] == [("m1", SONNET_M), ("m2", SONNET)]
  await conductor.word('desk_queue["a"].extend([item("m3", SONNET_M), item("h2")])\nROOM_CAP["sonnet/high"] = 2')
  await conductor.beat()
  assert conductor.keys()[2:] == ["h2"]
  assert conductor.queue("a") == ["h1", "m3"]


async def test_items_past_a_room_that_shrank_pause_newest_first_and_resume_when_a_seat_frees_unless_the_operator_paused_them(conductor: Round) -> None:
  await conductor.word('desk_width["a"] = 3\ndesk_queue["a"].extend([item("one"), item("two"), item("three")])')
  await conductor.beat()
  one, two, three = [x[0] for x in conductor.asked()]
  await conductor.word('ROOM_CAP["sonnet/high"] = 1\ndesk_queue["b"].append(item("four"))')
  await conductor.beat()
  assert [conductor.engine.paused(x) for x in (one, two, three)] == [False, True, True]
  await conductor.word(f"pause({three!r})")
  await conductor.end(one, "done")
  await conductor.beat()
  assert (conductor.engine.paused(two), conductor.engine.over(two), conductor.engine.paused(three)) == (False, False, True)
  assert conductor.keys() == ["one", "two", "three"]


async def test_a_fix_of_ci_pauses_the_live_items_of_its_topic_takes_each_new_failure_and_wakes_only_them(conductor: Round) -> None:
  await conductor.word('desk_queue["a"].extend([item("one"), item("two")])')
  await conductor.beat()
  one, two = [x[0] for x in conductor.asked()]
  await conductor.word(f"pause({two!r})\nfix = start_fix('a', 'red at h1')")
  fix = conductor.ns["fix"]
  assert conductor.engine.paused(one) and not conductor.engine.paused(fix)
  assert conductor.asked()[-1][1:] == ("a", SONNET, conductor.ns["FIX_HEAD"] + "red at h1")
  await conductor.word("start_fix('a', 'red at h2')\ndesk_queue['a'].append(item('three'))")
  assert conductor.notes(fix) == ["A new failure of the CI of w/converge for this fix:\n\nred at h2"]
  await conductor.beat()
  assert conductor.keys() == ["one", "two"]
  topic = conductor.ns["topic_chain"]["a"]
  conductor.sand.script[topic] = ['close("fixed at h3")']
  for held in [x[1] for x in said(conductor.log, "reply") if x[3] == topic and not conductor.engine.over(x[1])]:
    conductor.engine.say("done", held, ("assistant", "x = 1", (0, 0, 0, 0, 0.0), ["signed 5"]))
  await settle()
  assert conductor.engine.peek(fix) == "fixed at h3"
  await conductor.beat()
  assert (conductor.engine.paused(one), conductor.engine.paused(two)) == (False, True)
  assert "a ended the fix" in [x for x in conductor.ns["desk_notes"] if not x.startswith("Free seats")][-1]


async def test_a_cycle_starts_on_a_new_print_a_floor_after_the_start_of_the_last_and_sends_each_failure_to_its_topic(conductor: Round) -> None:
  conductor.file("finger.txt", "pr 7\nfail 1\nhead h1\n")
  await conductor.beat()
  ((cycle, label, model, message),) = conductor.asked("tuple[dict[str, str], str]")
  assert (label, model, message) == ("converge", OPUS_M, conductor.ns["CYCLE"])
  conductor.sand.now += 1000
  conductor.file("finger.txt", "pr 7\nfail 1\nhead h2\n")
  await conductor.end(cycle, ({"a": "test x fails", "nowhere": "y"}, "merged b"))
  await conductor.beat()
  assert [x[2] for x in conductor.asked() if x[1] == "a"] == [SONNET]
  assert conductor.ns["desk_notes"][0] == f"cycle {cycle}: merged b Sent to a."
  assert len(conductor.asked("tuple[dict[str, str], str]")) == 2
  conductor.file("finger.txt", "pr 7\nfail 2\nhead h3\n")
  conductor.sand.now += 600
  await conductor.end(conductor.asked("tuple[dict[str, str], str]")[-1][0], ({}, "merged a"))
  await conductor.beat()
  assert len(conductor.asked("tuple[dict[str, str], str]")) == 2
  conductor.sand.now += 300
  await conductor.beat()
  assert len(conductor.asked("tuple[dict[str, str], str]")) == 3


async def test_each_new_head_counts_one_merge_and_each_fourth_queues_one_audit_of_alignment_by_turns_across_a_restart(conductor: Round) -> None:
  await conductor.word("desk_time.clear()\ndesk_time['align_mark'] = 23.0")
  await conductor.load(TEST)
  for n in range(5):
    await conductor.word(f"merges_step('head h{n}')\nmerges_step('head h{n}')")
  assert [(x["key"], x["model"]) for x in conductor.ns["desk_queue"]["review"]] == [("align-audit-m27", "codex:gpt-6-astra/high")]
  for n in range(5, 13):
    await conductor.word(f"merges_step('head h{n}')")
  assert len(conductor.ns["desk_queue"]["review"]) == 1
  await conductor.load(TEST)
  await conductor.word("desk_queue['review'].clear()\nmerges_step('head h13')")
  assert [(x["key"], x["model"]) for x in conductor.ns["desk_queue"]["review"]] == [("align-audit-m36", OPUS)]


async def test_each_cycle_that_ends_starts_one_qa_run_when_no_cycle_runs_and_a_new_cycle_tells_it_to_wrap_up(conductor: Round) -> None:
  conductor.file("finger.txt", "head h1\n")
  await conductor.beat()
  ((cycle, *_),) = conductor.asked("tuple[dict[str, str], str]")
  assert conductor.keys("qa") == []
  await conductor.end(cycle, ({}, "pushed h1"))
  await conductor.beat()
  ((qa, *_),) = [x for x in conductor.asked() if x[1] == "qa"]
  conductor.file("finger.txt", "head h2\n")
  conductor.sand.now += 900
  await conductor.beat()
  assert conductor.notes(qa) == [conductor.ns["QA_WRAP"]]
  await conductor.end(qa, "rows")
  await conductor.end(conductor.asked("tuple[dict[str, str], str]")[-1][0], ({}, "pushed h2"))
  await conductor.beat()
  assert conductor.keys("qa") == ["qa-run-1", "qa-run-2"]


async def test_a_failed_run_of_the_gates_is_routed_once_to_its_topics_and_a_green_run_is_told(conductor: Round) -> None:
  conductor.file("ci.txt", "abc completed failure 11\n")
  await conductor.word("desk_time['ci_at'] = -1e9")
  await conductor.beat()
  ((route, label, model, _),) = conductor.asked("tuple[dict[str, str], str]")
  assert (label, model) == ("converge", OPUS_M)
  await conductor.end(route, ({"b": "test y fails"}, "one failure"))
  await conductor.word("desk_time['ci_at'] = -1e9")
  await conductor.beat()
  assert conductor.asked()[-1][1] == "b" and len(conductor.asked("tuple[dict[str, str], str]")) == 1
  conductor.file("ci.txt", "abd completed success 12\n")
  await conductor.word("desk_time['ci_at'] = -1e9")
  await conductor.beat()
  assert "The checks of CI of w/converge pass at abd (run 12)." in conductor.notes(conductor.root)[-1]


async def test_under_the_floor_of_the_disk_tidy_frees_the_oldest_ended_worktree_first_once(conductor: Round) -> None:
  conductor.file("df.txt", f"/dev/disk 1 1 {50 * 1048576} 99% /\n")
  await conductor.word("desk_queue['a'].append(item('one'))\ndesk_time['disk_at'] = -1e9")
  await conductor.beat()
  await conductor.word("desk_time['disk_at'] = -1e9")
  await conductor.beat()
  ((_, label, model, message),) = [x for x in conductor.asked() if x[1] == "tidy"]
  assert (label, model) == ("tidy", SONNET_M)
  assert "50 GB free, under the floor of 60 GB. Free the target of the oldest ended worktree first" in message
  assert message.endswith("The topics that build now, whose worktrees you keep: a.")


async def test_an_ended_item_is_judged_and_a_no_opens_it_again_one_effort_higher_at_most_three_times(conductor: Round) -> None:
  await conductor.word("SIDE[:] = [verify_step]\ndesk_queue['a'].append(item('one'))")
  await conductor.beat()
  for n in range(1, 5):
    await conductor.end([x for x in conductor.asked() if x[1] == "a"][-1][0], f"done, round {n}")
    await conductor.beat()
    judges = [x for x in conductor.asked() if x[1].startswith("verify")]
    assert (len(judges), judges[-1][1]) == (n, "verify one" if n == 1 else f"verify one-r{n - 1}")
    await conductor.end(judges[-1][0], "no: the test is missing")
    await conductor.beat()
  assert [(x["key"], x["model"].split("/")[-1]) for x in conductor.ns["desk_queue"]["a"]] == []
  assert [x[2].split("/")[-1] for x in conductor.asked() if x[1] == "a"] == ["high", "xhigh", "xhigh", "xhigh"]
  assert any("P0 unresolved: a:one-r3 failed its verification 3 times." in x for x in conductor.notes(conductor.root))


async def test_an_ended_item_whose_judge_gives_no_verdict_is_judged_again_and_then_reaches_the_lead_unresolved(conductor: Round) -> None:
  await conductor.word("SIDE[:] = [verify_step]\ndesk_queue['a'].append(item('one'))")
  await conductor.beat()
  await conductor.end([x for x in conductor.asked() if x[1] == "a"][-1][0], "done")
  await conductor.beat()
  for n in range(1, 4):
    judges = [x for x in conductor.asked() if x[1].startswith("verify")]
    assert len(judges) == n
    await conductor.end(judges[-1][0], RuntimeError("the judge fell"))
    await conductor.beat()
  assert len([x for x in conductor.asked() if x[1].startswith("verify")]) == 3
  assert conductor.ns["desk_verified"] == [] and conductor.queue("a") == []
  assert any("P0 unresolved: a:one has no verdict after 3 verifications" in x for x in conductor.notes(conductor.root))


async def test_the_needs_of_an_ended_item_go_to_the_triage_whose_answer_applies_whole_or_not_at_all(conductor: Round) -> None:
  await conductor.word("SIDE[:] = []\ndesk_width['b'] = 0\ndesk_queue['a'].append(item('one'))")
  await conductor.beat()
  await conductor.end(conductor.asked()[0][0], "done. Needs: b: export x\nNeeds: nowhere: no")
  await conductor.beat()
  assert [x["key"] for x in conductor.ns["triage_batch"]] == ["needs-one-1"]
  ((tri, label, model, _),) = conductor.asked("tuple[list[dict[str, object]], dict[str, str]]")
  assert (label, model) == ("triage", OPUS_M)
  await conductor.end(tri, ([], {}))
  await conductor.beat()
  assert conductor.queue("b") == ["needs-one-1"]
  assert "the triage was not applied: missing ['needs-one-1']" in conductor.ns["desk_text"]["triage_last"]
  await conductor.word("queue_found('k2 | a | sonnet/medium | p | fix p', 'r')")
  await conductor.beat()
  tri = conductor.asked("tuple[list[dict[str, object]], dict[str, str]]")[-1][0]
  await conductor.end(tri, ([{"key": "k2", "topic": "a", "class": "opus/high", "priority": 0, "brief": "hot fix p", "after": ["needs-one-1"]}], {}))
  await conductor.beat()
  assert conductor.ns["desk_queue"]["a"] == [{"key": "k2", "model": OPUS, "brief": "hot fix p", "after": ["needs-one-1"]}]
  assert await conductor.value("priority(desk_queue['a'][0])") == 0


async def test_the_deps_of_the_triage_apply_only_with_known_keys_and_no_cycle(conductor: Round) -> None:
  await conductor.word("desk_queue['a'].extend([item('x'), item('y')])\ndesk_width['a'] = 0")
  assert await conductor.value("deps_apply({'x': ['ghost']})") == "the deps of the triage were not applied: unknown or self keys ['x:ghost']"
  assert await conductor.value("deps_apply({'x': ['y'], 'y': ['x']})") == "the deps of the triage were not applied: a cycle holds ['x', 'y']"
  assert await conductor.value("deps_apply({'y': ['x']})") == "the triage set the deps: 1 items changed"
  assert conductor.ns["desk_queue"]["a"][1]["after"] == ["x"]


async def test_a_dead_wait_goes_to_the_triage_a_topic_that_runs_nothing_says_why_and_a_stuck_item_is_stepped(conductor: Round) -> None:
  await conductor.word("SIDE[:] = [blanks_and_stuck]\ndesk_queue['b'].append(item('lone', after=['ghost', 'one']))\ndesk_queue['a'].append(item('one'))")
  await conductor.beat()
  assert conductor.ns["desk_notes"][:2] == ["Dead waits sent to the triage: b:lone waits on ghost.", "b runs nothing, with 1 queued: lone (waits on ghost, one)."]
  ((_, label, _, ask),) = conductor.asked("dict[str, list[str]]")
  assert label == "triage" and "b:lone waits on ghost" in ask
  (one,) = conductor.ns["desk_live"]["a"]
  await conductor.beat()
  conductor.sand.now += 1300
  await conductor.beat()
  assert not any("made no step" in x for x in conductor.ns["desk_notes"])
  conductor.sand.now += 1500
  await conductor.beat()
  await conductor.beat()
  assert [x for x in [*conductor.ns["desk_notes"], *"\n\n".join(conductor.notes(conductor.root)).split("\n\n")] if "made no step" in x] == [f"a: {one} made no step for 47 minutes, and its last act still runs past the timeout of a commit."]


async def test_a_chain_past_its_bound_is_regrounded_once_until_it_grows_again_then_a_model_chooses_its_reads(conductor: Round) -> None:
  topic = await conductor.value('chain_of("a")')
  conductor.sand.cost = (600000, 0, 0, 0, 0.0)
  conductor.sand.script[topic] = ['read("notes.md")', "x = 2"]
  conductor.file("notes.md", "one\n")
  conductor.engine.prompt(str, "work", to=SONNET, on=topic)
  await settle()
  await conductor.beat()
  assert f"a ({topic}) read 600k tokens, 60% of its window, so it was regrounded on its reads." in conductor.ns["desk_notes"]
  await conductor.beat()
  assert conductor.ns["desk_ground"]["a"].startswith("g:")
  await conductor.word('desk_ground["a"] = "g:an-earlier-reply"')
  await conductor.beat()
  chose = conductor.ns["desk_ground"]["a"].removeprefix("c:")
  assert conductor.engine.get(chose)[0] == "prompt"
  await conductor.end(chose, ([], "nothing more to keep"))
  await conductor.beat()
  assert f"a ({topic}) still read 60% of its window after its reground, so a cheap model chose the acts to keep (0)." in conductor.ns["desk_notes"]


async def test_a_message_of_the_owner_is_judged_part_by_part_and_each_part_is_applied(conductor: Round) -> None:
  await conductor.word("SIDE[:] = [words_step]\ndesk_queue['a'].append(item('one'))\ndesk_width['a'] = 0")
  told = conductor.engine.prompt(str, "Do x and y.", on=conductor.root)
  await conductor.beat()
  ((judge, label, _, message),) = [x for x in conductor.asked() if x[1] == "words"]
  assert label == "words" and message.endswith("The message:\nDo x and y.") and told in message
  await conductor.end(judge, "covered x\namend one | y, verbatim\n- k9 | b | sonnet/medium | p | z")
  await conductor.beat()
  assert conductor.ns["desk_queue"]["a"][0]["brief"].endswith("a part of the ask of the owner that this item missed: y, verbatim")
  assert [x["key"] for x in conductor.ns["triage_batch"]] == ["k9"]
  await conductor.beat()
  assert len([x for x in conductor.asked() if x[1] == "words"]) == 1


async def test_reviews_come_each_hour_once_for_each_kind(conductor: Round) -> None:
  await conductor.word("desk_time['review_at'] = -1e9")
  await conductor.beat()
  assert [x[2] for x in conductor.asked() if x[1] == "review"] == ["codex:gpt-6-astra/high"] * 2
  assert conductor.queue("review") == ["review-a-1", "review-b-1"]
  conductor.sand.now += 3600
  await conductor.beat()
  assert len(conductor.queue("review")) == 2


async def test_notes_reach_the_root_at_once_when_urgent_and_else_each_hour(conductor: Round) -> None:
  await conductor.word("desk_time['notes_at'] = clock()\ndesk_notes.append('plain')")
  await conductor.beat()
  assert conductor.notes(conductor.root) == []
  await conductor.word("desk_notes.append('a P0 row')")
  await conductor.beat()
  assert conductor.notes(conductor.root) == ["Desk:\nplain\n\na P0 row"]


async def test_a_new_desk_ends_the_old_one_and_the_workers_live_on(conductor: Round) -> None:
  await conductor.word('desk_queue["b"].append(item("one"))\nfirst = desk()')
  ((worker, *_),) = conductor.asked()
  await conductor.word("second = desk()")
  first, second = conductor.ns["first"], conductor.ns["second"]
  assert conductor.engine.over(first) and not conductor.engine.over(second)
  assert conductor.ns["desk_text"]["id"] == second and not conductor.engine.over(worker)
  await conductor.word("close(None, second)")
  assert conductor.engine.over(second)


async def test_news_is_done_with_the_next_prompt_of_the_root(conductor: Round) -> None:
  await conductor.word("heard = news(600)")
  told = conductor.engine.prompt(str, "Next.", on=conductor.root)
  await settle()
  assert conductor.engine.peek(conductor.ns["heard"]) == told


async def test_the_lane_of_memory_reads_its_guard_and_holds_each_new_item_while_the_memory_runs_low(conductor: Round) -> None:
  conductor.file("mem.txt", "memory 9% free, swap 6000 MB in use, grown 0 MB in 1s\n")
  conductor.file("code.txt", "75\n")
  await conductor.word("SIDE[:] = [mem_step]\ndesk_time.update(mem_at=-1e9)")
  await conductor.beat()
  await conductor.beat()
  assert "The memory runs low: memory 9% free" in "\n".join(conductor.notes(conductor.root))
  assert conductor.ns["pick"]() is None


async def test_a_scripted_round_fills_each_free_seat_with_ready_work_hard_first_and_records_the_cause_of_each_seat_left_free(conductor: Round) -> None:
  await conductor.word("""ROOM_CAP.update({"sonnet/high": 2, "sonnet/medium": 1, "opus/high": 1})
desk_width.update({"a": 2, "b": 1})
POLISH_KEYS.append("shine")
desk_queue["a"].extend([item("shine"), item("deep", p=1), item("later", "claude-cli:opus/high", after=["deep"])])
desk_queue["b"].extend([item("fix-b", p=0), item("tidy-b", "claude-cli:sonnet/medium")])""")
  await conductor.beat()
  assert conductor.keys() == ["fix-b", "deep"]
  (said_,) = [x for x in conductor.ns["desk_notes"] if x.startswith("Free seats")]
  assert "opus/high: later waits on deep" in said_ and "sonnet/medium: tidy-b: b is at its width" in said_
  await conductor.beat()
  assert len([x for x in conductor.ns["desk_notes"] if x.startswith("Free seats")]) == 1
  deep = [x[0] for x in conductor.asked() if x[3].startswith("Item deep")][0]
  await conductor.end(deep, "done")
  await conductor.word("desk_time['merged_upto'] = clock() + 1")
  await conductor.beat()
  assert conductor.keys() == ["fix-b", "deep", "later", "shine"]
  assert not any(x[6] == OPERATOR for x in said(conductor.log, "prompt"))


async def test_a_part_of_the_owner_that_no_item_holds_reaches_the_lead_at_once(conductor: Round) -> None:
  await conductor.word("SIDE[:] = [words_step]\ndesk_time['notes_at'] = clock()")
  conductor.engine.prompt(str, "Do x.", on=conductor.root)
  await conductor.beat()
  ((judge, *_),) = [x for x in conductor.asked() if x[1] == "words"]
  await conductor.end(judge, "amend ghost | x, verbatim")
  await conductor.beat()
  assert any("not applied: ['amend ghost | x, verbatim']" in x for x in conductor.notes(conductor.root))


async def test_a_restart_before_and_after_the_fourth_merge_keeps_the_queue_and_audits_each_range_of_w_converge_and_main_once(tmp_path: Path) -> None:
  names = ("desk_time", "desk_text", "desk_queue", "desk_live")

  async def restart(old: Round, n: int) -> Round:
    kept = "\n".join(f"{name} = {old.ns[name]!r}" for name in names)
    (tmp_path / str(n)).mkdir()
    new = Round(tmp_path / str(n))
    await new.word(kept)
    await new.load(TEST)
    return new

  def audits(one: Round) -> list[tuple[str, str, str]]:
    return [(x["key"], x["model"], x["brief"].split("in the ranges ")[1].split(", each range")[0]) for x in one.ns["desk_queue"]["review"]]

  (tmp_path / "0").mkdir()
  life = Round(tmp_path / "0")
  await life.load(TEST)
  for n in range(3):
    await life.word(f"merges_step('head h{n}\\nmain m0')")
  life = await restart(life, 1)
  await life.word("merges_step('head h2\\nmain m0')")
  assert audits(life) == []
  await life.word("merges_step('head h3\\nmain m0')\nmerges_step('head h4\\nmain m1')")
  m4 = [("align-audit-m4", "codex:gpt-6-astra/high", "h0..h4 of w/converge and m0..m1 of main")]
  assert audits(life) == m4
  await life.word("merges_step('head h5\\nmain m1')\nmerges_step('head h6\\nmain m1')")
  life = await restart(life, 2)
  await life.word("merges_step('head h6\\nmain m1')\nmerges_step('head h7\\nmain m1')\nmerges_step('head h8\\nmain m2')")
  assert audits(life) == m4
  await life.beat()
  ((failed, *_),) = [x for x in life.asked() if x[1] == "review"]
  await life.end(failed, RuntimeError("the audit fell"))
  await life.beat()
  await life.word("merges_step('head h8\\nmain m2')")
  assert audits(life) == [("align-audit-m8", OPUS, "h0..h8 of w/converge and m0..m2 of main")]
  await life.beat()
  (ran,) = [x[0] for x in life.asked() if x[1] == "review" and x[0] != failed]
  await life.end(ran, "clean at h8 and m2")
  await life.beat()
  await life.word("\n".join(f"merges_step('head h{n}\\nmain m3')" for n in range(9, 13)))
  assert audits(life) == [("align-audit-m12", "codex:gpt-6-astra/high", "h8..h12 of w/converge and m2..m3 of main")]


async def test_the_ledger_holds_the_cost_of_each_ended_item_from_the_answers_on_its_chains_and_gives_the_cost_per_landed_item(conductor: Round) -> None:
  await conductor.word('desk_queue["a"].extend([item("one"), item("two", "claude-cli:opus/medium")])\ndesk_queue["b"].append(item("three"))')
  await conductor.beat()
  keys = dict(zip([x[0] for x in conductor.asked()], conductor.keys(), strict=True))
  uses = {"one": [None, (1000, 100, 600, 0, 0.5), (2000, 200, 1500, 0, 1.0)], "two": [(4000, 400, 1000, 0, 3.0)], "three": [(500, 50, 0, 0, 0.5)]}
  # A reply that the life writes itself, as the seat of a new chain, carries no usage: it is no step and costs nothing.
  ends = {"one": "done one", "two": "blocked: two", "three": "done three"}
  chains = set(conductor.ns["topic_chain"].values())
  while held := [x[1] for x in said(conductor.log, "reply") if x[3] in chains and not conductor.engine.over(x[1])]:
    for reply in held:
      key = keys[conductor.engine.get(conductor.engine.get(reply)[2])[2]]
      use = uses[key].pop(0)
      word = f"close({ends[key]!r})" if not uses[key] else f'debug(t"a step of {key}")'
      conductor.engine.say("done", reply, ("assistant", word, use, None))
    await settle()
  conductor.sand.now += 60
  await conductor.beat()
  assert len(conductor.ns["desk_ledger"]) == 3
  assert conductor.ns["desk_text"].get("ledger", "") == ""
  await conductor.word("desk_time['merged_upto'] = clock()")
  await conductor.beat()
  rows = {r["key"]: (r["topic"], r["class"], r["in"], r["out"], r["cached"], r["cost"], r["steps"], r["seconds"]) for r in conductor.ns["desk_ledger"]}
  assert rows == {
    "one": ("a", "sonnet/high", 3000, 300, 2100, 1.5, 2, 60.0),
    "two": ("a", "opus/medium", 4000, 400, 1000, 3.0, 1, 60.0),
    "three": ("b", "sonnet/high", 500, 50, 0, 0.5, 1, 60.0),
  }
  assert conductor.ns["desk_text"]["ledger"] == (
    "Cost per landed item since 0: by class opus/medium none of 1 landed, sonnet/high $1.00 (2 of 2 landed); "
    "by topic a $4.50 (1 of 2 landed), b $0.50 (1 of 1 landed); the cache gave 41% of 7500 tokens in."
  )
