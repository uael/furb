"""gathered, the data of a checkpoint of a life."""

from conftest import born, definitions, life, paragraphs, settle, sown, world_says
from furb import checkpoint, engine
from furb.engine import LARGEST, OPERATOR, Refused

WORD = """def f():
  return 1
k = [1, 'x', {2: (3,)}]
gone = iter([1])
loop = []
loop.append(loop)
shell = bash('echo one')
asked = prompt(str, 'Which?', to=OPERATOR)
later = wait(600, asked)"""
"""A word of the root that binds a definition, a plain value, a value that is not plain, a value that holds itself, a
command, an open prompt and a wait that is not over."""


async def gather() -> tuple[str, dict]:
  """A life whose root ran the word and runs a word that awaits a wait, and the data that gathered gives of it, with a
  floor of nine waits."""
  _, _, root = born()
  await engine.rung(WORD, on=root)
  await engine.rung("await shell", on=root)
  engine.rung("await wait(600, asked)", on=root)
  await settle()
  return root, checkpoint.gathered(vars(engine), lambda word: word, {"wait": 9})


def value(root: str, name: str) -> str:
  """The value of a name of the module of a chain."""
  return str(engine.module(root)[name])


async def test_gathered_gives_the_data_of_a_checkpoint_of_the_life_at_its_tip_which_seed_reads_as_plain_data_that_each_build_of_the_major_reads_taken_chains_and_acts() -> (
  None
):
  """gathered gives the data of a checkpoint of the life at its tip, which seed reads, as plain data that each build of the major reads: taken, chains and acts."""
  _, data = await gather()
  assert [*data] == ["taken", "chains", "acts"]
  assert [*checkpoint.seed(vars(engine), data)[0][0]] == ["chain", "chain1", OPERATOR, "", "root", ""]


async def test_taken_gives_the_highest_number_of_each_kind_that_an_act_of_a_chain_took_and_at_least_the_number_that_floor_gives_since_a_later_life_makes_no_act_again_that_an_ear_of_the_outside_made() -> (
  None
):
  """taken gives the highest number of each kind that an act of a chain took, and at least the number that floor gives, since a later life makes no act again that an ear of the outside made."""
  root, data = await gather()
  assert data["taken"]["bash"] == int(value(root, "shell").removeprefix("bash"))
  assert data["taken"]["wait"] == max(9, int(value(root, "later").removeprefix("wait")) + 1)
  assert data["taken"]["chain"] == 1


async def test_chains_holds_each_chain_of_the_life_a_chain_at_rest_among_them_each_with_its_parent_its_label_its_source_whether_it_is_paused_its_turns_but_the_facts_made_what_define_gives_of_each_word_that_ran_on_it_as_one_string_for_each_word_that_defines_a_name_in_order_the_text_that_binds_each_plain_value_of_its_module_again_the_names_of_the_other_values_and_the_acts_that_it_cut_each_word_that_ran_on_it_among_them() -> (
  None
):
  """chains holds each chain of the life, a chain at rest among them, each with its parent, its label, its source, whether it is paused, its turns but the facts made, what define gives of each word that ran on it, as one string for each word that defines a name, in order, the text that binds each plain value of its module again, the names of the other values, the acts that it cut, each word that ran on it among them, and each life word of the standing whose rung stands in the lineage of an act whose ear still hears on it at the tip, an open act or a done act with a lasting ear, as lives."""
  root, _ = await gather()
  # A verb with no words makes an act that makes an open prompt, as name_session makes the prompt of its chain.
  await engine.rung("with site.set(act('named', ROOT, None)):\n  named = prompt(str, 'Name?', to=OPERATOR)", on=root)
  await settle()
  world_says("made", root, "an engine", 9, "1.0", "0.4.2", "its text", "the text of its Kernel")
  await engine.rung("def g():\n  return 2", on=root)
  await settle()
  assert [x["definitions"] for x in checkpoint.gathered(vars(engine), definitions, {})["chains"]] == [
    ["def f():\n  return 1", "def g():\n  return 2"]
  ]
  [one] = checkpoint.gathered(vars(engine), lambda word: word, {"wait": 9})["chains"]
  assert one["lives"] == []
  assert (one["id"], one["on"], one["label"], one["source"], one["paused"]) == (root, "", "root", "", False)
  assert WORD in one["definitions"]
  assert one["values"]["k"] == "[1, 'x', {2: (3,)}]"
  assert one["values"]["shell"] == f"Act({value(root, 'shell')!r})"
  assert "gone" in one["lost"] and "loop" in one["lost"]
  [run] = [x[1] for x in engine.transcript(root, "run") if not engine.over(x[1]) and x[5] == "await wait(600, asked)"]
  assert one["cut"] == [run, value(root, "later")] and "again" not in one
  assert {"bash", "prompt", "tell"} <= {x[0] for x in one["turns"]} and "made" not in {x[0] for x in one["turns"]}
  rest = engine.chain("rest", source=root)
  await engine.rung("small = 1", on=rest)
  await settle()
  was = paragraphs(engine.turns(on=rest))
  data = checkpoint.gathered(vars(engine), lambda word: word, {})
  assert rest in [x["id"] for x in data["chains"]]
  life(sown(), checkpoint.seed(vars(engine), data))
  await settle()
  now = paragraphs(engine.turns(on=rest))
  assert now[: len(was) - 1] == was[:-1] and now[len(was) - 1].startswith(was[-1]), (was, now)


