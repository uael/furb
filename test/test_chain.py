"""chain, the act that holds facts, with a module and a working directory of its own."""

import asyncio
import symtable
from asyncio import CancelledError

import pytest

from conftest import DOOR, STANDS, Py, Sand, heads, life, named, paragraphs, ran, said, seen, settle, sown
from furb import engine
from furb.engine import OPERATOR, WORLD, Act, Refused, Text, take
from furb.kernel import ENGINE


def made(held: list[tuple]) -> list[str]:
  """The name of every act among the facts, which is the name the life holds an act under."""
  return [a[1] for a in held if engine.question(a) and a[1] in engine.acts]


def binding(id: str, of: str = "object") -> str:
  """The statement that binds the name of an act to the act, as a paragraph shows it."""
  return f"{id}: Act[{of}] = Act({id!r})"


def opened(id: str, text: str) -> str:
  """The paragraph that opens a chain: its header with its words, then the statement that binds its name."""
  return f"#{id} {text}\n{binding(id)}"


def stood(id: str) -> str:
  """The paragraph of the standing of the suite, as a chain without a source tells it."""
  return f"#{id} stands {STANDS!r}"


def steps(log: list[tuple], by: str) -> list[str]:
  """The rungs that one act made, in the order the life made them."""
  return [a[1] for a in said(log, "rung") if a[2] == by]


def notes(held: list[tuple], id: str) -> list[list[object]]:
  """The notes of every tell of a transcript about one act, in order."""
  return [a[3] for a in held if a[0] == "tell" and a[1] == id]


def on(a: tuple) -> str:
  """The chain a fact is on: the chain a question names, and the scope of the act that any other fact is about."""
  return a[3] if engine.question(a) else engine.scope(a[1])


def defined(text: str) -> set[str]:
  """The names a file of python binds at its top, as python reads the file."""
  table = symtable.symtable(text, "engine.py", "exec")
  return {s.get_name() for s in table.get_symbols() if s.is_assigned() or s.is_imported()}


async def test_chain_says_what_a_chain_does() -> None:
  """chain says what a chain does: how it is opened, what it tells, and what it answers for."""
  sand = sown()
  _, root = life(sand)
  two = engine.chain("two", source=root)
  await settle()
  assert isinstance(two, Act) and engine.peek(two) is None
  assert paragraphs(engine.turns(on=two)) == [opened(root, "root"), stood(root), opened(two, f"two from {root}")]
  assert engine.ask("program", two)[1] == {}
  assert engine.ask("stand", two)[1] == STANDS


async def test_boot_gives_the_root_and_chain_gives_the_chain_which_never_settles() -> None:
  """boot gives the root, and chain gives the chain, which never settles."""
  sand = sown()
  log, root = life(sand)
  two = engine.chain("two")
  await settle()
  assert (root, two) == ("chain1", "chain2")
  assert engine.peek(root) is None and engine.peek(two) is None
  assert [a for a in said(log, "done") if a[1] in (root, two)] == []


async def test_the_header_of_a_chain_with_a_source_carries_its_label_and_its_source() -> None:
  """The header of a chain with a source carries its label and its source."""
  sand = sown()
  _, root = life(sand)
  twin = engine.chain("twin", source=root)
  bare = engine.chain(source=root)
  await settle()
  assert heads(engine.turns(on=twin))[-1] == f"#{twin} twin from {root}" == "#chain2 twin from chain1"
  assert heads(engine.turns(on=bare))[-1] == f"#{bare} from {root}" == "#chain3 from chain1"


async def test_the_entry_that_opens_a_chain_with_a_source_carries_its_label_after_the_prefix() -> None:
  """The entry that opens a chain with a source carries its label, after the prefix."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  asked, was = engine.ask("transcript", root, root)
  twin = engine.chain("twin", source=root)
  await settle()
  held = engine.ask("transcript", twin, twin)[1]
  assert isinstance(was, list) and isinstance(held, list)
  opens = ("tell", twin, twin, [f"#{twin} twin from {root}", binding(twin)])
  assert held == [*was, asked, ("done", asked[1], root, was), opens]


async def test_chain_gives_the_new_chain_which_never_completes() -> None:
  """chain gives the new chain, which never completes."""
  sand = sown()
  log, root = life(sand)
  twin = engine.chain("twin", source=root)
  await settle()
  assert twin == "chain2" and engine.peek(twin) is None
  assert [a for a in said(log, "done") if a[1] == twin] == []


async def test_boot_gives_the_root_as_an_act_of_never_and_the_root_never_completes() -> None:
  """boot gives the root as an act of Never, and the root never completes."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("k = 1", on=root)
  await settle()
  one = engine.boot(list(sand.record), kernel=Py().kernel(), world=Sand(stands=STANDS).hears())
  await settle(300)
  assert one == root and engine.peek(one) is None


async def test_the_engine_makes_a_chain_in_one_way_by_running_its_ladder() -> None:
  """The engine makes a chain in one way: by running its rungs."""
  sand = sown()
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  twin = engine.chain("twin", source=root)
  await settle()
  assert ran(log) == ["k = 1", "k = 1"] and engine.modules[twin]["k"] == 1
  await engine.rung("later = 2", on=root)
  assert "later" not in engine.modules[twin]


async def test_in_a_chain_with_a_source_the_ladder_of_the_origin_runs_again_in_its_module() -> None:
  """In a chain with a source the rungs of the origin up to that source run again in the module of the new chain."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  one = engine.prompt(int, "count", on=root)
  assert await one == 2
  await settle()
  twin = engine.chain("twin", source=root)
  await settle()
  words = [f"{binding(root)}\n{binding(one, 'int')}", "a = 1", "close(a + 1)"]
  assert [(a[3], a[4]) for a in said(log, "run")] == [(root, word) for word in words] + [(twin, word) for word in words]
  assert engine.modules[twin]["a"] == 1 and engine.modules[twin]["__name__"] == twin


async def test_a_chain_given_a_source_stands_on_that_one() -> None:
  """A chain given a source stands on that one: it retells the words of it as they stand, each rung of it retelling a rung of that one, so that it makes the same acts and shares them, and what it holds of the transcript of that one is what its filter kept, though it runs every word all the same, so what it holds bound is more than its turns say."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nn = (await x).code\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  program = engine.ask("program", root)[1]
  assert isinstance(program, dict)
  narrow = engine.chain("narrow", source=root, filter=take(command, inside=False))
  await settle()
  retold = [a for a in said(log, "rung") if a[3] == narrow]
  assert [(a[4], a[5]) for a in retold] == [(word, one) for one, word in program.items()]
  assert [a[1] for a in said(log, "bash")] == [command]
  assert engine.modules[narrow]["n"] == 0 and engine.modules[narrow][command] == command
  assert command in named(engine.turns(on=root)) and command not in named(engine.turns(on=narrow))


