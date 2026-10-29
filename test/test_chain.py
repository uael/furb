"""chain, the act that holds facts, with a module and a working directory of its own."""

import asyncio
import symtable
from asyncio import CancelledError
from dataclasses import dataclass
from pathlib import Path

import pytest

import furb
from conftest import (
  DOOR,
  STANDS,
  Py,
  Sand,
  World,
  acts,
  born,
  chained,
  fresh,
  gatings,
  heads,
  ids,
  lasting,
  life,
  named,
  opened,
  paragraphs,
  plain,
  posed,
  prompted,
  ran,
  relived,
  said,
  seen,
  settle,
  takes,
  world_says,
  written,
)
from furb import engine
from furb.engine import OPERATOR, ROOT, Act, Refused, Text, take


def binding(id: str, of: str = "object") -> str:
  """The statement that binds the name of an act to the act, as a paragraph shows it."""
  return f"{id}: Act[{of}] = Act({id!r})"


def steps(log: list[tuple], by: str) -> list[str]:
  """The rungs that one act made, in the order the life made them."""
  return [a[1] for a in said(log, "rung") if a[2] == by]


def on(a: tuple) -> str:
  """The chain a fact is on: the chain a question names, and the scope of the act that any other fact is about."""
  return a[3] if engine.question(a) else engine.scope(a[1])


def defined(text: str) -> set[str]:
  """The names a file of python binds at its top, as python reads the file."""
  table = symtable.symtable(text, "engine.py", "exec")
  return {s.get_name() for s in table.get_symbols() if s.is_assigned() or s.is_imported()}


async def told_read() -> tuple[Sand, list[tuple], str, Act, Act]:
  """A life whose root reads a file, then reads the text that the read told: the World, what was said, the root, and
  the two rungs."""
  sand, log, root = born()
  first = engine.rung("t = read('a.txt')", on=root)
  assert await first is None
  second = engine.rung("got = read1_text", on=root)
  assert await second is None
  return sand, log, root, first, second


async def test_chain_says_what_a_chain_does() -> None:
  """chain says what a chain does: how it is opened, what it tells, and what it answers for."""
  _, _, root = born()
  two = engine.chain("two", source=root)
  await settle()
  assert isinstance(two, Act) and engine.peek(two) is None
  assert paragraphs(engine.turns(on=two)) == [opened(root, "root"), takes(root), opened(two, "two", root)]
  assert engine.program(two) == {}
  assert engine.standing() == STANDS


async def test_boot_gives_the_root_and_chain_gives_the_chain_which_never_settles() -> None:
  """boot gives the root, and chain gives the chain, which never settles."""
  _, log, root = born()
  two = await chained("two")
  assert (root, two) == ("chain1", "chain2")
  assert engine.peek(root) is None and engine.peek(two) is None
  assert [a for a in said(log, "done") if a[1] in (root, two)] == []


async def test_the_open_of_a_chain_with_a_source_binds_its_label_and_its_source() -> None:
  """The open of a chain with a source binds its label and its source."""
  _, _, root = born()
  twin = engine.chain("twin", source=root)
  bare = engine.chain(source=root)
  await settle()
  opens = "#chain2\nchain2_label = 'twin'\nchain2_source = 'chain1'\nchain2: Act[object] = Act('chain2')"
  assert paragraphs(engine.turns(on=twin))[-1] == opened(twin, "twin", root) == opens
  opens = "#chain3\nchain3_source = 'chain1'\nchain3: Act[object] = Act('chain3')"
  assert paragraphs(engine.turns(on=bare))[-1] == opened(bare, "", root) == opens


async def test_the_entry_that_opens_a_chain_with_a_source_carries_its_label_after_the_prefix() -> None:
  """The entry that opens a chain with a source carries its label, after the prefix."""
  _, _, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  was = list(engine.transcript(root))
  twin = await chained("twin", root)
  opens = ("tell", twin, twin, opened(twin, "twin", root).rsplit("\n", 1))
  assert engine.transcript(twin)[: len(was) + 2] == [*was, ("started", twin, twin), opens]


async def test_chain_gives_the_new_chain_which_never_completes() -> None:
  """chain gives the new chain, which never completes."""
  _, log, root = born()
  twin = await chained("twin", root)
  assert twin == "chain2" and engine.peek(twin) is None
  assert [a for a in said(log, "done") if a[1] == twin] == []


async def test_boot_gives_the_root_as_an_act_of_never_and_the_root_never_completes() -> None:
  """boot gives the root as an act of Never, and the root never completes."""
  sand, _, root = born()
  await engine.rung("k = 1", on=root)
  await settle()
  one = engine.boot(list(sand.record), kernel=Py().kernel(), gate=Py().gating(), world=Sand().hears())
  await settle(300)
  assert one == root and engine.peek(one) is None


async def test_the_engine_makes_a_chain_in_one_way_by_running_its_ladder() -> None:
  """The engine makes a chain in one way: by running its rungs."""
  _, log, root = born()
  laid = engine.rung("k = 1", on=root)
  await laid
  twin = await chained("twin", root)
  told = fresh(root, written(laid, "k = 1"))
  assert ran(log) == [told, "k = 1", opened(twin, "twin", root), told, "k = 1"] and engine.module(twin)["k"] == 1
  await engine.rung("later = 2", on=root)
  assert "later" not in engine.module(twin)


async def test_in_a_chain_with_a_source_the_ladder_of_the_origin_runs_again_in_its_module() -> None:
  """In a chain with a source the rungs of the origin that are over at that source run again in the module of the new chain."""
  _, log, root = born("a = 1", "close(a + 1)")
  one = engine.prompt(int, "count", on=root)
  assert await one == 2
  await settle()
  twin = await chained("twin", root)
  first, second = steps(log, one)
  told = [
    fresh(root, prompted(one, "int", "count"), f"#{first} advance on {one}"),
    f"#{first} told nothing\n\n#{second} advance on {one}",
  ]
  words = [told[0], "a = 1", told[1], "close(a + 1)"]
  theirs = [opened(twin, "twin", root), *words]
  assert [(a[3], a[5]) for a in said(log, "run")] == [(root, word) for word in words] + [
    (twin, word) for word in theirs
  ]
  assert engine.module(twin)["a"] == 1 and engine.module(twin)["__name__"] == twin


async def test_a_chain_given_a_source_stands_on_that_one() -> None:
  """A chain given a source stands on that one: it retells the words of it as they stand, each rung of it retelling a rung of that one, so that it makes the same acts and shares them, and what it holds of the transcript of that one is what its filter kept, though it runs every word all the same, so what it holds bound is more than its turns say."""
  _, log, root = born("x = bash('echo hi')\nn = (await x).code\nclose(1)", "close(None)")
  assert await engine.prompt(int, "run it", on=root) == 1
  assert await engine.prompt(None, "look", on=root) is None
  await settle()
  command = said(log, "bash")[0][1]
  program = engine.program(root)
  narrow = engine.chain("narrow", source=root, filter=take(command, inside=False))
  await settle()
  retold = [a for a in said(log, "rung") if a[3] == narrow]
  assert [(a[4], a[5]) for a in retold] == [(word, one) for one, word in program.items()]
  assert [a[1] for a in said(log, "bash")] == [command]
  assert engine.module(narrow)["n"] == 0 and engine.module(narrow)[command] == command
  assert command in named(engine.turns(on=root)) and command not in named(engine.turns(on=narrow))


async def test_a_prompt_to_an_actor_its_roster_does_not_hold_it_closes_with_the_refusal() -> None:
  """A prompt to an actor its roster does not hold it closes with the refusal."""
  _, _, root = born()
  for to in ("ghost", "n/high"):
    one = engine.prompt(int, "hi", to=to, on=root)
    await settle()
    got = engine.peek(one)
    assert isinstance(got, Refused) and str(got) == f"{to} no actor"


async def test_a_chain_that_the_word_of_a_rung_opens_is_a_scope_of_its_own() -> None:
  """A chain that the word of a rung opens is a scope of its own: its words are on itself, though the chain fact itself stands on the chain of the rung that opened it; and when that rung is retold, the word makes the same chain, since a rung that retells another shares the acts it makes, so a word that opens a chain opens it once."""
  sand, log, root = born("side = chain('side', source=__name__)\nclose(side)", "close(None)")
  side = await engine.prompt(str, "fork one", on=root)
  await settle()
  twin = await chained("twin", root, 200)
  assert [a[1] for a in said(log, "chain")] == [root, side, twin]
  assert [a[3] for a in said(log, "chain") if a[1] == side] == [root]
  sand.script[side] = ["x = bash('echo hi')\nclose((await x).code)", "close(None)"]
  assert await engine.prompt(int, "count", on=side) == 0
  assert [a[3] for a in said(log, "bash")] == [side]


async def test_the_engine_refuses_a_prompt_to_an_actor_outside_the_roster() -> None:
  """The engine refuses a prompt to an actor outside the roster."""
  _, log, root = born()
  one = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  got = engine.peek(one)
  assert isinstance(got, Refused) and str(got) == "ghost no actor" and said(log, "reply") == []