async def test_acts_holds_the_last_stand_the_last_clock_each_prompt_and_each_command_of_a_chain_in_chains_and_each_act_of_a_verb_that_a_value_names_so_a_later_life_holds_each_thread_of_a_chain_a_closed_one_too() -> (
  None
):
  """acts holds the last stand, the last clock, each prompt and each command of a chain in chains, and each act of a verb that a value names: an act that is over with its value, an open prompt with its shape, its message, its actor and the ear that started it, and another act as cut, with a refusal that says why, so a later life holds each thread of a chain, a closed one too, and a prompt on it goes on."""
  root, _ = await gather()
  thread = engine.prompt(str, "Done?", to=OPERATOR, on=root)
  engine.close("yes", thread)
  await settle()
  data = checkpoint.gathered(vars(engine), lambda word: word, {"wait": 9})
  acts = {x["id"]: x for x in data["acts"]}
  by = lambda name: engine.get(name)[2]  # noqa: E731
  assert acts[thread] == {
    "kind": "prompt",
    "id": thread,
    "on": root,
    "by": by(thread),
    "words": engine.get(thread)[4:],
    "value": "yes",
  }
  shell, asked, later = (value(root, name) for name in ("shell", "asked", "later"))
  assert acts[shell] == {
    "kind": "bash",
    "id": shell,
    "on": root,
    "by": by(shell),
    "words": engine.get(shell)[4:],
    "value": engine.peek(shell),
  }
  held = acts[asked]
  assert (held["kind"], held["shape"], held["message"], held["to"]) == ("prompt", "str", "Which?", OPERATOR)
  cut = acts[later]["value"]
  assert isinstance(cut, Refused) and str(cut) == f"{later} was interrupted: the life that ran it ended before it did"
  assert [x["kind"] for x in data["acts"]][:1] == ["stand"]
  life(sown(), checkpoint.seed(vars(engine), data))
  await settle()
  went = engine.prompt(None, "Go on.", on=thread)
  assert engine.get(went)[3] == thread


async def test_each_chain_and_each_act_of_the_data_names_who_made_it_as_by_the_maker_that_a_fact_carried_names_for_it_since_a_birth_made_it_again_else_the_maker_that_its_question_says() -> (
  None
):
  """Each chain and each act of the data names who made it as by: the maker that a fact carried names for it, since a birth made it again, else the maker that its question says."""
  root, data = await gather()
  shell, asked = value(root, "shell"), value(root, "asked")
  makers = {root: OPERATOR, shell: engine.get(shell)[2], asked: engine.get(asked)[2]}
  assert makers[shell].startswith("rung") and makers[asked].startswith("rung")
  named = {x["id"]: x["by"] for x in [*data["chains"], *data["acts"]] if x["id"] in makers}
  assert named == makers
  life(sown(), checkpoint.seed(vars(engine), data))
  await settle()
  assert (engine.get(shell)[2], engine.get(asked)[2]) == ("journal", OPERATOR)
  again = checkpoint.gathered(vars(engine), lambda word: word, {})
  assert {x["id"]: x["by"] for x in [*again["chains"], *again["acts"]] if x["id"] in makers} == makers