async def test_a_prompt_to_an_actor_its_roster_does_not_hold_it_closes_with_the_refusal() -> None:
  """A prompt to an actor its roster does not hold it closes with the refusal."""
  sand = sown()
  _, root = life(sand)
  one = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  got = engine.peek(one)
  assert isinstance(got, Refused) and str(got) == "ghost no actor"
  wrong = engine.prompt(int, "hi", to="n/high", on=root)
  await settle()
  shut = engine.peek(wrong)
  assert isinstance(shut, Refused) and str(shut) == "n/high no actor"


async def test_a_chain_that_the_word_of_a_rung_opens_is_a_scope_of_its_own() -> None:
  """A chain that the word of a rung opens is a scope of its own: its words are on itself, though the chain fact itself stands on the chain of the rung that opened it; and when that rung is retold, the word makes the same chain, since a rung that retells another shares the acts it makes, so a word that opens a chain opens it once."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["side = chain('side', source=__name__)\nclose(side)", "close(None)"]
  side = await engine.prompt(str, "fork one", on=root)
  await settle()
  twin = engine.chain("twin", source=root)
  await settle(200)
  assert [a[1] for a in said(log, "chain")] == [root, side, twin]
  assert [a[3] for a in said(log, "chain") if a[1] == side] == [root]
  sand.script[side] = ["x = bash('echo hi')\nclose((await x).code)", "close(None)"]
  assert await engine.prompt(int, "count", on=side) == 0
  assert [a[3] for a in said(log, "bash")] == [side]


async def test_the_engine_refuses_a_prompt_to_an_actor_outside_the_roster() -> None:
  """The engine refuses a prompt to an actor outside the roster."""
  sand = sown()
  log, root = life(sand)
  one = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  got = engine.peek(one)
  assert isinstance(got, Refused) and str(got) == "ghost no actor" and said(log, "ask") == []


async def test_the_chain_holds_the_control_it_says_itself() -> None:
  """The chain holds the control it says itself, so what it closes tells the model what was done to it."""
  sand = sown()
  log, root = life(sand)
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  assert isinstance(engine.peek(ghost), Refused)
  sand.script[root] = ["k = BAD", "close(1)"]
  one = engine.prompt(int, "try", on=root)
  assert await one == 1
  step, last = steps(log, one)
  held = engine.ask("transcript", root, root)[1]
  assert isinstance(held, list)
  shut = [(a[1], a[2], a[4]) for a in held if a[0] == "close"]
  assert shut == [
    (ghost, root, [f"#{ghost} closed Refused('ghost no actor')"]),
    (step, root, [f"#{step} closed Refused()"]),
    (one, last, [f"#{one} closed 1"]),
  ]
  assert [line for line in heads(engine.turns(on=root)) if " closed " in line] == [told[0] for _, _, told in shut]


async def test_the_engine_binds_the_exception_of_a_raise_in_the_globals_of_the_chain() -> None:
  """The engine binds the exception of a raise in the globals of the chain, under the name raised."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["raise ValueError('boom')", "close(1)", "close(None)"]
  assert await engine.prompt(int, "try", on=root) == 1
  assert isinstance(engine.modules[root]["raised"], ValueError)
  assert str(engine.modules[root]["raised"]) == "boom"


async def test_the_engine_binds_the_exception_again_at_each_raise() -> None:
  """The engine binds the exception again at each raise."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["raise ValueError('one')", "raise KeyError('two')", "close(1)", "close(None)"]
  assert await engine.prompt(int, "try", on=root) == 1
  assert isinstance(engine.modules[root]["raised"], KeyError)


async def test_a_chain_its_module_which_is_the_engine_itself_named_for_the_chain() -> None:
  """A chain: its module, which is the engine itself, named for the chain, since everything the file defines is the model's to call and nothing of it is bound to one chain, and what the operator would add to it, it makes a rung of, which binds it, stands in the program and is said again in a later life, a fact said from a run being on the chain of that run, and a word that wants a chain of its own giving the name of its own as the source."""
  sand = sown()
  log, root = life(sand)
  assert engine.modules[root]["__name__"] == root and engine.modules[root]["bash"] is engine.bash
  await engine.rung("k = 21", on=root)
  _, program = engine.ask("program", root)
  assert isinstance(program, dict)
  assert list(program.values()) == ["k = 21"] and engine.modules[root]["k"] == 21
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  assert said(log, "bash")[0][3] == root
  _, over = life(Sand(stands=STANDS), list(sand.record))
  await settle(300)
  assert engine.modules[over]["k"] == 21


async def test_a_chain_is_chain_n_whether_boot_or_chain_opened_it_and_the_root_is_chain1() -> None:
  """A chain is chainN whether boot or chain opened it, and the root is chain1."""
  sand = sown()
  log, root = life(sand)
  two = engine.chain("two")
  sand.script[root] = ["side = chain('side')\nclose(side)"]
  one = engine.prompt(str, "fork one", on=root)
  theirs = await one
  assert (root, two, theirs) == ("chain1", "chain2", "chain3")
  assert [a[1:3] for a in said(log, "chain")] == [(root, OPERATOR), (two, OPERATOR), (theirs, steps(log, one)[0])]


async def test_a_chain_is_an_act_of_never_so_the_chain_never_completes() -> None:
  """A chain is an act of Never, so the chain never completes."""
  sand = sown()
  log, _ = life(sand)
  two = engine.chain("two")
  await settle()
  assert engine.peek(two) is None and [a for a in said(log, "done") if a[1] == two] == []


async def test_the_turns_of_a_chain_tell_the_standing_and_the_acts_of_the_operator() -> None:
  """The turns of a chain tell the standing and the acts of the operator."""
  sand = sown()
  _, root = life(sand)
  step = engine.rung("k = 1", on=root)
  assert await step is None
  one = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert paragraphs(engine.turns(on=root)) == [
    opened(root, "root"),
    stood(root),
    f"#{step}\nk = 1",
    f"#{one} how many?\n{binding(one, 'int')}",
  ]