async def test_the_chain_holds_the_control_it_says_itself() -> None:
  """The chain holds the control it says itself, which tells nothing, since a control from inside a chain tells nothing to it."""
  sand, log, root = born()
  ghost = engine.prompt(int, "hi", to="ghost", on=root)
  await settle()
  assert isinstance(engine.peek(ghost), Refused)
  sand.script[root] = ["k = BAD", "close(1)"]
  one = engine.prompt(int, "try", on=root)
  assert await one == 1
  step, last = steps(log, one)
  held = engine.transcript(root)
  assert [(a[1], a[2]) for a in said(held, "close")] == [(ghost, root), (step, root), (one, last)]
  assert [a for a in said(held, "tell") if a[3][0].split("\n")[0].split(" ")[1:2] == ["closed"]] == []
  assert [p for p in paragraphs(engine.turns(on=root)) if p.split("\n")[0].split(" ")[1:2] == ["closed"]] == []


async def test_the_engine_binds_the_exception_of_every_raise_but_a_cancellederror() -> None:
  """The engine binds the exception of every raise but a CancelledError in the globals of the chain, under the name raised."""
  _, _, root = born("raise ValueError('boom')", "close(1)", "close(None)")
  assert await engine.prompt(int, "try", on=root) == 1
  assert isinstance(engine.module(root)["raised"], ValueError)
  assert str(engine.module(root)["raised"]) == "boom"
  with pytest.raises(KeyboardInterrupt):
    await engine.rung("raise KeyboardInterrupt('stop')", on=root)
  assert isinstance(engine.module(root)["raised"], KeyboardInterrupt)
  with pytest.raises(CancelledError):
    await engine.rung("w = wait(5)\ncancel(w)\nawait w", on=root)
  assert isinstance(engine.module(root)["raised"], KeyboardInterrupt)


async def test_the_engine_binds_the_exception_again_at_each_raise() -> None:
  """The engine binds the exception again at each raise but a CancelledError."""
  _, _, root = born("raise ValueError('one')", "raise KeyError('two')", "close(1)", "close(None)")
  assert await engine.prompt(int, "try", on=root) == 1
  assert isinstance(engine.module(root)["raised"], KeyError)
  with pytest.raises(CancelledError):
    await engine.rung("w = wait(5)\ncancel(w)\nawait w", on=root)
  assert isinstance(engine.module(root)["raised"], KeyError)