async def test_gathered_is_a_view_it_makes_nothing_and_says_nothing() -> None:
  """gathered is a view: it makes nothing and says nothing."""
  root, _ = await gather()
  before = engine.transcript(root)
  checkpoint.gathered(vars(engine), lambda word: word, {})
  assert engine.transcript(root) == before


async def test_paused_says_whether_a_pause_stands_over_the_chain_at_the_tip_and_no_prompt_of_it_pauses_it_so_a_later_life_goes_on_as_the_tip_left_it() -> (
  None
):
  """paused says whether a pause stands over the chain at the tip, and no prompt of it pauses it, so a later life goes on as the tip left it."""
  root, data = await gather()
  assert [x["paused"] for x in data["chains"]] == [False]
  engine.pause(root)
  [one] = checkpoint.gathered(vars(engine), lambda word: word, {})["chains"]
  assert one["paused"]


async def test_chains_holds_each_plain_value_of_a_chain_whatever_its_size_and_seed_binds_each_from_the_fact_carried_of_its_chain_so_a_later_life_holds_it_exactly_and_no_turn_tells_it() -> (
  None
):
  """chains holds each plain value of a chain whatever its size, and seed binds each from the fact carried of its chain, so a later life holds it exactly and no turn tells it."""
  _, _, root = born()
  rest = engine.chain("rest", source=root)
  await engine.rung("big = 'x' * LARGEST\nran = Exit(0, Text('o', 'y' * LARGEST), Text('e', ''))\nsmall = 1", on=rest)
  await settle()
  big, ran = engine.module(rest)["big"], engine.module(rest)["ran"]
  for _ in (1, 2):
    data = checkpoint.gathered(vars(engine), definitions, {})
    [one] = [x for x in data["chains"] if x["id"] == rest]
    assert (one["values"]["small"], {"big", "ran"} <= one["values"].keys(), one["lost"]) == ("1", True, [])
    assert not [d for d in one["definitions"] if "LARGEST" in d]
    entries = checkpoint.seed(vars(engine), data)
    assert all(len(str(word).encode()) < LARGEST // 8 for x in entries if x[0][0] == "rung" for word in x[0][4:5])
    log, _ = life(sown(), entries)
    await settle()
    here = engine.module(rest)
    assert (here["big"], here["ran"], here["small"], here["raised"]) == (big, ran, 1, None)
    assert all(len(str(x).encode()) < LARGEST // 8 for x in log if x[0] == "tell")
    assert not [x for x in log if x[0] == "tell" and " raised" in x[3][0].split("\n")[0]]


async def test_chains_holds_each_chain_whose_ear_still_hears_after_a_number_that_no_chain_of_the_life_holds_too() -> (
  None
):
  """chains holds each chain whose ear still hears, after a number that no chain of the life holds too."""
  _, _, root = born()
  engine.chain("lost", on=root)
  await settle()
  data = checkpoint.gathered(vars(engine), lambda word: word, {})
  data["chains"] = [c for c in data["chains"] if c["id"] != "chain2"]
  life(sown(), checkpoint.seed(vars(engine), data))
  await settle()
  assert engine.get("chain2") is None
  kept = engine.chain("kept", on=root)
  engine.prompt(str, "Which?", to=OPERATOR, on=kept)
  await settle()
  assert kept in [c["id"] for c in checkpoint.gathered(vars(engine), lambda word: word, {})["chains"]]


async def test_gathered_reads_the_questions_of_the_life_alone_so_the_fact_of_an_act_of_an_earlier_life_that_a_carried_turn_holds_raises_nothing() -> (
  None
):
  """gathered reads the questions of the life alone, so the fact of an act of an earlier life that a carried turn holds raises nothing."""
  _, _, root = born()
  await settle()
  earlier = ("prompt", "prompt99", OPERATOR, "chain9", "str", "Which?", "")
  engine.say("prefix", root, [earlier])
  data = checkpoint.gathered(vars(engine), lambda word: word, {})
  assert [c["id"] for c in data["chains"]] == [root]
  assert "prompt99" not in [a["id"] for a in data["acts"]]


async def test_the_definitions_of_a_chain_take_no_text_that_the_chain_heard() -> None:
  """The definitions of a chain take only the words whose rung is an act, so no text that the chain heard, as a note in prose of a tell, and the word of a checkpoint parses, so a later life binds each value again."""
  _, _, root = born()
  await engine.rung("k = 1", on=root)
  world_says("tell", root, ["A note in prose, which no python parses."])
  await engine.rung("j = 2", on=root)
  await settle()
  data = checkpoint.gathered(vars(engine), definitions, {})
  [one] = [x for x in data["chains"] if x["id"] == root]
  assert not [d for d in one["definitions"] if "A note in prose" in d]
  _, again = life(sown(), checkpoint.seed(vars(engine), data))
  await settle()
  assert (engine.module(again)["k"], engine.module(again)["j"]) == (1, 2)


async def test_the_turns_of_a_chain_keep_no_reply_that_is_not_done_so_a_later_life_holds_no_reply_that_no_ear_of_it_takes() -> (
  None
):
  """The turns of a chain keep no reply that is not done, so a later life holds no reply that no ear of it takes."""
  _, log, root = born("close(1)", auto=False)
  assert await engine.prompt(int, "count", on=root) == 1
  engine.prompt(int, "again", on=root)
  await settle()
  (open_reply,) = [a[1] for a in log if a[0] == "reply" and not engine.over(a[1])]
  (chain,) = [c for c in checkpoint.gathered(vars(engine), lambda word: word, {})["chains"] if c["id"] == root]
  replies = [x[1] for x in chain["turns"] if x[0] in ("reply", "started", "done") and x[1].startswith("reply")]
  assert open_reply not in replies and replies != []


async def test_the_text_of_a_value_is_the_python_that_makes_it_again() -> None:
  """The text of a value is the python that makes it again: None, a bool, an int, a float, a str, a Text, an Exit or an exception as python shows it, a list, a tuple, a set or a dict of those as python writes it, and an Act as Act('bash1')."""
  _, _, root = born()
  word = "k = {'one': [None, True, 1, 2.5, 'x', Text('a', 'b'), Exit(0, Text(''), Text('')), (1,), set(), {3}], 2: ()}\ne = KeyError('k')\nshell = bash('echo one')"
  await engine.rung(word, on=root)
  await settle()
  [one] = checkpoint.gathered(vars(engine), lambda word: word, {})["chains"]
  assert one["values"]["k"] == repr(engine.module(root)["k"])
  assert one["values"]["e"] == "KeyError('k')"
  assert one["values"]["shell"] == f"Act({str(engine.module(root)['shell'])!r})"


async def test_every_other_value_is_lost_and_the_rest_of_the_checkpoint_is_gathered_all_the_same(
  engine_of: str,
) -> None:
  """Every other value is lost, and so is a float that is not finite, an exception whose arguments are lost, and a list, a tuple, a set or a dict that holds a lost value at any depth, so its name stands among the lost names and the rest of the checkpoint is gathered all the same."""
  _, _, root = born()
  word = (
    "gone = iter([1])\nmany = [1, gone]\nonce = (gone,)\nkeyed = {'k': gone}\ndeep = {1: [gone]}\n"
    "listed = [{'k': gone}]\npaired = ({'k': len},)\nnan = float('nan')\ninf = -float('inf')\nkept = 1"
  )
  # monty makes an exception of no argument or of one string alone, so only this interpreter holds one whose argument
  # is lost.
  wrong = {"wrong"} if engine_of == "python" else set()
  await engine.rung(word + "\nwrong = ValueError(len)" * bool(wrong), on=root)
  [one] = checkpoint.gathered(vars(engine), lambda word: word, {})["chains"]
  lost = {"gone", "many", "once", "keyed", "deep", "listed", "paired", "nan", "inf"} | wrong
  assert lost <= set(one["lost"]) and not lost & one["values"].keys()
  assert one["values"]["kept"] == "1"