async def test_the_transcript_of_a_chain_is_the_facts_on_it_in_the_order_it_heard_them() -> None:
  """The transcript of a chain is the facts on it, in the order it heard them."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  await settle()
  held = engine.ask("transcript", root, root)[1]
  assert isinstance(held, list)
  assert all(on(a) == root for a in held)
  assert made(held) == [one, *steps(log, one)]
  heard = [log.index(a) for a in held if a in log]
  assert heard == sorted(heard) and [a[0] for a in held if a not in log] == ["holds"]


async def test_nothing_leaves_a_transcript_once_the_transcript_holds_it() -> None:
  """Nothing leaves a transcript once the transcript holds it."""
  sand = sown()
  _, root = life(sand)
  one = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  was = engine.ask("transcript", root, root)[1]
  engine.cancel(one)
  await settle()
  now = engine.ask("transcript", root, root)[1]
  assert isinstance(was, list) and isinstance(now, list)
  assert now[: len(was)] == was and len(now) > len(was)


async def test_an_assistant_turn_keeps_the_role_assistant_in_every_chain_made_from_the_chain() -> None:
  """An assistant turn keeps the role assistant in every chain made from the chain."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["close(1)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  twin = engine.chain("twin", source=root)
  await settle()
  assert [turn[0] for turn in engine.turns(on=twin)] == [turn[0] for turn in engine.turns(on=root)]
  assert [turn[0] for turn in engine.turns(on=twin)].count("assistant") == 1


async def test_the_engine_adds_the_acts_that_caused_a_kept_act_to_what_the_filter_kept() -> None:
  """The engine adds the acts that caused a kept act to what the filter kept."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command = said(log, "bash")[0]
  one, ack = [a[1] for a in said(log, "prompt")]
  step = command[2]
  twin = engine.chain("twin", source=root, filter=take(command[1]))
  await settle()
  assert steps(log, one) == [step] and ack in named(engine.turns(on=root))
  assert named(engine.turns(on=twin)) == [root, root, one, step, command[1], one, command[1], twin]


async def test_the_globals_of_a_chain_whose_filter_is_a_take_that_is_not_inside() -> None:
  """The globals of a chain whose filter is a take that is not inside hold the bindings of the skipped words still."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nn = (await x).code", "close(n)"]
  one = engine.prompt(int, "run it", on=root)
  assert await one == 0
  await settle()
  command, skipped = said(log, "bash")[0][1], steps(log, one)[0]
  narrow = engine.chain("narrow", source=root, filter=take(skipped, inside=False))
  await settle()
  words = [turn[1] for turn in engine.turns(on=root) if turn[0] == "assistant"]
  assert words == ["x = bash('echo hi')\nn = (await x).code", "close(n)"]
  assert [turn[1] for turn in engine.turns(on=narrow) if turn[0] == "assistant"] == words[1:]
  assert skipped not in named(engine.turns(on=narrow)) and command not in named(engine.turns(on=narrow))
  assert (engine.modules[narrow]["x"], engine.modules[narrow]["n"]) == (command, 0)


async def test_a_chain_with_a_source_made_after_the_rung_that_defined_a_door_has_the_door_too() -> None:
  """A chain with a source made after the rung whose word defined a door has the door too."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = [DOOR, "close(None)"]
  assert await engine.prompt(int, "a door", on=root) == 1
  await settle()
  twin = engine.chain("twin", source=root)
  await settle()
  assert engine.read("note://a", on=twin).content == "kept"


async def test_the_globals_of_a_chain_are_those_of_a_module_named_by_the_id_of_the_chain() -> None:
  """The globals of a chain are those of a module named by the id of the chain."""
  sand = sown()
  _, root = life(sand)
  two = engine.chain("two")
  assert engine.modules[root]["__name__"] == root and engine.modules[two]["__name__"] == two
  await engine.rung("k = 1", on=root)
  assert "k" in engine.modules[root] and "k" not in engine.modules[two]


async def test_every_name_that_the_file_defines_is_in_the_globals_of_a_chain() -> None:
  """Every name that the file defines is in the globals of a chain."""
  sand = sown()
  _, root = life(sand)
  assert defined(ENGINE) <= set(engine.modules[root])


async def test_a_chain_holds_whole_every_act_made_on_the_chain() -> None:
  """A chain holds whole every act made on the chain, but a rung it makes itself, which it runs and holds nothing of."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nx = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "work", on=root) == 1
  await settle()
  held = engine.ask("transcript", root, root)[1]
  assert isinstance(held, list)
  mine = [a for a in log if engine.question(a) and a[1] in engine.acts and a[3] == root]
  assert [a[0] for a in mine] == ["prompt", "rung", "rung", "bash", "prompt", "rung", "rung"]
  own = [a[1] for a in mine if a[0] == "rung" and a[2] == root]
  assert own == ["rung2", "rung4"] and [engine.acts[one][4] for one in own] == [
    f"{root}: Act[object] = Act('{root}')\nprompt1: Act[int] = Act('prompt1')",
    "bash1: Act[Exit] = Act('bash1')\nprompt2: Act[None] = Act('prompt2')",
  ]
  assert [a[1] for a in mine if a not in held] == own
  assert [a for a in held if a[1] in own] == []
  assert [a for a in held if a[0] == "bash"] == said(log, "bash")