async def test_a_chain_its_module_which_is_the_engine_itself_named_for_the_chain() -> None:
  """A chain: its module, which is the engine itself, named for the chain, since everything the file defines is the model's to call and nothing of it is bound to one chain, and what the operator would add to it, it makes a rung of, which binds it, stands in the program and is said again in a later life, a fact said from a run being on the chain of that run, and a word that wants a chain of its own giving the name of its own as the source."""
  sand, log, root = born()
  assert engine.module(root)["__name__"] == root and engine.module(root)["bash"] is engine.bash
  laid = engine.rung("k = 21", on=root)
  await laid
  program = engine.program(root)
  assert list(program.values()) == [fresh(root, written(laid, "k = 21")), "k = 21"] and engine.module(root)["k"] == 21
  sand.script[root] = ["x = bash('echo hi')\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  assert said(log, "bash")[0][3] == root
  _, over = life(Sand(), list(sand.record))
  await settle(300)
  assert engine.module(over)["k"] == 21
  own = await engine.rung("close(chain('own', source=__name__))", on=over)
  assert isinstance(own, str) and engine.get(own)[4:] == ("own", over)


async def test_a_chain_is_chain_n_whether_boot_or_chain_opened_it_and_the_root_is_chain1() -> None:
  """A chain is chainN whether boot or chain opened it, and the root is chain1."""
  sand, log, root = born()
  two = engine.chain("two")
  sand.script[root] = ["side = chain('side')\nclose(side)"]
  one = engine.prompt(str, "fork one", on=root)
  theirs = await one
  assert (root, two, theirs) == ("chain1", "chain2", "chain3")
  assert [a[1:3] for a in said(log, "chain")] == [(root, OPERATOR), (two, OPERATOR), (theirs, steps(log, one)[0])]


async def test_a_chain_is_an_act_of_never_so_the_chain_never_completes() -> None:
  """A chain is an act of Never, so the chain never completes."""
  _, log, _ = born()
  two = await chained("two")
  assert engine.peek(two) is None and [a for a in said(log, "done") if a[1] == two] == []


async def test_the_turns_of_a_chain_tell_the_standing_and_the_acts_of_the_operator() -> None:
  """The turns of a chain tell the standing and the acts of the operator."""
  _, _, root = born()
  step = engine.rung("k = 1", on=root)
  assert await step is None
  one = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert paragraphs(engine.turns(on=root)) == [
    opened(root, "root"),
    takes(root),
    written(step, "k = 1"),
    prompted(one, "int", "how many?"),
  ]


async def test_the_transcript_of_a_chain_is_the_facts_on_it_in_the_order_it_heard_them() -> None:
  """The transcript of a chain is the facts on it, in the order they were said."""
  _, log, root = born("close(1)")
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  await settle()
  held = engine.transcript(root)
  assert all(on(a) == root for a in held)
  assert ids(held) == ["stand1", one, *steps(log, one), "run1", "reply1", "gate1", "run2"]
  facts, log = lasting(held), lasting(log)
  heard = [log.index(a) for a in facts if not engine.question(a)]
  assert heard == sorted(heard) and [a for a in facts if a not in log] == []


async def test_nothing_leaves_a_transcript_once_the_transcript_holds_it() -> None:
  """Nothing leaves a transcript once the transcript holds it, but by a prefix said on its chain, which takes the place of what the transcript held."""
  _, _, root, one = await posed()
  was = list(engine.transcript(root))
  engine.cancel(one)
  await settle()
  now = engine.transcript(root)
  assert now[: len(was)] == was and len(now) > len(was)
  kept = [a for a in now if a[1] != one]
  engine.say("prefix", root, kept)
  assert [a[:2] for a in engine.transcript(root)] == [a[:2] for a in kept] != [a[:2] for a in now]


async def test_an_assistant_turn_keeps_the_role_assistant_in_every_chain_made_from_the_chain() -> None:
  """An assistant turn keeps the role assistant in every chain made from the chain."""
  _, _, root = born("close(1)", "close(None)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  twin = await chained("twin", root)
  assert [turn[0] for turn in engine.turns(on=twin)] == [turn[0] for turn in engine.turns(on=root)]
  assert [turn[0] for turn in engine.turns(on=twin)].count("assistant") == 1


async def test_the_engine_adds_the_acts_that_caused_a_kept_act_to_what_the_filter_kept() -> None:
  """The engine adds the acts that caused a kept act to what the filter kept."""
  _, log, root = born("x = bash('echo hi')\nclose(1)", "close(None)")
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command = said(log, "bash")[0]
  one, ack = [a[1] for a in said(log, "prompt")]
  step = command[2]
  twin = engine.chain("twin", source=root, filter=take(command[1]))
  await settle()
  assert steps(log, one) == [step] and ack in named(engine.turns(on=root))
  assert named(engine.turns(on=twin)) == [root, root, one, step, command[1], command[1], twin]


async def test_the_globals_of_a_chain_whose_filter_is_a_take_that_is_not_inside() -> None:
  """The globals of a chain whose filter is a take that is not inside hold the bindings of the skipped words still."""
  _, log, root = born("x = bash('echo hi')\nn = (await x).code", "close(n)")
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
  assert (engine.module(narrow)["x"], engine.module(narrow)["n"]) == (command, 0)


async def test_a_chain_with_a_source_made_after_the_rung_that_defined_a_door_has_the_door_too() -> None:
  """A chain with a source made after the rung whose word defined a door has the door too."""
  _, _, root = born(DOOR, "close(None)")
  assert await engine.prompt(int, "a door", on=root) == 1
  await settle()
  twin = await chained("twin", root)
  assert engine.read("note://a", on=twin).content == "kept"


async def test_the_globals_of_a_chain_are_those_of_a_module_named_by_the_id_of_the_chain() -> None:
  """The globals of a chain are those of a module named by the id of the chain."""
  _, _, root = born()
  two = engine.chain("two")
  assert engine.module(root)["__name__"] == root and engine.module(two)["__name__"] == two
  await engine.rung("k = 1", on=root)
  assert "k" in engine.module(root) and "k" not in engine.module(two)


async def test_every_name_that_the_file_defines_is_in_the_globals_of_a_chain() -> None:
  """Every name that the file defines is in the globals of a chain."""
  _, _, root = born()
  assert defined(Path(furb.python.__file__).read_text(encoding="utf-8")) <= set(engine.module(root))


async def test_a_chain_holds_whole_every_act_made_on_the_chain() -> None:
  """A chain holds whole every act made on the chain, the rungs it makes itself among them, which tell nothing."""
  _, log, root = born("t = read('a.txt')\nx = bash('echo hi')\nclose(1)", "close(None)")
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  await settle()
  held = engine.transcript(root)
  mine = [a for a in held if engine.question(a)]
  assert [a for a in acts(log).values() if a[3] == root and a not in mine] == []
  kinds = ["stand", "prompt", "rung", "run", "reply", "gate", "run", "read", "bash", "merged"]
  assert [a[0] for a in mine] == [*kinds, "prompt", "rung", "run", "reply", "gate", "run"]
  engine.write(Text(act, ""), on=root)
  await settle()
  held = engine.transcript(root)
  assert [a for a in acts(log).values() if a[3] == root and a not in held] == []
  own = [a[1] for a in held if a[0] == "rung" and a[2] == root]
  told = [
    "#read1\nread1_path = 'a.txt'\n<s:read1_text>\none\ntwo</s:read1_text>",
    f"#bash1\n{binding('bash1', 'Exit')}",
    prompted("prompt2", "None", "bash1 done", "prompt1"),
    "#bash1 exited 0\nbash1_stdout = 'ran echo hi'",
    "#rung2 advance on prompt2",
  ]
  assert own == ["rung3", "rung4", "rung5"] and [engine.get(one)[4:6] for one in own] == [
    (fresh(root, prompted(act, "int", "work"), "#rung1 advance on prompt1"), "rung1_told"),
    ("\n\n".join(told), "rung2_told"),
    ("close(None)", "rung2"),
  ]
  assert [a for a in held if a[1] in own and a[0] == "tell"] == []
  assert [a for a in held if a[0] == "bash"] == said(log, "bash")


async def test_a_chain_holds_every_fact_on_it_and_an_act_on_another_chain_is_that_chains() -> None:
  """A chain holds every fact on it, and an act on another chain is that chain's."""
  sand, log, root = born()
  two = engine.chain("two")
  sand.script[root] = [f"p = prompt(int, 'hi', on={two!r})\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "delegate", on=root) == 1
  await settle()
  theirs = said(log, "prompt")[-1]
  held, ours = engine.transcript(two), engine.transcript(root)
  on_two = [a for a in log if (a[3] if engine.question(a) else engine.scope(a[1])) == two]
  assert sorted(lasting(held), key=repr) == sorted(lasting(on_two), key=repr)
  assert theirs in held
  assert theirs not in ours


async def test_a_prompt_that_a_step_of_another_chain_made_reads_nothing_of_that_chain() -> None:
  """A prompt that a step of another chain made reads nothing of that chain."""
  sand, log, root = born()
  two = engine.chain("two")
  sand.script[root] = [f"secret = 1\nclose(await prompt(int, 'count', on={two!r}))", "close(None)"]
  sand.script[two] = ["close(2)"]
  assert await engine.prompt(int, "delegate", on=root) == 2
  await settle()
  theirs = next(a for a in said(log, "reply") if a[3] == two)
  asked = next(a[1] for a in said(log, "prompt") if a[3] == two)
  assert paragraphs(sand.turns[theirs[1]]) == [
    opened(two, "two"),
    takes(two),
    prompted(asked, "int", "count"),
    f"#{theirs[2]} advance on {asked}",
  ]


async def test_the_word_of_a_rung_rebinds_the_default_actor_like_any_name() -> None:
  """The word of a rung rebinds the default actor like any name, and so does an answer of a stand that changes the actor of the standing, and the last binding in record order wins."""
  sand, log, root = born("actor = 'n/low'\nclose(1)", "close(None)")
  assert await engine.prompt(int, "rebind", on=root) == 1
  await settle()
  assert engine.module(root)["actor"] == "n/low"
  sand.script[root] = ["close(2)", "close(None)"]
  assert await engine.prompt(int, "again", on=root) == 2
  assert said(log, "reply")[-1][4] == "n/low"
  await relived(Sand(), list(sand.record))
  assert engine.module(root)["actor"] == "n/low"
  await relived(Sand(stands=[STANDS[0], "/v", STANDS[2]]), list(sand.record))
  assert engine.module(root)["actor"] == "n/low"
  await relived(Sand(stands=[STANDS[0], "/w", "m/high"]), list(sand.record))
  assert engine.module(root)["actor"] == "m/high"


async def test_the_transcript_of_the_root_begins_with_the_open_of_the_root_and_then_the_standing() -> None:
  """The transcript of the root begins with the open of the root and then the standing."""
  _, _, root = born()
  held = [a for a in engine.transcript(root) if a[0] == "tell"]
  opens = opened(root, "root").rsplit("\n", 1)
  assert held[:2] == [("tell", root, root, opens), ("tell", root, root, [takes(root)])]
  assert paragraphs(engine.turns(on=root))[:2] == [opened(root, "root"), takes(root)]


async def test_the_transcript_then_holds_the_prompt_of_the_operator_and_the_replies_of_the_model() -> None:
  """The transcript then holds the prompt of the operator, and the replies of the model, each with its turn."""
  _, log, root = born("a = 1", "close(a)")
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  await settle()
  held = engine.transcript(root)
  assert [a[1] for a in held if a[0] == "prompt"] == [one]
  replies = [a[1] for a in held if a[0] == "reply"]
  assert [a[2] for a in held if a[0] == "reply"] == steps(log, one)
  assert [a[3][1] for a in said(held, "done") if a[1] in replies] == ["a = 1", "close(a)"]


async def test_the_engine_reads_its_own_names_through_the_globals_of_the_chain() -> None:
  """The engine reads its own names through the globals of the chain."""
  sand, _, root = born(files={"/w/mine.txt": "mine\n"})
  assert engine.module(root)["read"] is engine.read
  await engine.rung("def read(path, show=HEAD, on=''):\n  return ask('read', __name__, 'mine.txt')", on=root)
  sand.script[root] = ["close(read('any.txt').content)", "close(None)"]
  assert await engine.prompt(str, "read it", on=root) == "mine\n"
  assert [a[4] for a in sand.calls if a[0] == "read"] == ["mine.txt"]


async def test_the_engine_uses_a_rebound_name_from_the_next_use_on() -> None:
  """The engine uses a rebound name from the next use on."""
  sand, _, root = born(
    "was = read('a.txt').content",
    "def read(path, show=HEAD, on=''):\n  return ask('read', __name__, 'mine.txt')",
    "close([was, read('a.txt').content])",
    "close(None)",
    files={"/w/a.txt": "one\ntwo\n", "/w/mine.txt": "mine\n"},
  )
  assert await engine.prompt(list, "read it", on=root) == ["one\ntwo\n", "mine\n"]
  assert [a[4] for a in sand.calls if a[0] == "read"] == ["a.txt", "mine.txt"]


async def test_no_close_reaches_a_chain_since_a_chain_never_completes() -> None:
  """No close reaches a chain, since a chain never completes."""
  sand, _, _ = born()
  two = engine.chain("two")
  engine.close(5, two)
  await settle()
  assert engine.peek(two) is None
  sand.script[two] = ["close(1)", "close(None)"]
  assert await engine.prompt(int, "still here", on=two) == 1


async def test_to_await_a_chain_never_returns() -> None:
  """To await a chain never returns."""
  _, _, _ = born()
  two = engine.chain("two")
  with pytest.raises(TimeoutError):
    await asyncio.wait_for(asyncio.shield(two), 0.01)


async def test_a_chain_has_a_globals_dict_and_a_working_directory_of_its_own() -> None:
  """A chain has a globals dict and a working directory of its own."""
  _, _, root = born()
  two = engine.chain("two")
  await engine.rung("here = 1", on=root)
  engine.cd("/deep", on=two)
  assert (engine.cwd(on=root), engine.cwd(on=two)) == ("/w", "/deep")
  assert "here" in engine.module(root) and "here" not in engine.module(two)


async def test_a_chain_with_a_source_holds_the_acts_it_inherited_as_the_filter_kept_them() -> None:
  """A chain with a source holds the acts it inherited from that source, as the filter kept them."""
  _, log, root = born("x = bash('echo hi')\ny = bash('echo there')\nclose(1)", "close(None)")
  assert await engine.prompt(int, "run them", on=root) == 1
  await settle()
  one, two = said(log, "bash")
  twin = engine.chain("twin", source=root, filter=take(two[1], inside=False))
  await settle()
  held = engine.transcript(twin)
  assert one in held and two not in held


async def test_a_chain_with_a_source_holds_whole_each_fact_of_its_origin_about_a_name_that_no_act_holds() -> None:
  """A chain with a source holds whole each fact of its origin about a name that no act of the life holds, as a turn that its origin carried from another record."""
  _, _, root = born()
  await settle()
  turn = ("assistant", "x = 1", (1, 2, 3, 4, 0.5), None)
  carried = [
    ("tell", "reply0", "carry9", ["#bash40 exited 0"]),
    ("reply", "reply0", "carry9", root, "m"),
    ("done", "reply0", "carry9", turn),
    ("reply", "reply77", "rung70", root, "m"),
    ("done", "reply77", "provider", turn),
    ("tell", "bash40", "bash40", ["#bash40"]),
  ]
  engine.say("prefix", root, [*engine.transcript(root), *carried])
  twin = engine.chain("twin", source=root, filter=take("chain0"))
  await settle()
  assert [a for a in engine.transcript(twin) if a in carried] == carried


async def test_the_steps_that_an_inherited_prompt_takes_after_the_point_enter_its_owner_alone() -> None:
  """The steps that an inherited prompt takes after the point enter the transcript of its owner alone."""
  _, log, root = born("a = 1", "close(a + 1)")
  one = engine.prompt(int, "count", on=root)
  engine.pause(one)
  await settle()
  twin = await chained("twin", root)
  engine.wake(one)
  await settle()
  assert (await one) == 2
  first, *later = steps(log, one)
  replies = [a[1] for a in said(log, "reply")]
  held = engine.transcript(twin)
  assert later and [(a[0], a[1]) for a in held if a[:3:2] == ("rung", one) or a[1] in replies] == [
    ("rung", first),
    ("reply", replies[0]),
    ("started", replies[0]),
    ("done", replies[0]),
  ]
  assert [a for a in held if a[1] in later] == []
  theirs = engine.transcript(root)
  assert [a[2] for a in theirs if a[0] == "reply"] == [first, *later]


async def test_what_a_chain_binds_is_its_own_and_a_chain_with_a_source_is_how_it_gets_isolation() -> None:
  """What a chain binds is its own, and a chain with a source is how a chain gets isolation."""
  _, _, root = born()
  await engine.rung("k = 1", on=root)
  twin = await chained("twin", root)
  await engine.rung("k = 2", on=twin)
  assert (engine.module(root)["k"], engine.module(twin)["k"]) == (1, 2)


async def test_two_chains_from_one_source_hold_the_same_values() -> None:
  """Two chains from one source hold the same values, since both ran the same rungs."""
  _, _, root = born()
  await engine.rung("k = 21", on=root)
  one = engine.chain("one", source=root)
  two = engine.chain("two", source=root)
  await settle()
  assert engine.module(one)["k"] == engine.module(two)["k"] == 21


async def test_a_chain_with_a_source_inherits_the_default_actor_with_the_globals_of_its_origin() -> None:
  """A chain with a source inherits the default actor with the globals of its origin at that source."""
  _, _, root = born()
  twin = await chained("twin", root)
  assert engine.module(twin)["actor"] == engine.module(root)["actor"] == "m/low"
  _, _, root = born("actor = 'n/low'\nkid = chain('kid')\nclose(kid)", "close(None)")
  kid = await engine.prompt(str, "open one", on=root)
  twin = await chained("twin", kid)
  assert engine.module(twin)["actor"] == engine.module(kid)["actor"] == "n/low"


async def test_a_chain_with_no_source_stands_on_the_default_actor_of_the_chain_it_is_on() -> None:
  """A chain with no source stands on the default actor of the chain it is on when it is made, which for a chain that a word opens is the chain of that word, and on the actor of the standing when it is on no chain, and its standing tells that actor."""
  sand, _, root = born("actor = 'n/low'\nkid = chain('kid')\nclose(kid)", "close(None)")
  kid = await engine.prompt(str, "open one", on=root)
  ours = await chained("ours")
  assert [engine.module(one)["actor"] for one in (root, kid, ours)] == ["n/low", "n/low", "m/low"]
  assert paragraphs(engine.turns(on=kid))[1] == takes(kid, [*STANDS[:2], "n/low"])
  assert paragraphs(engine.turns(on=ours))[1] == takes(ours)
  await relived(Sand(), plain(sand.record))
  assert [engine.module(one)["actor"] for one in (root, kid, ours)] == ["n/low", "n/low", "m/low"]


async def test_a_chain_with_a_source_that_holds_an_act_reads_the_close_of_the_act() -> None:
  """A chain with a source that holds an act reads the close of the act in its transcript."""
  _, log, root = born("x = bash('echo hi')\nclose((await x).code)", "close(None)")
  assert await engine.prompt(int, "run it", on=root) == 0
  await settle()
  command = said(log, "bash")[0][1]
  twin = await chained("twin", root)
  shut = [one for one in paragraphs(engine.turns(on=twin)) if one.startswith(f"#{command} exited")]
  assert shut == [f"#{command} exited 0\n{command}_stdout = 'ran echo hi'"]


async def test_a_chain_with_a_source_awaits_or_peeks_an_inherited_act_as_it_likes() -> None:
  """A chain with a source awaits or peeks an inherited act as it likes."""
  sand, log, root = born("x = bash('echo hi')\nclose(1)", "close(None)")
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  twin = await chained("twin", root)
  sand.script[twin] = ["out = await x\nseen = peek(x)\nassert isinstance(seen, Exit)\nclose([out.code, seen.code])"]
  assert await engine.prompt(list, "look at it", on=twin) == [0, 0]
  assert [a[1] for a in said(log, "bash")] == [command]


async def test_the_engine_does_not_wake_a_chain_with_a_source_for_an_inherited_act() -> None:
  """The engine does not wake a chain with a source for the result of an inherited act."""
  sand, log, root = born("x = bash('slow')\nclose(1)", "close(None)", auto=False)
  assert await engine.prompt(int, "start one", on=root) == 1
  await settle()
  command = said(log, "bash")[0][1]
  twin = await chained("twin", root)
  before = len([a for a in said(log, "reply") if a[3] == twin])
  sand.exits(command, 0)
  await settle()
  assert len([a for a in said(log, "reply") if a[3] == twin]) == before
  assert [a for a in said(log, "prompt") if a[2] == twin] == []


async def test_the_globals_of_a_chain_with_a_source_may_hold_more_than_its_turns_say() -> None:
  """The globals of a chain with a source may hold more than its turns say."""
  _, log, root = born("x = bash('echo hi')\nn = (await x).code\nclose(1)", "close(None)")
  assert await engine.prompt(int, "run it", on=root) == 1
  assert await engine.prompt(None, "look", on=root) is None
  await settle()
  command = said(log, "bash")[0][1]
  narrow = engine.chain("narrow", source=root, filter=take(command, inside=False))
  await settle()
  assert command not in named(engine.turns(on=narrow))
  assert [one for one in paragraphs(engine.turns(on=narrow)) if binding(command, "Exit") in one] == []
  assert (engine.module(narrow)["n"], engine.module(narrow)["x"], engine.module(narrow)[command]) == (
    0,
    command,
    command,
  )


async def test_the_transcript_of_a_chain_with_a_source_holds_the_entries_up_to_that_source_first() -> None:
  """The transcript of a chain with a source holds the entries up to that source first, then the entry that opened it."""
  _, _, root = born()
  await engine.rung("k = 1", on=root)
  was = list(engine.transcript(root))
  twin = await chained("twin", root)
  opens = ("tell", twin, twin, opened(twin, "twin", root).rsplit("\n", 1))
  assert engine.transcript(twin)[: len(was) + 2] == [*was, ("started", twin, twin), opens]
  assert all(on(a) == root for a in was)


async def test_the_acts_of_the_prefix_of_a_chain_with_a_source_keep_the_ids_they_had() -> None:
  """The acts of the prefix of a chain with a source keep the ids they had on the origin."""
  _, _, root = born("x = bash('echo hi')\nclose(1)", "close(None)")
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  held = engine.transcript(root)
  was = ids(held)
  twin = await chained("twin", root)
  now = engine.transcript(twin)
  assert ids(now)[: len(was)] == was


async def test_the_globals_of_a_chain_with_a_source_are_those_of_a_module_of_its_own() -> None:
  """The globals of a chain with a source are those of a module of its own."""
  _, _, root = born()
  await engine.rung("k = 1", on=root)
  twin = await chained("twin", root)
  assert engine.module(twin) is not engine.module(root)
  assert engine.module(twin)["__name__"] == twin and engine.module(twin)["k"] == 1


async def test_the_objects_of_a_rung_of_a_chain_with_a_source_are_that_chains_own() -> None:
  """The objects of a rung of a chain with a source are that chain's own, made again, and only code is shared."""
  _, _, root = born()
  await engine.rung("marks = [1]\ndef twice(x):\n  return x * 2", on=root)
  twin = await chained("twin", root)
  assert engine.module(twin)["marks"] == engine.module(root)["marks"]
  assert engine.module(twin)["marks"] is not engine.module(root)["marks"]
  assert engine.module(twin)["twice"] is not engine.module(root)["twice"]
  mine, theirs = engine.program(twin), engine.program(root)
  assert isinstance(mine, dict) and isinstance(theirs, dict)
  assert list(mine.values()) == [opened(twin, "twin", root), *theirs.values()] and list(mine)[1:] == list(theirs)


async def test_the_globals_of_a_chain_with_a_source_are_the_origins_whatever_the_filter_kept() -> None:
  """The globals of a chain with a source are what the rungs of the origin that are over at that source bound, whatever the filter kept."""
  _, _, root = born()
  await engine.rung("k = 21", on=root)
  twin = engine.chain("twin", source=root, filter=take())
  await settle()
  assert engine.module(twin)["k"] == 21
  assert paragraphs(engine.turns(on=twin)) == [opened(root, "root"), takes(root), opened(twin, "twin", root)]


async def test_a_chain_with_a_source_inherits_the_words_its_caller_wrote_with_its_prefix() -> None:
  """A chain with a source inherits the words its caller wrote with its prefix, wherever its source."""
  sand, _, root = born()
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["j = k + 1\nclose(j)", "close(None)"]
  assert await engine.prompt(int, "count", on=root) == 2
  await engine.rung("m = 3", on=root)
  twin = await chained("twin", root)
  assert (engine.module(twin)["k"], engine.module(twin)["j"], engine.module(twin)["m"]) == (1, 2, 3)


async def test_a_filter_that_skips_a_rung_its_caller_wrote_keeps_it_out_of_the_turns() -> None:
  """A filter that skips a rung its caller wrote keeps it out of the turns of the new chain."""
  _, _, root = born()
  laid = engine.rung("k = 1", on=root)
  await laid
  twin = engine.chain("twin", source=root, filter=take(laid, inside=False))
  await settle()
  assert engine.module(twin)["k"] == 1
  assert paragraphs(engine.turns(on=root)) == [opened(root, "root"), takes(root), written(laid, "k = 1")]
  assert paragraphs(engine.turns(on=twin)) == [opened(root, "root"), takes(root), opened(twin, "twin", root)]


async def test_a_chain_with_a_source_holds_the_classes_its_origin_defined_before_that_source() -> None:
  """A chain with a source holds the classes that its origin defined before that source."""
  _, _, root = born()
  await engine.rung("class Plan:\n  n = 21\n\nmine = Plan().n", on=root)
  twin = await chained("twin", root)
  assert isinstance(engine.module(twin)["Plan"], type) and engine.module(twin)["mine"] == 21
  assert engine.module(twin)["Plan"] is not engine.module(root)["Plan"]


async def test_a_chain_with_a_source_reads_nothing_that_its_origin_did_after_that_source() -> None:
  """A chain with a source reads nothing that its origin did after that source."""
  _, _, root = born()
  twin = await chained("twin", root)
  told = engine.turns(on=twin)
  after = engine.rung("after = 1", on=root)
  await after
  await settle()
  assert engine.turns(on=twin) == told == [("user", "\n\n".join(paragraphs(told)), None, None)]
  assert paragraphs(told) == [opened(root, "root"), takes(root), opened(twin, "twin", root)]
  assert "after" not in engine.module(twin) and paragraphs(engine.turns(on=root))[-1:] == [written(after, "after = 1")]


async def test_a_chain_with_a_source_retells_no_rung_of_its_origin_that_is_not_over_at_its_birth() -> None:
  """A chain with a source retells no rung of its origin that is not over at its birth, so the word of that rung goes on in its origin alone, and no copy of it waits for it or runs it a second time."""
  _, log, root = born()
  await engine.rung("k = 1", on=root)
  slow = engine.rung("x = 1\ny = await prompt(int, 'n', to=OPERATOR)\nz = y", on=root)
  await settle()
  twin = await chained("twin", root)
  (asked,) = [a[1] for a in said(log, "prompt")]
  engine.close(4, asked)
  await settle()
  assert [a for a in said(log, "rung") if a[5] == slow] == []
  assert engine.module(root)["z"] == 4 and engine.module(twin)["k"] == 1
  assert not {"x", "y", "z"} & set(engine.module(twin))


async def test_what_a_rung_of_a_chain_with_a_source_binds_lands_on_that_chain() -> None:
  """What a rung of a chain with a source binds lands on that chain and not on its origin."""
  _, _, root = born()
  twin = await chained("twin", root)
  await engine.rung("mine = 1", on=twin)
  assert engine.module(twin)["mine"] == 1 and "mine" not in engine.module(root)


async def test_the_origin_holds_the_chain_fact_of_a_chain_that_a_rung_of_it_made() -> None:
  """The origin holds the chain fact of a chain that a rung of it made, and nothing of that chain's own."""
  _, _, root = born("side = chain('side', source=__name__)\nclose(side)", "close(None)")
  side = await engine.prompt(str, "fork one", on=root)
  await settle()
  held = engine.transcript(root)
  assert [a[1] for a in held if a[0] == "chain"] == [side]
  assert [a for a in held if on(a) == side] == []


async def test_two_names_of_the_module_are_the_chains_to_bind() -> None:
  """Two names of the module are the chain's to bind: the actor it stands on, and what the last rung that raised raised, so that the word of a rung reads what the word before it came to."""
  _, _, root = born("raise ValueError('boom')", "close([actor, str(raised)])", "close(None)")
  assert await engine.prompt(list, "try", on=root) == ["m/low", "boom"]


async def test_it_holds_the_transcript_of_its_origin_first_and_tells_its_own_open_after_it() -> None:
  """It holds the transcript of its origin first and tells its own open after it, and the rungs it runs again tell nothing, since every word of those stands in what it inherited."""
  _, log, root = born()
  step = engine.rung("k = 1", on=root)
  await step
  was = list(engine.transcript(root))
  twin = await chained("twin", root)
  held = engine.transcript(twin)
  assert isinstance(was, list) and isinstance(held, list)
  opens = ("tell", twin, twin, opened(twin, "twin", root).rsplit("\n", 1))
  assert held[: len(was) + 2] == [*was, ("started", twin, twin), opens]
  retold = [a[1] for a in said(log, "rung") if a[3] == twin]
  assert [engine.get(one)[5] for one in retold] == [f"{step}_told", step]
  assert [a for a in held if a[1] in retold and a[0] == "tell"] == []
  assert paragraphs(engine.turns(on=twin)) == [
    opened(root, "root"),
    takes(root),
    written(step, "k = 1"),
    opened(twin, "twin", root),
  ]


async def test_it_hears_no_control_that_ends_it() -> None:
  """It hears no control that ends it, since nothing that happens to a chain ends it, and a control over a chain is over the acts on it, which end themselves."""
  sand, _, _ = born()
  two = engine.chain("two")
  one = engine.prompt(int, "one", to=OPERATOR, on=two)
  await settle()
  engine.cancel(two)
  await settle()
  assert isinstance(engine.peek(one), CancelledError) and engine.peek(two) is None
  sand.script[two] = ["close(1)", "close(None)"]
  assert await engine.prompt(int, "still here", on=two) == 1


async def test_every_done_it_holds_answers_a_question_it_holds() -> None:
  """Every done it holds answers a question it holds, since it holds every question made on it, the ones it made itself among them."""
  _, _, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  await settle()
  held = engine.transcript(root)
  names = {a[1] for a in held if engine.question(a)}
  assert [a[1] for a in held if a[0] == "done" and a[1] not in names] == []
  assert [a[1] for a in held if engine.question(a) and a[2] == root] == ["run1", "gate1", "run2"]
  assert [a[1] for a in held if a[0] == "done" and a[1] in ("run1", "gate1", "run2")] == ["run1", "gate1", "run2"]


async def test_what_a_chain_with_a_source_holds_of_the_transcript_of_its_origin() -> None:
  """What a chain with a source holds of the transcript of its origin: what its filter kept, everything that made what it kept, the reply of every rung it kept, which cuts its turns where the origin asked its model, and the open of the origin, which tells the standing."""
  _, log, root = born("x = bash('echo hi')\nclose(1)", "close(None)")
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  command, one = said(log, "bash")[0], said(log, "prompt")[0][1]
  twin = engine.chain("twin", source=root, filter=take(command[1]))
  await settle()
  held, theirs = engine.transcript(twin), engine.transcript(root)
  asked = [a for a in theirs if a[0] == "reply"]
  assert command in held and [a[1] for a in held if a[0] == "prompt"] == [one]
  assert ids(held[: held.index(("started", twin, twin))]) == [one, command[2], asked[0][1], command[1]]
  assert [a[3] for a in said(held, "tell") if a[1] == root] == [opened(root, "root").rsplit("\n", 1), [takes(root)]]
  assert [a for a in held if a[0] == "reply"] == asked[:1] and len(asked) == 2
  assert [a[1] for a in held if a[0] == "done" and a[1] in (asked[0][1], asked[1][1])] == [asked[0][1]]
  got = engine.turns(on=twin)
  assert [role for role, *_ in got] == ["user", "assistant", "user"]
  assert got[0][1].endswith(f"#{command[2]} advance on {one}")


async def test_the_filter_is_given_every_act_what_the_operator_asked_among_them() -> None:
  """The filter is given every act, what the operator asked among them, since what the operator asked of a chain no rung of it says again."""
  _, _, root = born()
  engine.read("a.txt", on=root)
  engine.cd("/deep", on=root)
  await engine.rung("k = 1", on=root)
  watch: list[tuple] = []
  twin = engine.chain("twin", source=root, filter=seen(watch))
  await settle()
  assert all(engine.question(a) for a in watch)
  assert [a[0] for a in watch if a[2] == OPERATOR] == ["stand", "read", "cd", "rung"]
  held = engine.transcript(twin)
  assert [a[4] for a in held if a[0] == "cd"] == ["/deep"]


async def test_what_a_chain_said_of_itself_it_keeps_whatever_the_filter_says() -> None:
  """What a chain said of itself it keeps whatever the filter says, so the open of the origin stands in the new chain."""
  _, _, root = born()
  await engine.rung("k = 1", on=root)
  twin = engine.chain("twin", source=root, filter=take())
  await settle()
  assert heads(engine.turns(on=twin)) == [f"#{root}", f"#{root} standing", f"#{twin}"]


async def test_the_source_of_a_chain_is_a_chain_by_its_name() -> None:
  """The source of a chain is a chain, by its name, and means the transcript of that chain as it stands."""
  _, log, root = born()
  await engine.rung("k = 1", on=root)
  was = list(engine.transcript(root))
  twin = await chained("twin", root)
  await engine.rung("later = 2", on=root)
  held = engine.transcript(twin)
  assert said(log, "chain")[-1][5] == root
  assert lasting(held[: len(was)]) == lasting(was) and "later" not in engine.module(twin)


async def test_a_source_that_names_no_chain_of_the_life_refuses_the_call() -> None:
  """A source that names no chain of the life refuses the call in the caller, and no chain is made."""
  sand, log, root = born()
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
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith(f"#{forked} raised")] == [
    f"#{forked} raised\n{forked}_raised = Refused('no chain {forked}')"
  ]
  assert [a[4] for a in said(log, "chain")] == ["root"]
  assert [e[0][4] for e in sand.record if e[0][0] == "chain"] == ["root"]


async def test_a_chain_with_a_source_reads_what_its_origin_stands_on_as_it_stands() -> None:
  """A chain with a source reads what its origin stands on, as it stands."""
  sand, log, root = born()
  before = [a for a in sand.calls if a[0] == "stand"]
  twin = await chained("twin", root)
  assert [a for a in sand.calls if a[0] == "stand"] == before
  assert [a[1] for a in acts(log).values() if a[0] == "stand"] == ["stand1"]
  assert engine.standing() == STANDS and engine.module(twin)["actor"] == "m/low"
  later = Sand(stands=[[STANDS[0][0]], "/z", "operator"])
  _, over = await relived(later, list(sand.record))
  again = engine.chain("again", source=over)
  await settle()
  assert engine.standing() == later.stands and engine.cwd(on=again) == "/z"
  assert engine.module(again)["actor"] == "operator"


async def test_the_chain_retells_the_rungs_of_its_origin_through_the_program_of_the_origin() -> None:
  """The chain retells the rungs of its origin through the program of the origin, and it makes the rungs it retells itself, so they tell nothing."""
  _, log, root = born("a = 1", "close(a + 1)")
  one = engine.prompt(int, "count", on=root)
  assert await one == 2
  await settle()
  twin = await chained("twin", root)
  theirs = engine.program(root)
  first, second = steps(log, one)
  told = [
    fresh(root, prompted(one, "int", "count"), f"#{first} advance on {one}"),
    f"#{first} told nothing\n\n#{second} advance on {one}",
  ]
  assert list(theirs.values()) == [told[0], "a = 1", told[1], "close(a + 1)"]
  retold = [a for a in said(log, "rung") if a[3] == twin]
  assert [(a[2], a[4], a[5]) for a in retold] == [(twin, word, donor) for donor, word in theirs.items()]
  assert engine.program(twin) == {f"{retold[0][1]}_told": opened(twin, "twin", root), **theirs}
  held = engine.transcript(twin)
  owned = {a[1] for a in retold}
  assert [a for a in held if a[1] in owned and a[0] == "tell"] == []


async def test_a_replay_makes_the_rungs_of_a_chain_again_from_its_donor() -> None:
  """A replay makes the rungs of a chain again from its donor: it keeps each rung of the ladder whose word is the next part of the words it is given, it makes one rung of what is left, and every other rung of the ladder is gone."""
  _, log, root = born("a = 1", "b = 2", "c = 3", "close(None)")
  act = engine.prompt(None, "count", on=root)
  assert await act is None
  await settle()
  first, *gone = steps(log, act)
  was = [a[1] for a in said(log, "rung")]
  engine.write(Text(act, "a = 1\nb = 20\nc = 30"), on=root)
  await settle()
  made = [a for a in said(log, "rung") if a[1] not in was]
  bind = fresh(root, prompted(act, "None", "count"), f"#{first} advance on {act}")
  advances = [
    (f"#{before} told nothing\n\n#{one} advance on {act}", f"{one}_told")
    for before, one in zip([first, *gone], gone, strict=False)
  ]
  assert [(a[4], a[5]) for a in made] == [(bind, f"{first}_told"), ("a = 1", first), *advances, ("b = 20\nc = 30", "")]
  last = made[-1][1]
  kept = engine.program(root)
  held = [f"{first}_told", first, *[one for _, one in advances], f"{last}_told", last]
  told = [bind, "a = 1", *[one for one, _ in advances], engine.unquoted(written(last, "b = 20\nc = 30", act))]
  assert list(kept) == held and list(kept.values()) == [*told, "b = 20\nc = 30"]
  assert (engine.module(root)["b"], engine.module(root)["c"]) == (20, 30)


async def test_the_donor_of_a_replay_is_the_ladder_of_the_origin_or_the_ladder_as_it_stands() -> None:
  """The donor of a replay is the rungs of the origin that are over at the birth of the chain for a chain with a source, and the rungs of the chain as they stand for a write of the door of one of its prompts."""
  _, log, root = born()
  await engine.rung("k = 1", on=root)
  twin = await chained("twin", root, 300)
  theirs = [a for a in said(log, "rung") if a[3] == twin]
  assert [a[5] for a in theirs] == list(engine.program(root))
  act = engine.prompt(int, "more", to=OPERATOR, on=twin)
  engine.write(Text(act, "k = 2"), on=twin)
  await settle()
  engine.write(Text(act, "k = 3"), on=twin)
  await settle()
  mine = [a for a in said(log, "rung") if a[3] == twin]
  (made, again) = [a[1] for a in mine if a[2] == act]
  donors = [theirs[0][5], theirs[1][5], f"{theirs[0][1]}_told", f"{made}_told"]
  assert [a[5] for a in mine[len(theirs) :]] == ["", *donors, ""] and mine[-1][1] == again
  assert engine.module(twin)["k"] == 3 and engine.module(root)["k"] == 1


async def test_a_write_of_a_door_gives_the_words_of_that_ladder_alone() -> None:
  """A write of a door gives the words of that ladder alone, so every rung of the chain that is no rung of that ladder stands as it did."""
  sand, log, root = born()
  laid = engine.rung("mine = 1", on=root)
  await laid
  sand.script[root] = ["a = 1", "close(None)"]
  act = engine.prompt(None, "count", on=root)
  assert await act is None
  await settle()
  assert engine.read(act, on=root) == Text(act, "a = 1\nclose(None)")
  engine.write(Text(act, "a = 2"), on=root)
  await settle()
  program = engine.program(root)
  first, closer, made = steps(log, act)
  assert list(program.values()) == [
    fresh(root, written(laid, "mine = 1")),
    "mine = 1",
    f"{prompted(act, 'None', 'count')}\n\n#{first} advance on {act}",
    f"#{first} told nothing\n\n#{closer} advance on {act}",
    written(made, "a = 2", act),
    "a = 2",
  ]
  assert engine.module(root)["mine"] == 1 and engine.module(root)["a"] == 2


async def test_a_write_of_a_door_cancels_each_rung_that_it_drops() -> None:
  """A write of a door cancels each rung of that ladder that it drops, but the rung that writes, so the plan that it replaces ends wholly: a word that waits ends where it waits, with every act under it, the rung that writes among them when it answers that word, and a word that is done ends each act that it made and that is not done."""
  _, log, root = born(
    "got = await prompt(str, 'next?')\nclose(got)",
    "door = get(get(get(acting())[2])[2])[2]\nwrite(Text(door, 'close(\"new\")'))",
    auto=False,
  )
  act = engine.prompt(str, "plan", on=root)
  assert await act == "new"
  await settle()
  old, new = [a[1] for a in said(log, "rung") if a[2] == act]
  (asked,) = [a[1] for a in said(log, "prompt") if a[2] == old]
  (writer,) = [a[1] for a in said(log, "rung") if a[2] == asked]
  assert [a[2] for a in said(log, "reply")] == [old, writer]
  assert [type(engine.peek(one)).__name__ for one in (old, asked, writer)] == ["CancelledError"] * 3
  assert [a[4] for a in said(log, "rung") if a[1] in (old, new)] == ["", 'close("new")']
  assert said(log, "cancel") == [("cancel", old, root, None)]
  later = engine.prompt(None, "more", to=OPERATOR, on=root)
  engine.write(Text(later, "b = bash('slow')"), on=root)
  await settle()
  (ended,) = [a[1] for a in said(log, "rung") if a[2] == later]
  command = said(log, "bash")[-1][1]
  assert engine.peek(ended, ...) is None and engine.peek(command, ...) is ...
  engine.write(Text(later, "c = 1"), on=root)
  await settle()
  assert isinstance(engine.peek(command), CancelledError)
  assert said(log, "cancel")[-1] == ("cancel", ended, root, None)


async def test_a_write_of_a_door_that_drops_no_rung_that_ran_keeps_the_module() -> None:
  """A write of a door that drops no rung that ran, since the gate refused each rung that it drops, keeps the module and every rung of the chain as they stand, and makes only the rung of what is new."""
  _, log, root = born(
    "k = 1", "ok = BAD", "mine = get(acting())[2]\nwrite(read(mine).replace('BAD', '2'))", "close(ok)"
  )
  act = engine.prompt(int, "fix it", on=root)
  assert await act == 2
  await settle()
  one, _, writer, made, _ = [a[1] for a in said(log, "rung") if a[2] == act]
  assert len(said(engine.transcript(root), "module")) == 1 and engine.module(root)["mine"] == act
  assert [a for a in said(log, "rung") if a[2] == root] == []
  ran = [a[4] for a in said(log, "run")]
  assert {a[6] for a in said(log, "run")} == {""} and [ran.count(one) for one in (one, writer, made)] == [1, 1, 1]
  assert engine.read(act, on=root).content == "k = 1\nok = 2\nclose(ok)"


async def test_a_replay_makes_the_module_of_the_chain_again_as_it_was_at_its_birth() -> None:
  """A replay makes the module of the chain again, as it was at its birth but on the standing the chain stands on then, and makes its rungs in that one, so what a word it drops bound is gone, and a word that runs while it happens ends in the module it began in."""
  sand, _, root = born()
  act = engine.prompt(None, "work", to=OPERATOR, on=root)
  engine.write(Text(act, "a = 1\nb = 2"), on=root)
  await settle()
  engine.write(Text(act, "a = 1"), on=root)
  await settle()
  assert engine.read(act, on=root) == Text(act, "a = 1")
  assert engine.module(root)["a"] == 1 and "b" not in engine.module(root)
  sand.script[root] = ["c = 0", "write(Text(get(acting())[2], 'c = 3'))\nd = 4", "close(None)"]
  later = engine.prompt(None, "edit", on=root)
  assert await later is None
  await settle(300)
  assert engine.read(later, on=root) == Text(later, "c = 3\nclose(None)")
  assert engine.module(root)["c"] == 3 and engine.module(root)["a"] == 1
  assert "d" not in engine.module(root)
  await relived(Sand(stands=[STANDS[0], "/w", "m/high"]), list(sand.record))
  assert engine.module(root)["actor"] == "m/high"
  engine.write(Text(act, "a = 5"), on=root)
  await settle()
  assert engine.module(root)["a"] == 5 and engine.module(root)["actor"] == "m/high"


async def test_before_every_reply_the_chain_tells_the_last_line_of_the_turn() -> None:
  """Before every reply the chain tells the last paragraph of the turn, whose header says what the answer is for, as #rung5 advance on prompt1, which names the one that made the rung."""
  sand, log, root = born("a = 1", "close(a)", "k = 2")
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  bare = engine.rung(on=root)
  assert await bare is None
  await settle()
  first, second = steps(log, one)
  asks = [(a[2], sand.turns[a[1]][-1]) for a in said(log, "reply")]
  assert [(by, last[0], last[1].split("\n")[-1]) for by, last in asks] == [
    (first, "user", f"#{first} advance on {one}"),
    (second, "user", f"#{second} advance on {one}"),
    (bare, "user", f"#{bare} advance on {OPERATOR}"),
  ]
  assert engine.module(root)["k"] == 2


async def test_the_advance_binds_the_share_of_the_window_that_the_last_reply_read() -> None:
  """The advance binds the share of the window that the last reply on the chain read, as rung5_share = 0.25: the tokens that reply read over the window that the roster gives its actor, to two places, and no share before a reply that read a token."""
  _, log, root = born("a = 1", "close(1)", cost=(100000, 0, 0, 0, 0.0))
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  first, second = steps(log, act)
  told = paragraphs(engine.turns(on=root))
  assert f"#{first} advance on {act}" in told and f"#{second} advance on {act}\n{second}_share = 0.25" in told


async def test_the_advance_binds_each_act_that_a_word_of_the_chain_awaits() -> None:
  """The advance binds each act that a word of the chain awaits and that is not over, as rung5_waits = ['bash3'], and no waits when no word awaits, so the model sees which acts the words that it wrote before still wait for."""
  _, log, root = born("x = bash('slow')\nawait x", "close(2)", auto=False)
  one = engine.prompt(int, "count", on=root)
  await settle()
  two = engine.prompt(int, "tally", on=root)
  assert await two == 2
  waited = said(log, "bash")[0][1]
  (first,), (second,) = steps(log, one), steps(log, two)
  told = paragraphs(engine.turns(on=root))
  assert f"#{first} advance on {one}" in told and f"#{second} advance on {two}\n{second}_waits = [{waited!r}]" in told


async def test_the_chain_tells_the_doctrine_of_its_actor_before_each_standing_that_it_takes() -> None:
  """Before the paragraph of each standing that it takes, the chain tells the doctrine of its actor, when the roster entry of that actor holds one, as the quote doctrine, in a paragraph headed doctrine, so the doctrine stands before the words that the extensions play and the message that changes from chain to chain, and each chain that starts the same holds it in the same prefix."""
  taught = [*STANDS[0], ["q", ["low", "high"], 300000, "Look first.\nThen fix."]]
  doctrine = "doctrine\n<s:doctrine>\nLook first.\nThen fix.</s:doctrine>"
  prefixes = []
  for message in ("count", "tally"):
    _, _, root = born("close(1)", stands=[taught, "/w", "q/low"])
    one = engine.prompt(int, message, on=root)
    assert await one == 1
    told = paragraphs(engine.turns(on=root))
    at = told.index(f"#{root} {doctrine}")
    assert told[at + 1].startswith(f"#{root} standing\n")
    assert told.count(f"#{root} {doctrine}") == 1
    prefixes.append(told[: at + 1])
  assert prefixes[0] == prefixes[1]
  later = engine.chain("later", on=root)
  assert paragraphs(engine.turns(on=later))[1:3] == [f"#{later} {doctrine}", paragraphs(engine.turns(on=later))[2]]
  assert paragraphs(engine.turns(on=later))[2].startswith(f"#{later} standing\n")


async def test_the_chain_tells_a_doctrine_once_before_the_advance_when_no_prompt_told_it() -> None:
  """Before the first prompt to another actor whose roster entry holds a doctrine, and before the advance of a reply to such an actor, the chain tells its doctrine when nothing told it, and the chain never tells a doctrine that its transcript tells already, so the model reads it once, and a chain whose actor reads none and that asks no such actor tells nothing of it."""
  taught = [*STANDS[0], ["q", ["low", "high"], 300000, "Look first.\nThen fix."]]
  _, log, root = born("a = 1", "close(2)", "close(3)", stands=[taught, "/w", "m/low"])
  await engine.rung(to="q/low", on=root)
  assert await engine.prompt(int, "again", to="q/high", on=root) == 2
  assert await engine.prompt(int, "plain", on=root) == 3
  first = said(log, "reply")[0][2]
  doctrine = f"#{root} doctrine\n<s:doctrine>\nLook first.\nThen fix.</s:doctrine>"
  told = paragraphs(engine.turns(on=root))
  assert told[told.index(doctrine) + 1] == f"#{first} advance on operator"
  assert told.count(doctrine) == 1 and sum("<s:doctrine>" in one for one in told) == 1
  _, _, root = born("close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  assert not any("<s:doctrine>" in one for one in paragraphs(engine.turns(on=root)))


async def test_the_chain_tells_a_word_that_told_nothing_before_the_advance() -> None:
  """When nothing was told on the chain since the word of the last rung that the chain did not make was ready, the chain tells that rung under the header told nothing before the advance, so the model sees that the word showed it nothing."""
  _, log, root = born("x = read('/w/a.txt', show=HIDDEN)", "x = read('/w/a.txt')", "close(1)")
  one = engine.prompt(int, "count", on=root)
  assert await one == 1
  first, second, third = steps(log, one)
  look = said(log, "read")[1][1]
  assert heads(engine.turns(on=root))[3:8] == [
    f"#{first} advance on {one}",
    f"#{first} told nothing",
    f"#{second} advance on {one}",
    f"#{look}",
    f"#{third} advance on {one}",
  ]


async def test_the_chain_holds_the_text_of_each_paragraph_told_on_it() -> None:
  """The chain holds the text of each paragraph told on it, in order."""
  _, log, root, first, second = await told_read()
  told = [a[5] for a in said(log, "run") if a[4] in (f"{first}_told", f"{second}_told")]
  assert "\n\n".join(told) == engine.unquoted("\n\n".join(paragraphs(engine.turns(on=root))))


async def test_at_the_advance_of_the_next_rung_the_chain_runs_that_text() -> None:
  """At the advance of the next rung, before it asks the reply, or at the first ready of a rung that it does not advance, the chain runs that text as the told rung of that rung, and holds nothing more."""
  _, log, _, first, second = await told_read()
  runs = said(log, "run")
  assert [a[4] for a in runs] == [f"{first}_told", first, f"{second}_told", second]
  assert not runs[2][5].startswith(runs[0][5])


async def test_each_act_that_a_told_paragraph_opened_and_each_value_that_it_told_is_bound() -> None:
  """Each act that a told paragraph opened and each value that it told is bound for the word of that rung, whoever wrote it."""
  _, _, root, _, _ = await told_read()
  assert engine.module(root)["got"] == "one\ntwo"


async def test_the_gate_never_reads_a_told_rung_as_a_word() -> None:
  """The gate never reads a told rung as a word, since the engine wrote it."""
  _, log, _, _, _ = await told_read()
  assert [word for word, _ in gatings(log)] == ["t = read('a.txt')", "got = read1_text"]


async def test_the_turns_do_not_show_a_told_rung() -> None:
  """The turns do not show a told rung, since the turn shows its python already."""
  _, _, root, first, second = await told_read()
  assert paragraphs(engine.turns(on=root)) == [
    opened(root, "root"),
    takes(root),
    written(first, "t = read('a.txt')"),
    "#read1\nread1_path = 'a.txt'\n<s:read1_text>\none\ntwo</s:read1_text>",
    written(second, "got = read1_text"),
  ]


async def test_a_told_rung_is_no_act() -> None:
  """A told rung is no act, and the record keeps neither it nor its run."""
  sand, _, _, first, second = await told_read()
  assert engine.get(f"{first}_told") is None and engine.get(f"{second}_told") is None
  assert [e for e in sand.record if "_told" in str(e[0][:2])] == []


async def test_a_later_life_hears_the_same_paragraphs_and_runs_the_told_rung_again() -> None:
  """A later life hears the same paragraphs and runs the told rung again."""
  sand, _, root, _, _ = await told_read()
  await relived(Sand(), list(sand.record))
  assert engine.module(root)["got"] == "one\ntwo"


async def test_a_told_rung_takes_the_name_of_the_rung_whose_advance_or_first_ready_runs_it() -> None:
  """A told rung takes the name of the rung whose advance or first ready runs it, with _told after it, as rung5_told."""
  _, log, root = born()
  step = engine.rung("k = 1", on=root)
  assert await step is None
  assert [a[4] for a in said(log, "run")] == [f"{step}_told", step]
  assert engine.program(root) == {f"{step}_told": said(log, "run")[0][5], step: "k = 1"}


async def test_a_rung_makes_at_most_one_told_rung_at_its_advance_or_at_its_first_ready() -> None:
  """A rung makes at most one told rung, at its advance or at its first ready, so no two told rungs share a name."""
  _, log, root = born()
  assert await engine.rung("k = 1", on=root) is None
  twin = await chained("twin", root)
  retold = [a[1] for a in said(log, "rung") if a[3] == twin]
  told = [a[4] for a in said(log, "run") if a[3] == twin and a[4] not in retold]
  assert told == [f"{retold[0]}_told"]
  names = [a[4] for a in said(log, "run") if a[4].endswith("_told")]
  assert len(names) == len(set(names))


async def test_the_chain_takes_its_own_act() -> None:
  """The chain takes its own act, since the engine is the one that runs it."""
  sand, log, root = born()
  two = await chained("two")
  assert [(a[1], a[2]) for a in said(log, "started") if a[1] in (root, two)] == [(root, root), (two, two)]
  assert [a for a in said(log, "done") if a[1] in (root, two)] == [] and engine.peek(two) is None
  assert [a for a in sand.calls if a[1] in (root, two)] == []


async def test_the_chain_asks_the_model_of_a_rung_for_its_word_by_a_reply() -> None:
  """The chain asks the model of a rung for its word by a reply it makes under the site of that rung, whose one word is the actor."""
  sand, log, root = born("close(1)")
  one = engine.prompt(int, "count", to="n/low", on=root)
  assert await one == 1
  (step,) = steps(log, one)
  assert [(a[2], a[3], *a[4:]) for a in said(log, "reply")] == [(step, root, "n/low")]
  assert engine.get(step)[6] == "n/low" and [a[1] for a in said(sand.calls, "reply")] == ["reply1"]


async def test_when_an_act_that_a_rung_made_on_another_chain_is_done_the_chain_tells_it() -> None:
  """When an act that a rung of the chain made on another chain is done while the word of that rung does not run and no word of the chain awaited it, the chain tells it about that rung under the header done, and binds what it came to under the word value, since its model reads no other transcript."""
  word = "far = chain('far')\nx = prompt(str, 'one', on=far)\ny = prompt(str, 'two', to=OPERATOR, on=far)\nclose(1)"
  sand, log, root = born(word, "close(await y)")
  sand.script["chain2"] = ["close('a\\nb')"]
  act = engine.prompt(int, "fan out", on=root)
  assert await act == 1
  await settle()
  (step,) = steps(log, act)
  far, (x, y) = "chain2", [next(a[1] for a in said(log, "prompt") if a[5] == message) for message in ("one", "two")]
  engine.close("c", y)
  await settle()
  assert [(a[1], a[2], a[3]) for a in said(log, "tell") if a[3][0].startswith((f"#{x} done", f"#{y} done"))] == [
    (step, root, [f"#{x} done\n<s:{x}_value>\na\nb</s:{x}_value>"])
  ]
  assert f"#{x} done\n<s:{x}_value>\na\nb</s:{x}_value>" in paragraphs(engine.turns(on=root))
  assert [a[3::2] for a in said(log, "prompt") if engine.scope(a[1]) == root] == [(root, "fan out"), (act, f"{x} done")]
  assert engine.peek(y) == "c" and engine.get(far)[0] == "chain"


@dataclass
class Down(Sand):
  """A World whose model of each chain but the root answers nothing: at the next turn of the loop it pauses the chain
  of such a reply and refuses the reply."""

  def hear(self, a: tuple) -> World:
    """The World of the suite, but for a reply on a chain that is not the root."""
    match a:
      case ("reply", about, _, on, _) if on != ROOT:
        yield "started", about
        self.loop.call_soon(self.refuse, on, about)
      case _:
        yield from super().hear(a)

  def refuse(self, on: str, about: str) -> None:
    """Pause the chain of a reply, then refuse the reply."""
    engine.pause(on)
    engine.say("done", about, Refused("m answered nothing: overloaded"))


async def test_the_chain_tells_the_pause_of_a_prompt_that_one_of_its_rungs_made_on_another_chain() -> None:
  """When the chain of a prompt that a rung of the chain made on another chain pauses at a refused reply of that prompt, the chain tells that rung under the header paused and binds the refusal under the word why, and when the word of that rung does not run it prompts nothing, so a model sees that the prompt waits for a wake."""
  sand = Down()
  log, root = life(sand)
  sand.script[root] = ["lane = chain('lane')\nwork = prompt(int, 'work', on=lane)\nclose(0)", "close(None)"]
  act = engine.prompt(int, "lead", on=root)
  assert await act == 0
  await settle(300)
  (step,) = steps(log, act)
  (work,) = [a[1] for a in said(log, "prompt") if a[5] == "work"]
  told = f"#{work} paused\n{work}_why = Refused('m answered nothing: overloaded')"
  assert [(a[1], a[2], a[3]) for a in said(log, "tell") if a[3][0].startswith(f"#{work} paused")] == [
    (step, root, [told])
  ]
  assert told in paragraphs(engine.turns(on=root))
  assert [a[3::2] for a in said(log, "prompt") if engine.scope(a[1]) == root] == [
    (root, "lead"),
    (act, f"{work} paused"),
  ]


async def test_the_chain_advances_no_rung_that_is_over() -> None:
  """The chain advances no rung that is over, so it asks no reply for a rung that a close or a cancel ended before its advance."""
  _, log, root = born()
  first = engine.prompt(int, "first", on=root)
  second = engine.prompt(int, "second", on=root)
  await settle()
  ((_, reply, rung, *_),) = said(log, "reply")
  world_says("done", reply, ("assistant", f"close(0, {second!r})\nclose(1)", (0, 0, 0, 0, 0.0), []))
  assert await first == 1
  await settle()
  assert engine.peek(second) == 0
  assert [a[2] for a in said(log, "reply")] == [rung]


async def test_a_statement_that_the_model_of_a_rung_writes_runs_after_the_told_rung_of_that_rung() -> None:
  """A statement that the model of a rung writes runs after the told rung of that rung, so it reads each value that the advance of that rung bound."""
  _, log, root = born()
  engine.prompt(int, "count", on=root)
  await settle()
  rung = said(log, "rung")[0][1]
  world_says("writing", rung, f"k = {root}_label\n", 0)
  await settle()
  assert engine.module(root)["k"] == "root"
