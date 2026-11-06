"""gathered, the data of a checkpoint of a life."""

from conftest import born, life, paragraphs, settle, sown, world_says
from furb import engine
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
  return root, engine.gathered(lambda word: word, {"wait": 9})


def value(root: str, name: str) -> str:
  """The value of a name of the module of a chain."""
  return str(engine.module(root)[name])


async def test_gathered_gives_the_data_of_a_checkpoint_of_the_life_at_its_tip_which_seed_reads_as_plain_data_that_each_build_of_the_major_reads_taken_chains_and_acts() -> (
  None
):
  """gathered gives the data of a checkpoint of the life at its tip, which seed reads, as plain data that each build of the major reads: taken, chains and acts."""
  _, data = await gather()
  assert [*data] == ["taken", "chains", "acts"]
  assert [*engine.seed(data)[0][0]] == ["chain", "chain1", OPERATOR, "", "root", ""]


async def test_taken_gives_the_highest_number_of_each_kind_that_an_act_of_a_chain_took_and_at_least_the_number_that_floor_gives_since_a_later_life_makes_no_act_again_that_an_ear_of_the_outside_made() -> (
  None
):
  """taken gives the highest number of each kind that an act of a chain took, and at least the number that floor gives, since a later life makes no act again that an ear of the outside made."""
  root, data = await gather()
  assert data["taken"]["bash"] == int(value(root, "shell").removeprefix("bash"))
  assert data["taken"]["wait"] == max(9, int(value(root, "later").removeprefix("wait")) + 1)
  assert data["taken"]["chain"] == 1


async def test_chains_holds_each_chain_of_the_life_a_chain_at_rest_among_them_each_with_its_parent_its_label_its_source_whether_it_is_paused_its_turns_but_the_facts_made_what_define_gives_of_each_word_that_ran_on_it_the_text_that_binds_each_plain_value_of_its_module_again_the_names_of_the_other_values_and_the_acts_that_it_cut_each_word_that_ran_on_it_among_them() -> (
  None
):
  """chains holds each chain of the life, a chain at rest among them, each with its parent, its label, its source, whether it is paused, its turns but the facts made, what define gives of each word that ran on it, the text that binds each plain value of its module again, the names of the other values, the acts that it cut, each word that ran on it among them, and each life word of the standing whose act was open on it at the tip, as lives."""
  root, _ = await gather()
  world_says("made", root, "an engine", 9, "1.0", "0.4.2", "its text", "the text of its Kernel")
  [one] = engine.gathered(lambda word: word, {"wait": 9})["chains"]
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
  data = engine.gathered(lambda word: word, {})
  assert rest in [x["id"] for x in data["chains"]]
  life(sown(), engine.seed(data))
  await settle()
  now = paragraphs(engine.turns(on=rest))
  assert now[: len(was) - 1] == was[:-1] and now[len(was) - 1].startswith(was[-1]), (was, now)


async def test_acts_holds_the_last_stand_the_last_clock_and_each_act_of_a_verb_that_a_value_names_or_that_is_a_command_of_a_chain_in_chains_an_act_that_is_over_with_its_value_an_open_prompt_with_its_shape_its_message_its_actor_and_the_ear_that_started_it_and_another_act_as_cut_with_a_refusal_that_says_why() -> (
  None
):
  """acts holds the last stand, the last clock, and each act of a verb that a value names or that is a command of a chain in chains: an act that is over with its value, an open prompt with its shape, its message, its actor and the ear that started it, and another act as cut, with a refusal that says why."""
  root, data = await gather()
  acts = {x["id"]: x for x in data["acts"]}
  shell, asked, later = (value(root, name) for name in ("shell", "asked", "later"))
  assert acts[shell] == {"kind": "bash", "id": shell, "on": root, "value": engine.peek(shell)}
  held = acts[asked]
  assert (held["kind"], held["shape"], held["message"], held["to"]) == ("prompt", "str", "Which?", OPERATOR)
  cut = acts[later]["value"]
  assert isinstance(cut, Refused) and str(cut) == f"{later} was interrupted: the life that ran it ended before it did"
  assert [x["kind"] for x in data["acts"]][:1] == ["stand"]


async def test_gathered_is_a_view_it_makes_nothing_and_says_nothing() -> None:
  """gathered is a view: it makes nothing and says nothing."""
  root, _ = await gather()
  before = engine.transcript(root)
  engine.gathered(lambda word: word, {})
  assert engine.transcript(root) == before


async def test_paused_says_whether_a_pause_stands_over_the_chain_at_the_tip_and_no_prompt_of_it_pauses_it_so_a_later_life_goes_on_as_the_tip_left_it() -> (
  None
):
  """paused says whether a pause stands over the chain at the tip, and no prompt of it pauses it, so a later life goes on as the tip left it."""
  root, data = await gather()
  assert [x["paused"] for x in data["chains"]] == [False]
  engine.pause(root)
  [one] = engine.gathered(lambda word: word, {})["chains"]
  assert one["paused"]


async def test_chains_holds_each_plain_value_of_a_chain_whatever_its_size_and_seed_binds_a_large_one_from_the_carried_fact_of_its_chain_so_a_later_life_holds_it_exactly_and_no_turn_tells_it() -> (
  None
):
  """chains holds each plain value of a chain whatever its size, and seed binds a large one from the carried fact of its chain, so a later life holds it exactly and no turn tells it."""
  _, _, root = born()
  rest = engine.chain("rest", source=root)
  await engine.rung("big = 'x' * LARGEST\nran = Exit(0, Text('o', 'y' * LARGEST), Text('e', ''))\nsmall = 1", on=rest)
  await settle()
  big, ran = engine.module(rest)["big"], engine.module(rest)["ran"]
  data = engine.gathered(lambda word: word, {})
  [one] = [x for x in data["chains"] if x["id"] == rest]
  assert (one["values"]["small"], {"big", "ran"} <= one["values"].keys(), one["lost"]) == ("1", True, [])
  entries = engine.seed(data)
  assert all(len(str(word).encode()) < LARGEST // 8 for x in entries if x[0][0] == "rung" for word in x[0][4:5])
  log, _ = life(sown(), entries)
  await settle()
  assert (engine.module(rest)["big"], engine.module(rest)["ran"], engine.module(rest)["small"]) == (big, ran, 1)
  assert all(len(str(x).encode()) < LARGEST // 8 for x in log if x[0] == "tell")