async def test_a_chain_holds_every_fact_on_it_and_an_act_on_another_chain_is_that_chains() -> None:
  """A chain holds every fact on it, and an act on another chain is that chain's."""
  sand = sown()
  log, root = life(sand)
  two = engine.chain("two")
  sand.script[root] = [f"p = prompt(int, 'hi', on={two!r})\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "delegate", on=root) == 1
  await settle()
  theirs = said(log, "prompt")[-1]
  held, ours = engine.ask("transcript", two, two)[1], engine.ask("transcript", root, root)[1]
  assert isinstance(held, list) and isinstance(ours, list)
  assert theirs in held
  assert theirs not in ours


async def test_a_prompt_that_a_step_of_another_chain_made_reads_nothing_of_that_chain() -> None:
  """A prompt that a step of another chain made reads nothing of that chain."""
  sand = sown()
  log, root = life(sand)
  two = engine.chain("two")
  sand.script[root] = [f"secret = 1\nclose(await prompt(int, 'count', on={two!r}))", "close(None)"]
  sand.script[two] = ["close(2)"]
  assert await engine.prompt(int, "delegate", on=root) == 2
  await settle()
  theirs = next(a for a in said(log, "ask") if a[3] == two)
  asked = next(a[1] for a in said(log, "prompt") if a[3] == two)
  assert paragraphs(theirs[5]) == [
    opened(two, "two"),
    stood(two),
    f"#{asked} count\n{binding(asked, 'int')}",
    f"#{theirs[1]} advance on {asked}",
  ]


async def test_the_word_of_a_rung_rebinds_the_default_actor_like_any_name() -> None:
  """The word of a rung rebinds the default actor like any name, and the last binding in record order wins."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["actor = 'n/low'\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "rebind", on=root) == 1
  await settle()
  assert engine.modules[root]["actor"] == "n/low"
  sand.script[root] = ["close(2)", "close(None)"]
  assert await engine.prompt(int, "again", on=root) == 2
  assert [a[4] for a in said(log, "ask")][-1] == "n/low"


async def test_the_transcript_of_the_root_begins_with_the_open_of_the_root_and_then_the_standing() -> None:
  """The transcript of the root begins with the open of the root and then the standing."""
  sand = sown()
  _, root = life(sand)
  held = engine.ask("transcript", root, root)[1]
  assert isinstance(held, list)
  assert held[:2] == [("tell", root, root, [f"#{root} root", binding(root)]), ("tell", root, root, [stood(root)])]
  assert paragraphs(engine.turns(on=root))[:2] == [opened(root, "root"), stood(root)]


async def test_the_transcript_then_holds_the_prompt_of_the_operator_and_the_asks_and_the_responses() -> None:
  """The transcript then holds the prompt of the operator, and the asks and the responses of the model."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a)"]
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  await settle()
  held = engine.ask("transcript", root, root)[1]
  assert isinstance(held, list)
  assert [a[1] for a in held if a[0] == "prompt"] == [one]
  assert [a[1] for a in held if a[0] == "ask"] == [a[1] for a in held if a[0] == "answer"] == steps(log, one)
  assert [a[3][1] for a in held if a[0] == "answer"] == ["a = 1", "close(a)"]


async def test_the_engine_reads_its_own_names_through_the_globals_of_the_chain() -> None:
  """The engine reads its own names through the globals of the chain."""
  sand = Sand(files={"/w/mine.txt": "mine\n"}, stands=STANDS)
  _, root = life(sand)
  assert engine.modules[root]["read"] is engine.read
  await engine.rung("def read(path, show=HEAD, on=''):\n  return ask('read', __name__, 'mine.txt')[1]", on=root)
  sand.script[root] = ["close(read('any.txt').content)", "close(None)"]
  assert await engine.prompt(str, "read it", on=root) == "mine\n"
  assert [a[4] for a in sand.calls if a[0] == "read"] == ["mine.txt"]


async def test_the_engine_uses_a_rebound_name_from_the_next_use_on() -> None:
  """The engine uses a rebound name from the next use on."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n", "/w/mine.txt": "mine\n"}, stands=STANDS)
  _, root = life(sand)
  sand.script[root] = [
    "was = read('a.txt').content",
    "def read(path, show=HEAD, on=''):\n  return ask('read', __name__, 'mine.txt')[1]",
    "close([was, read('a.txt').content])",
    "close(None)",
  ]
  assert await engine.prompt(list, "read it", on=root) == ["one\ntwo\n", "mine\n"]
  assert [a[4] for a in sand.calls if a[0] == "read"] == ["a.txt", "mine.txt"]


async def test_no_close_reaches_a_chain_since_a_chain_never_completes() -> None:
  """No close reaches a chain, since a chain never completes."""
  sand = sown()
  _, _ = life(sand)
  two = engine.chain("two")
  engine.close(5, two)
  await settle()
  assert engine.peek(two) is None
  sand.script[two] = ["close(1)", "close(None)"]
  assert await engine.prompt(int, "still here", on=two) == 1


async def test_to_await_a_chain_never_returns() -> None:
  """To await a chain never returns."""
  sand = sown()
  _, _ = life(sand)
  two = engine.chain("two")
  with pytest.raises(TimeoutError):
    await asyncio.wait_for(asyncio.shield(two), 0.01)


async def test_a_chain_has_a_globals_dict_and_a_working_directory_of_its_own() -> None:
  """A chain has a globals dict and a working directory of its own."""
  sand = sown()
  _, root = life(sand)
  two = engine.chain("two")
  await engine.rung("here = 1", on=root)
  engine.cd("/deep", on=two)
  assert (engine.cwd(on=root), engine.cwd(on=two)) == ("/w", "/deep")
  assert "here" in engine.modules[root] and "here" not in engine.modules[two]


async def test_a_chain_with_a_source_holds_the_acts_it_inherited_as_the_filter_kept_them() -> None:
  """A chain with a source holds the acts it inherited from that source, as the filter kept them."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\ny = bash('echo there')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run them", on=root) == 1
  await settle()
  one, two = said(log, "bash")
  twin = engine.chain("twin", source=root, filter=take(two[1], inside=False))
  await settle()
  held = engine.ask("transcript", twin, twin)[1]
  assert isinstance(held, list)
  assert one in held and two not in held


async def test_the_steps_that_an_inherited_prompt_takes_after_the_point_enter_its_owner_alone() -> None:
  """The steps that an inherited prompt takes after the point enter the transcript of its owner alone."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  one = engine.prompt(int, "count", on=root)
  engine.pause(one)
  await settle()
  twin = engine.chain("twin", source=root)
  await settle()
  engine.wake(one)
  await settle()
  assert (await one) == 2
  first, *later = steps(log, one)
  held = engine.ask("transcript", twin, twin)[1]
  assert isinstance(held, list)
  assert later and [(a[0], a[1]) for a in held if a[0] in ("rung", "ask", "answer")] == [
    ("rung", first),
    ("ask", first),
    ("answer", first),
  ]
  assert [a for a in held if a[1] in later] == []
  _, theirs = engine.ask("transcript", root, root)
  assert isinstance(theirs, list)
  assert [a[1] for a in theirs if a[0] == "answer"] == [first, *later]


async def test_what_a_chain_binds_is_its_own_and_a_chain_with_a_source_is_how_it_gets_isolation() -> None:
  """What a chain binds is its own, and a chain with a source is how a chain gets isolation."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("k = 1", on=root)
  twin = engine.chain("twin", source=root)
  await settle()
  await engine.rung("k = 2", on=twin)
  assert (engine.modules[root]["k"], engine.modules[twin]["k"]) == (1, 2)


async def test_two_chains_from_one_source_hold_the_same_values() -> None:
  """Two chains from one source hold the same values, since both ran the same rungs."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("k = 21", on=root)
  one = engine.chain("one", source=root)
  two = engine.chain("two", source=root)
  await settle()
  assert engine.modules[one]["k"] == engine.modules[two]["k"] == 21


async def test_a_chain_with_a_source_inherits_the_default_actor_with_the_globals_of_its_origin() -> None:
  """A chain with a source inherits the default actor with the globals of its origin at that source."""
  sand = sown()
  _, root = life(sand)
  twin = engine.chain("twin", source=root)
  await settle()
  assert engine.modules[twin]["actor"] == engine.modules[root]["actor"] == "m/low"


async def test_a_chain_with_a_source_that_holds_an_act_reads_the_close_of_the_act() -> None:
  """A chain with a source that holds an act reads the close of the act in its transcript."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  command = said(log, "bash")[0][1]
  twin = engine.chain("twin", source=root)
  await settle()
  shut = [one for one in paragraphs(engine.turns(on=twin)) if one.startswith(f"#{command} exited")]
  assert shut == [f"#{command} exited 0\n# {command}/stdout, 0 known\n# 1 ran echo hi"]


async def test_a_chain_with_a_source_awaits_or_peeks_an_inherited_act_as_it_likes() -> None:
  """A chain with a source awaits or peeks an inherited act as it likes."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  twin = engine.chain("twin", source=root)
  await settle()
  sand.script[twin] = ["out = await x\nseen = peek(x)\nassert isinstance(seen, Exit)\nclose([out.code, seen.code])"]
  assert await engine.prompt(list, "look at it", on=twin) == [0, 0]
  assert [a[1] for a in said(log, "bash")] == [command]


async def test_the_engine_does_not_wake_a_chain_with_a_source_for_an_inherited_act() -> None:
  """The engine does not wake a chain with a source for the result of an inherited act."""
  sand = Sand(files={"/w/a.txt": "one\ntwo\n"}, stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "start one", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  twin = engine.chain("twin", source=root)
  await settle()
  before = len([a for a in said(log, "ask") if a[3] == twin])
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert len([a for a in said(log, "ask") if a[3] == twin]) == before
  assert [a for a in said(log, "prompt") if a[2] == twin] == []


async def test_the_globals_of_a_chain_with_a_source_may_hold_more_than_its_turns_say() -> None:
  """The globals of a chain with a source may hold more than its turns say."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nn = (await x).code\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  narrow = engine.chain("narrow", source=root, filter=take(command, inside=False))
  await settle()
  assert command not in named(engine.turns(on=narrow))
  assert [one for one in paragraphs(engine.turns(on=narrow)) if binding(command, "Exit") in one] == []
  assert (engine.modules[narrow]["n"], engine.modules[narrow]["x"], engine.modules[narrow][command]) == (
    0,
    command,
    command,
  )


async def test_the_transcript_of_a_chain_with_a_source_holds_the_entries_up_to_that_source_first() -> None:
  """The transcript of a chain with a source holds the entries up to that source first, then the entry that opened it."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("k = 1", on=root)
  asked, was = engine.ask("transcript", root, root)
  twin = engine.chain("twin", source=root)
  await settle()
  held = engine.ask("transcript", twin, twin)[1]
  assert isinstance(was, list) and isinstance(held, list)
  opens = ("tell", twin, twin, [f"#{twin} twin from {root}", binding(twin)])
  assert held == [*was, asked, ("done", asked[1], root, was), opens]
  assert all(on(a) == root for a in held[:-1])


async def test_the_acts_of_the_prefix_of_a_chain_with_a_source_keep_the_ids_they_had() -> None:
  """The acts of the prefix of a chain with a source keep the ids they had on the origin."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  held = engine.ask("transcript", root, root)[1]
  assert isinstance(held, list)
  was = made(held)
  twin = engine.chain("twin", source=root)
  await settle()
  now = engine.ask("transcript", twin, twin)[1]
  assert isinstance(now, list)
  assert made(now)[: len(was)] == was


async def test_the_globals_of_a_chain_with_a_source_are_those_of_a_module_of_its_own() -> None:
  """The globals of a chain with a source are those of a module of its own."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("k = 1", on=root)
  twin = engine.chain("twin", source=root)
  await settle()
  assert engine.modules[twin] is not engine.modules[root]
  assert engine.modules[twin]["__name__"] == twin and engine.modules[twin]["k"] == 1


async def test_the_objects_of_a_rung_of_a_chain_with_a_source_are_that_chains_own() -> None:
  """The objects of a rung of a chain with a source are that chain's own, made again, and only code is shared."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("marks = [1]\ndef twice(x):\n  return x * 2", on=root)
  twin = engine.chain("twin", source=root)
  await settle()
  assert engine.modules[twin]["marks"] == engine.modules[root]["marks"]
  assert engine.modules[twin]["marks"] is not engine.modules[root]["marks"]
  assert engine.modules[twin]["twice"] is not engine.modules[root]["twice"]
  mine, theirs = engine.ask("program", twin)[1], engine.ask("program", root)[1]
  assert isinstance(mine, dict) and isinstance(theirs, dict)
  assert list(mine.values()) == list(theirs.values()) and list(mine) != list(theirs)


async def test_the_globals_of_a_chain_with_a_source_are_the_origins_whatever_the_filter_kept() -> None:
  """The globals of a chain with a source are the origin's at that source, whatever the filter kept."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("k = 21", on=root)
  twin = engine.chain("twin", source=root, filter=take())
  await settle()
  assert engine.modules[twin]["k"] == 21
  assert paragraphs(engine.turns(on=twin)) == [opened(root, "root"), stood(root), opened(twin, f"twin from {root}")]


async def test_a_chain_with_a_source_inherits_the_words_its_caller_wrote_with_its_prefix() -> None:
  """A chain with a source inherits the words its caller wrote with its prefix, wherever its source."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["j = k + 1\nclose(j)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await engine.rung("m = 3", on=root)
  twin = engine.chain("twin", source=root)
  await settle()
  assert (engine.modules[twin]["k"], engine.modules[twin]["j"], engine.modules[twin]["m"]) == (1, 2, 3)


async def test_a_filter_that_skips_a_rung_its_caller_wrote_keeps_it_out_of_the_turns() -> None:
  """A filter that skips a rung its caller wrote keeps it out of the turns of the new chain."""
  sand = sown()
  _, root = life(sand)
  laid = engine.rung("k = 1", on=root)
  await laid
  twin = engine.chain("twin", source=root, filter=take(laid, inside=False))
  await settle()
  assert engine.modules[twin]["k"] == 1
  assert paragraphs(engine.turns(on=root)) == [opened(root, "root"), stood(root), f"#{laid}\nk = 1"]
  assert paragraphs(engine.turns(on=twin)) == [opened(root, "root"), stood(root), opened(twin, f"twin from {root}")]


async def test_a_chain_with_a_source_holds_the_classes_its_origin_defined_before_that_source() -> None:
  """A chain with a source holds the classes that its origin defined before that source."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("class Plan:\n  n = 21\n\nmine = Plan().n", on=root)
  twin = engine.chain("twin", source=root)
  await settle()
  assert isinstance(engine.modules[twin]["Plan"], type) and engine.modules[twin]["mine"] == 21
  assert engine.modules[twin]["Plan"] is not engine.modules[root]["Plan"]


async def test_a_chain_with_a_source_reads_nothing_that_its_origin_did_after_that_source() -> None:
  """A chain with a source reads nothing that its origin did after that source."""
  sand = sown()
  _, root = life(sand)
  twin = engine.chain("twin", source=root)
  await settle()
  told = engine.turns(on=twin)
  after = engine.rung("after = 1", on=root)
  await after
  await settle()
  assert engine.turns(on=twin) == told == [("user", "\n\n".join(paragraphs(told)), None, None)]
  assert paragraphs(told) == [opened(root, "root"), stood(root), opened(twin, f"twin from {root}")]
  assert "after" not in engine.modules[twin] and paragraphs(engine.turns(on=root))[-1:] == [f"#{after}\nafter = 1"]


async def test_what_a_rung_of_a_chain_with_a_source_binds_lands_on_that_chain() -> None:
  """What a rung of a chain with a source binds lands on that chain and not on its origin."""
  sand = sown()
  _, root = life(sand)
  twin = engine.chain("twin", source=root)
  await settle()
  await engine.rung("mine = 1", on=twin)
  assert engine.modules[twin]["mine"] == 1 and "mine" not in engine.modules[root]


async def test_the_origin_holds_the_chain_fact_of_a_chain_that_a_rung_of_it_made() -> None:
  """The origin holds the chain fact of a chain that a rung of it made, and nothing of that chain's own."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["side = chain('side', source=__name__)\nclose(side)", "close(None)"]
  side = await engine.prompt(str, "fork one", on=root)
  await settle()
  held = engine.ask("transcript", root, root)[1]
  assert isinstance(held, list)
  assert [a[1] for a in held if a[0] == "chain"] == [side]
  assert [a for a in held if on(a) == side] == []


async def test_two_names_of_the_module_are_the_chains_to_bind() -> None:
  """Two names of the module are the chain's to bind: the actor it stands on, and what the last rung that raised raised, so that the word of a rung reads what the word before it came to."""
  sand = sown()
  _, root = life(sand)
  sand.script[root] = ["raise ValueError('boom')", "close([actor, str(raised)])", "close(None)"]
  assert await engine.prompt(list, "try", on=root) == ["m/low", "boom"]


async def test_it_holds_the_transcript_of_its_origin_first_and_tells_its_own_open_after_it() -> None:
  """It holds the transcript of its origin first and tells its own open after it, and it holds nothing of the rungs it runs again, since every word of those stands in what it inherited."""
  sand = sown()
  log, root = life(sand)
  step = engine.rung("k = 1", on=root)
  await step
  was = engine.ask("transcript", root, root)[1]
  twin = engine.chain("twin", source=root)
  await settle()
  held = engine.ask("transcript", twin, twin)[1]
  assert isinstance(was, list) and isinstance(held, list)
  assert held[: len(was)] == was and held[-1] == ("tell", twin, twin, [f"#{twin} twin from {root}", binding(twin)])
  retold = [a[1] for a in said(log, "rung") if a[3] == twin]
  assert [engine.get(one)[5] for one in retold] == [step]
  assert [a for a in held if a[1] in retold] == []
  assert paragraphs(engine.turns(on=twin)) == [
    opened(root, "root"),
    stood(root),
    f"#{step}\nk = 1",
    opened(twin, f"twin from {root}"),
  ]


async def test_it_hears_no_control_that_ends_it() -> None:
  """It hears no control that ends it, since nothing that happens to a chain ends it, and a control over a chain is over the acts on it, which end themselves."""
  sand = sown()
  _, _ = life(sand)
  two = engine.chain("two")
  one = engine.prompt(int, "one", to=OPERATOR, on=two)
  await settle()
  engine.cancel(two)
  await settle()
  assert isinstance(engine.peek(one), CancelledError) and engine.peek(two) is None
  sand.script[two] = ["close(1)", "close(None)"]
  assert await engine.prompt(int, "still here", on=two) == 1


async def test_it_holds_no_done_of_a_query_it_does_not_hold() -> None:
  """It holds no done of a query it does not hold, since the answer to what it asked of the gate is of the moment, so every done it holds answers a question it holds."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  held = engine.ask("transcript", root, root)[1]
  assert isinstance(held, list)
  names = {a[1] for a in held if engine.question(a)}
  assert all(a[1] in names for a in held if a[0] == "done")
  gates = [a[1] for a in engine.asked.values() if a[0] == "gate"]
  assert [a[1] for a in said(log, "done") if a[1] in gates] == gates == [f"gate@{root}.4"]
  assert [a for a in held if a[1] in gates] == []


async def test_what_a_chain_with_a_source_holds_of_the_transcript_of_its_origin() -> None:
  """What a chain with a source holds of the transcript of its origin: what its filter kept, everything that made what it kept, each holds of that transcript, which cuts its turns where an ask of the origin stood, and the open of the origin, which tells the standing."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command, one = said(log, "bash")[0], said(log, "prompt")[0][1]
  twin = engine.chain("twin", source=root, filter=take(command[1]))
  await settle()
  held, theirs = engine.ask("transcript", twin, twin)[1], engine.ask("transcript", root, root)[1]
  assert isinstance(held, list) and isinstance(theirs, list)
  assert command in held and [a[1] for a in held if a[0] == "prompt"] == [one]
  assert made(held) == [one, command[2], command[1]]
  assert notes(held, root) == [[f"#{root} root", binding(root)], [stood(root)]]
  assert [a for a in held if a[0] == "holds"] == [a for a in theirs if a[0] == "holds"] != []
  got = engine.turns(on=twin)
  assert [role for role, *_ in got] == ["user", "assistant", "user"]
  assert got[0][1].endswith(f"#{command[2]} advance on {one}")


async def test_the_filter_is_given_every_act_the_queries_of_the_operator_among_them() -> None:
  """The filter is given every act, the queries of the operator among them, since what the operator asked of a chain no rung of it says again."""
  sand = sown()
  _, root = life(sand)
  engine.cd("/deep", on=root)
  await engine.rung("k = 1", on=root)
  watch: list[tuple] = []
  twin = engine.chain("twin", source=root, filter=seen(watch))
  await settle()
  assert all(engine.question(a) for a in watch)
  assert [a[0] for a in watch if a[2] == OPERATOR] == ["cd", "rung"]
  held = engine.ask("transcript", twin, twin)[1]
  assert isinstance(held, list)
  assert [a[4] for a in held if a[0] == "cd"] == ["/deep"]


async def test_what_a_chain_said_of_itself_it_keeps_whatever_the_filter_says() -> None:
  """What a chain said of itself it keeps whatever the filter says, so the open of the origin stands in the new chain."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("k = 1", on=root)
  twin = engine.chain("twin", source=root, filter=take())
  await settle()
  assert heads(engine.turns(on=twin)) == [f"#{root} root", stood(root), f"#{twin} twin from {root}"]


async def test_the_source_of_a_chain_is_a_chain_by_its_name() -> None:
  """The source of a chain is a chain, by its name, and means the transcript of that chain as it stands."""
  sand = sown()
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  was = engine.ask("transcript", root, root)[1]
  twin = engine.chain("twin", source=root)
  await settle()
  await engine.rung("later = 2", on=root)
  held = engine.ask("transcript", twin, twin)[1]
  assert isinstance(was, list) and isinstance(held, list)
  assert said(log, "chain")[-1][5] == root
  assert held[: len(was)] == was and "later" not in engine.modules[twin]


async def test_a_source_that_names_no_chain_of_the_life_refuses_the_call() -> None:
  """A source that names no chain of the life refuses the call in the caller, and no chain is made."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  step = engine.rung("k = 1", on=root)
  assert await step is None
  with pytest.raises(Refused, match=f"^no chain {step}$"):
    engine.chain("sub", source=step, on=root)
  with pytest.raises(Refused, match=r"^no chain chain9$"):
    engine.chain("sub", source="chain9", on=root)
  sand.script[root] = ["chain('sub', source=acting())", "close(1)"]
  one = engine.prompt(int, "fork", on=root)
  assert await one == 1
  await settle()
  forked = steps(log, one)[0]
  assert [a for a in heads(engine.turns(on=root)) if " raised " in a] == [
    f"#{forked} raised Refused('no chain {forked}')"
  ]
  assert [a[4] for a in said(log, "chain")] == ["root"]
  assert [e[1][4] for e in sand.record if e[1][0] == "chain"] == ["root"]


async def test_a_chain_with_a_source_asks_its_origin_what_it_stands_on() -> None:
  """A chain with a source asks its origin what it stands on, and the origin answers with its standing as it stands."""
  sand = sown()
  _, root = life(sand)
  before = [a for a in sand.calls if a[0] == "stand"]
  twin = engine.chain("twin", source=root)
  await settle()
  assert [a for a in sand.calls if a[0] == "stand"] == before
  asked = [a for a in engine.asked.values() if a[0] == "stand"]
  assert [(a[2], a[3]) for a in asked] == [(root, root), (twin, root)]
  assert engine.outcomes[asked[1][1]] == STANDS and engine.modules[twin]["actor"] == "m/low"


async def test_the_chain_answers_a_stand_asked_on_it_with_what_it_stands_on() -> None:
  """The chain answers a stand asked on it with what it stands on, so a grant reads the roster off the chain it is on."""
  sand = Sand(stands=STANDS, cost=(200000, 0, 0, 0, 0.0))
  log, root = life(sand)
  assert engine.ask("stand", root)[1] == STANDS
  sand.script[root] = ["close(1)"]
  engine.grant(share=0.9, on=root)
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  await settle()
  answered = steps(log, one)[0]
  assert [a for a in heads(engine.turns(on=root)) if " ledger " in a] == [f"#{answered} ledger spent=0.0 filled=0.5"]
  assert [a for a in sand.calls if a[0] == "stand"] == sand.calls[:1]


async def test_the_chain_answers_a_transcript_asked_of_one_of_its_names() -> None:
  """The chain answers a transcript asked of one of its names with its transcript up to that act, and whole for the chain itself."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  whole = engine.ask("transcript", root, root)[1]
  one = said(log, "prompt")[0][1]
  cut = engine.ask("transcript", root, one)[1]
  assert isinstance(whole, list) and isinstance(cut, list)
  assert cut == whole[: len(cut)] and len(cut) < len(whole)
  assert cut[-1][1] == one


async def test_the_chain_answers_a_program_asked_on_it() -> None:
  """The chain answers a program asked on it with the word of every rung it holds that runs, as python, each under the name of that rung, in order."""
  sand = sown()
  log, root = life(sand)
  mine = engine.rung("mine = 1", on=root)
  await mine
  sand.script[root] = ["<S1>\nhi\n</S1>\na = S1", "b = BAD", "close(len(a))"]
  one = engine.prompt(int, "count", on=root)
  assert await one == 3
  await settle()
  first, refused, last = steps(log, one)
  (bind,) = steps(log, root)
  program = engine.ask("program", root)[1]
  assert isinstance(program, dict)
  assert list(program.items()) == [
    (mine, "mine = 1"),
    (bind, f"{binding(root)}\n{binding(one, 'int')}"),
    (first, "S1 = 'hi\\n'\n\n\na = S1"),
    (last, "close(len(a))"),
  ]
  assert refused not in program and [a[1] for a in said(log, "run")] == list(program)


async def test_the_chain_retells_the_ladder_of_its_origin_through_the_program() -> None:
  """The chain retells the rungs of its origin through the program the origin answers, and it owns the rungs it retells, though it holds nothing of them."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  one = engine.prompt(int, "count", on=root)
  assert await one == 2
  await settle()
  twin = engine.chain("twin", source=root)
  await settle()
  asked = [a for a in engine.asked.values() if a[0] == "program"]
  assert [a[2:] for a in asked] == [(twin, root)]
  theirs = engine.outcomes[asked[0][1]]
  assert isinstance(theirs, dict)
  assert list(theirs.values()) == [f"{binding(root)}\n{binding(one, 'int')}", "a = 1", "close(a + 1)"]
  retold = [a for a in said(log, "rung") if a[3] == twin]
  assert [(a[2], a[4], a[5]) for a in retold] == [(twin, word, donor) for donor, word in theirs.items()]
  assert engine.ask("program", twin)[1] == {a[1]: a[4] for a in retold}
  held = engine.ask("transcript", twin, twin)[1]
  assert isinstance(held, list)
  owned = {a[1] for a in retold}
  assert [a for a in held if a[1] in owned] == []


async def test_a_replay_makes_the_rungs_of_a_chain_again_from_its_donor() -> None:
  """A replay makes the rungs of a chain again from its donor: it keeps each rung of the ladder while the words it is given repeat it, it makes one rung of what is left, and every rung of the chain after the first word that differs is gone."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "b = 2", "c = 3", "close(None)"]
  act = engine.prompt(None, "count", on=root)
  assert await act is None
  await settle()
  (bind,) = steps(log, root)
  first, *_ = steps(log, act)
  was = [a[1] for a in said(log, "rung")]
  engine.write(Text(act, "a = 1\nb = 20\nc = 30"), on=root)
  await settle()
  kept = engine.ask("program", root)[1]
  assert isinstance(kept, dict)
  assert list(kept.values()) == [f"{binding(root)}\n{binding(act, 'None')}", "a = 1", "b = 20\nc = 30"]
  assert [one for one in kept if one in was] == []
  assert [engine.get(one)[5] for one in kept] == [bind, first, ""]
  assert (engine.modules[root]["b"], engine.modules[root]["c"]) == (20, 30)


async def test_the_donor_of_a_replay_is_the_ladder_of_the_origin_or_the_ladder_as_it_stands() -> None:
  """The donor of a replay is the rungs of the origin for a chain with a source, and the rungs of the chain as they stand for a write of the door of one of its prompts."""
  sand = sown()
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  twin = engine.chain("twin", source=root)
  await settle(300)
  theirs = [a for a in said(log, "rung") if a[3] == twin]
  assert [a[5] for a in theirs] == [a[1] for a in said(log, "rung") if a[3] == root]
  act = engine.prompt(int, "more", to=OPERATOR, on=twin)
  engine.write(Text(act, "k = 2"), on=twin)
  await settle()
  mine = [a for a in said(log, "rung") if a[3] == twin]
  assert [a[5] for a in mine[len(theirs) :]] == [theirs[0][5], ""]
  assert engine.modules[twin]["k"] == 2 and engine.modules[root]["k"] == 1


async def test_a_write_of_a_door_gives_the_words_of_that_ladder_alone() -> None:
  """A write of a door gives the words of that ladder alone, so a rung of the chain that is no rung of that ladder and stands before the first word that differs stands as it did."""
  sand = sown()
  _, root = life(sand)
  await engine.rung("mine = 1", on=root)
  sand.script[root] = ["a = 1", "close(None)"]
  act = engine.prompt(None, "count", on=root)
  assert await act is None
  await settle()
  assert engine.read(act, on=root) == Text(act, "a = 1\nclose(None)")
  engine.write(Text(act, "a = 2"), on=root)
  await settle()
  _, program = engine.ask("program", root)
  assert isinstance(program, dict)
  assert list(program.values()) == ["mine = 1", f"{binding(root)}\n{binding(act, 'None')}", "a = 2"]
  assert engine.modules[root]["mine"] == 1 and engine.modules[root]["a"] == 2


async def test_a_replay_makes_the_module_of_the_chain_again_as_it_was_at_its_birth() -> None:
  """A replay makes the module of the chain again, as it was at its birth, and makes its rungs in that one, so what a word it drops bound is gone, and a word that runs while it happens ends in the module it began in."""
  sand = sown()
  _, root = life(sand)
  act = engine.prompt(None, "work", to=OPERATOR, on=root)
  engine.write(Text(act, "a = 1\nb = 2"), on=root)
  await settle()
  engine.write(Text(act, "a = 1"), on=root)
  await settle()
  assert engine.read(act, on=root) == Text(act, "a = 1")
  assert engine.modules[root]["a"] == 1 and "b" not in engine.modules[root]
  sand.script[root] = ["write(Text(get(acting())[2], 'c = 3'))\nd = 4", "close(None)"]
  later = engine.prompt(None, "edit", on=root)
  assert await later is None
  await settle(300)
  assert engine.read(later, on=root) == Text(later, "c = 3\nclose(None)")
  assert engine.modules[root]["c"] == 3 and engine.modules[root]["a"] == 1
  assert "d" not in engine.modules[root]


async def test_before_every_ask_the_chain_tells_the_last_line_of_the_turn() -> None:
  """Before every ask the chain tells the last line of the turn, which says what the answer is for: #rung5 advance on prompt1 for a rung of a prompt, and #rung5 advance for a rung that no prompt made."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a)", "k = 2"]
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  bare = engine.rung(on=root)
  assert await bare is None
  await settle()
  first, second = steps(log, one)
  asks = said(log, "ask")
  assert [(a[1], a[5][-1][0], a[5][-1][1].split("\n")[-1]) for a in asks] == [
    (first, "user", f"#{first} advance on {one}"),
    (second, "user", f"#{second} advance on {one}"),
    (bare, "user", f"#{bare} advance"),
  ]
  assert engine.modules[root]["k"] == 2


async def test_where_it_asks_the_chain_makes_a_rung_of_the_statements_the_turn_shows() -> None:
  """Where it asks, the chain makes a rung of the statements the turn shows of every act but a rung, which bind the name of each act the turn opened; the gate does not read that rung, since the engine wrote it, and the turns do not show it, since the turn shows its statements already."""
  sand = sown()
  log, root = life(sand)
  laid = engine.rung("k = 1", on=root)
  assert await laid is None
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  one = engine.prompt(int, "run it", on=root)
  assert await one == 1
  await settle()
  command = said(log, "bash")[0][1]
  ack = said(log, "prompt")[-1][1]
  assert engine.peek(ack) is None and ack != one
  binders = steps(log, root)
  asks = said(log, "ask")
  assert [[a for a in log[: log.index(ask)] if a[0] == "rung"][-1][1] for ask in asks] == binders
  statements = [binding(root), binding(one, "int"), binding(command, "Exit"), binding(ack, "None")]
  assert [engine.get(bind)[4] for bind in binders] == ["\n".join(statements[:2]), "\n".join(statements[2:])]
  shown = [line for part in paragraphs(engine.turns(on=root)) for line in part.split("\n") if line[:1] != "#"]
  assert shown == [statements[0], "k = 1", *statements[1:]]
  assert [engine.modules[root][name] for name in (root, one, command, ack)] == [root, one, command, ack]
  assert [a[1] for a in said(log, "rung") if a[1] in engine.modules[root]] == []
  words = [turn[1] for turn in engine.turns(on=root) if turn[0] == "assistant"]
  assert words == ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert [a[4] for a in engine.asked.values() if a[0] == "gate"] == ["k = 1", *words]
  assert [name for name in named(engine.turns(on=root)) if name in binders] == []
