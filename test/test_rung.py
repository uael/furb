"""rung, the run of one word on a chain."""

from asyncio import CancelledError

import pytest

from conftest import STANDS, Sand, attr, gated, life, ran, said, settle, tags, text_of
from furb import engine
from furb.engine import WORLD, Exit, Refused


async def test_the_run_of_a_word_on_a_chain() -> None:
  """The run of a word on a chain: a word its caller wrote, which it tells, since nothing else did; or, with no word, a turn of a model, which its chain asks for at the turn it gives it and which the World answers, of which it tells nothing, since that turn stands as the turn it is."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("k = 1", on=root)
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  told = [tag[2] for tag in tags(engine.turns(on=root), "opened") if tag[2]]
  assert told == ["k = 1"]
  assert [text_of(turn) for turn in engine.turns(on=root) if turn[0] == "assistant"] == ["close(k + 1)"]


async def test_a_rung_is_an_act_the_run_of_one_word_in_the_globals_of_its_chain() -> None:
  """A rung is an act: the run of one word in the globals of its chain, which the chain has the Kernel run."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("k = 21", on=root)
  assert await act is None
  made = said(log, "rung")[0]
  assert made[1] == act and engine.acts[act] is made
  assert (made[3], made[4]) == (root, "k = 21")
  assert [a[1] for a in said(log, "run")] == [act]
  assert ran(log) == ["k = 21"] and engine.modules[root]["k"] == 21


async def test_the_engine_tells_what_a_step_raised() -> None:
  """The engine tells what a step raised."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["raise ValueError('boom')", "close(1)"]
  assert await engine.prompt(int, "try", on=root) == 1
  assert [attr(tag, "message") for tag in tags(engine.turns(on=root), "raised")] == ["boom"]


async def test_a_step_that_raised_nothing_and_debugged_nothing_tells_nothing() -> None:
  """A step that raised nothing and debugged nothing tells nothing."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  told = engine.turns(on=root)
  assert tags(told, "raised") == [] and tags(told, "debugged") == []


async def test_the_opened_tag_of_a_rung_with_a_word_carries_that_word_as_its_body() -> None:
  """The opened tag of a rung with a word carries that word as its body."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.rung("k = 1", on=root)
  await act
  assert ("opened", [("id", act)], "k = 1") in tags(engine.turns(on=root), "opened")


async def test_the_opened_tag_of_a_rung_with_no_word_tells_the_actor_and_the_close_its_word_must_say() -> None:
  """The opened tag of a rung with no word tells the actor that is asked and the close its word must say to answer the prompt."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  engine.prompt(int, "count", "m/high", on=root)
  await settle()
  mine = said(log, "rung")[0]
  assert (mine[6], mine[7]) == ("m/high", "int")
  opened = tags(engine.turns(on=root), "opened")[-1]
  assert opened[1] == [("id", mine[1]), ("actor", "m/high"), ("says", "close(int)")]


async def test_the_raised_tag_tells_the_type_and_the_message_of_the_exception_as_attributes() -> None:
  """The raised tag tells the type and the message of the exception as attributes."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  raised = tags(engine.turns(on=root), "raised")
  assert [tag[1][1:] for tag in raised] == [[("type", "ValueError"), ("message", "boom")]]


async def test_a_rung_that_retells_another_rung_names_its_acts_under_that_one() -> None:
  """A rung that retells another rung names its acts under that one, so it makes the same acts and shares them."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  _, command, *_ = said(log, "bash")[0]
  twin = engine.chain("twin", source=root)
  await settle(300)
  retold = [a for a in said(log, "rung") if a[5]]
  assert [a[5] for a in retold] == [said(log, "rung")[0][1]]
  assert [a[1] for a in said(log, "bash")] == [command]
  assert engine.modules[twin]["x"] == command


async def test_a_cancel_of_a_rung_is_the_kernels_to_do() -> None:
  """A cancel of a rung is the Kernel's to do, since the Kernel is the one running the word."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  _, step, *_ = said(log, "rung")[0]
  engine.cancel(step)
  await settle()
  assert [type(a[3]).__name__ for a in said(log, "ran")] == ["CancelledError"]
  assert isinstance(engine.peek(step, on=root), CancelledError)
  engine.cancel(act)


async def test_the_word_of_a_model_is_python_code_and_nothing_else() -> None:
  """The word of a model is python code and nothing else."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1 + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  assert [a[4] for a in said(log, "run")] == ["close(1 + 1)"]
  assert [turn[1] for turn in engine.turns(on=root) if turn[0] == "assistant"] == [["close(1 + 1)"]]


async def test_a_step_that_raised_keeps_what_it_bound_before_the_raise() -> None:
  """A step that raised keeps what it bound before the raise."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["a = 1\nraise ValueError('boom')", "close(a + 1)"]
  assert await engine.prompt(int, "try", on=root) == 2
  assert engine.modules[root]["a"] == 1


async def test_a_rung_may_await_at_its_top_level() -> None:
  """A rung may await at its top level."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("out = await bash('echo hi')\nclose(out.code)", on=root) == 0


async def test_a_rung_answers_the_prompt_of_its_chain_with_close() -> None:
  """A rung answers the prompt of its chain with close, wherever in its word the close is said."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("close(21)", on=root) == 21


async def test_the_kernel_gives_nothing_for_a_word_that_ran_to_its_end() -> None:
  """The Kernel gives nothing for a word that ran to its end, since a word that answers says a close and stops there."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("close(21)", on=root)
  assert [isinstance(a[3], CancelledError) for a in said(log, "ran")] == [True]


async def test_the_kernel_gives_what_a_word_raised() -> None:
  """The Kernel gives what a word raised, and a top-level return is no python, which the gate refuses as it refuses any word that is not python."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  assert [a[3] for a in said(log, "ran")] == [None]
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  assert isinstance(said(log, "ran")[1][3], ValueError)
  with pytest.raises(SyntaxError, match="'return' outside function"):
    compile("return 1", "<rung>", "exec")
  with pytest.raises(Refused):
    await engine.rung("k = (", on=root)
  assert gated(log)[-1] == "k = (" and ran(log) == ["k = 1", "raise ValueError('boom')"]


async def test_the_word_of_a_rung_runs_to_its_next_await_and_continues_when_the_close_it_awaits_comes() -> None:
  """The word of a rung runs to its next await and continues when the close it awaits comes."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("p = prompt(int, 'how many?', to='operator')\nk = 1\nclose(await p)", on=root)
  await settle()
  _, theirs, *_ = said(log, "prompt")[0]
  assert engine.modules[root]["k"] == 1 and act not in engine.outcomes
  engine.close(21, theirs)
  await settle()
  assert (await act) == 21


async def test_the_word_of_a_rung_answers_its_prompt_with_close() -> None:
  """The word of a rung answers its prompt with close, which carries the value the prompt is done with."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["close(21)"]
  assert await engine.prompt(int, "how many?", on=root) == 21


async def test_the_turns_of_the_chain_of_another_prompt_tell_the_rung_of_a_word_its_caller_wrote() -> None:
  """The turns of the chain of another prompt tell the rung of a word its caller wrote."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  sand.script[root] = [f"rung('helper = 2', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "delegate", on=root) == 1
  await settle()
  assert [tag[2] for tag in tags(engine.turns(on=two), "opened") if tag[2]] == ["helper = 2"]
  assert [tag[2] for tag in tags(engine.turns(on=root), "opened") if tag[2]] == []


async def test_a_word_its_caller_wrote_is_a_user_turn_the_opened_tag_of_its_rung() -> None:
  """A word its caller wrote is a user turn, the opened tag of its rung."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.rung("k = 1", on=root)
  await act
  turn = engine.turns(on=root)[-1]
  assert turn[0] == "user" and ("opened", [("id", act)], "k = 1") in turn[1]


async def test_rung_is_given_a_word_and_runs_it_on_a_chain_in_the_globals_of_that_chain() -> None:
  """rung is given a word and runs it on a chain in the globals of that chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await engine.rung("k = 1", on=two)
  assert engine.modules[two]["k"] == 1 and "k" not in engine.modules[root]
  assert [(a[4], a[3]) for a in said(log, "rung")] == [("k = 1", two)]


async def test_the_kernel_gates_the_word_its_caller_wrote_like_any_word() -> None:
  """The Kernel gates the word its caller wrote like any word."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  assert gated(log) == ["k = 1"]
  with pytest.raises(Refused):
    await engine.rung("k = BAD", on=root)
  assert gated(log) == ["k = 1", "k = BAD"] and ran(log) == ["k = 1"]


async def test_a_rung_with_a_word_completes_with_what_that_word_raises() -> None:
  """A rung with a word completes with what that word raises, and with nothing when the word runs to its end."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  assert await engine.rung("k = 1", on=root) is None
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)


async def test_a_close_ends_the_rung_that_runs_in_a_prompt_at_its_next_await() -> None:
  """A close ends the rung that runs in a prompt at its next await."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  _, step, *_ = said(log, "rung")[0]
  engine.close(21, act)
  await settle()
  assert (await act) == 21 and isinstance(engine.peek(step, on=root), CancelledError)


async def test_a_chain_with_a_source_and_its_origin_share_the_one_act() -> None:
  """A chain with a source and its origin share the one act, the record's."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  _, command, *_ = said(log, "bash")[0]
  twin = engine.chain("twin", source=root)
  await settle(300)
  assert [a[1] for a in said(log, "bash")] == [command]
  assert [a[1] for a in said(sand.calls, "start")] == [command]
  assert engine.modules[twin]["x"] == engine.modules[root]["x"] == command


async def test_a_chain_with_a_source_awaits_what_its_origin_started() -> None:
  """A chain with a source awaits what its origin started, through the name of the act."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(1)"]
  assert await engine.prompt(int, "start one", on=root) == 1
  _, command, *_ = said(log, "bash")[0]
  twin = engine.chain("twin", source=root)
  await settle(300)
  sand.script[twin] = ["close((await x).code)"]
  act = engine.prompt(int, "await it", on=twin)
  await settle()
  assert act not in engine.outcomes
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert (await act) == 0 and [a[1] for a in said(log, "bash")] == [command]


async def test_it_says_its_word_may_run_as_soon_as_it_holds_one() -> None:
  """It says its word may run as soon as it holds one, whichever way that word came, and what the chain makes of the word is the chain's."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  laid = engine.rung("k = 1", on=root)
  await laid
  sand.script[root] = ["close(k + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  _, asking, *_ = said(log, "rung")[-1]
  assert [(a[2], a[3]) for a in said(log, "ready")] == [(laid, "k = 1"), (asking, "close(k + 1)")]
  assert [a[1] for a in said(log, "run")] == [laid, asking]


async def test_the_chain_has_the_kernel_begin_it_in_the_module_of_that_chain() -> None:
  """The chain has the Kernel begin it in the module of that chain, and the run carries it forward at the done of every act the word waits for, so that the engine owns the order of it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nout = await x\nclose(out.code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  made = said(log, "bash")[0]
  assert [(a[1], a[3]) for a in said(log, "run")] == [(made[2], root)]
  assert [(a[1], type(a[3]).__name__) for a in said(log, "sent")] == [(made[2], "Exit")]
  out = engine.modules[root]["out"]
  assert isinstance(out, Exit) and out.code == 0


async def test_it_is_done_with_what_the_word_gave() -> None:
  """It is done with what the word gave, nothing for a word that ran to its end and the exception for a raise, which it tells with its type and its message, which its close then holds none of."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  await engine.rung("close(21)", on=root)
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  assert [a[3] is None for a in said(log, "ran")][:2] == [True, False]
  assert isinstance(said(log, "ran")[2][3], ValueError)
  hurt = tags(engine.turns(on=root), "raised")[0]
  shut = [tag for tag in tags(engine.turns(on=root), "closed") if ("id", attr(hurt, "id")) in tag[1]]
  assert [tag[2] for tag in shut] == [None]


async def test_of_the_queries_its_word_asked_it_tells_nothing() -> None:
  """Of the queries its word asked it tells nothing, since they tell themselves."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nclose(len(t.lines))"]
  assert await engine.prompt(int, "read it", on=root) == 1
  _, step, *_ = said(log, "rung")[0]
  told = engine.turns(on=root)
  assert [tag[1] for tag in tags(told, "read")] == [[("path", "a.txt")]]
  assert [tag[0] for tag in tags(told) if ("id", step) in tag[1]] == ["opened"]


async def test_an_answer_with_no_text_is_a_word_like_any_other() -> None:
  """An answer with no text is a word like any other, so the gate reads it, the run gives no value, and the model is asked again."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["", "close(1)"]
  assert await engine.prompt(int, "work", on=root) == 1
  assert gated(log) == ["", "close(1)"] and ran(log) == ["", "close(1)"]


async def test_the_chain_with_a_source_makes_a_rung_of_its_own_retelling_each_rung() -> None:
  """The chain with a source makes a rung of its own retelling each rung of the ladder it is given."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["k = 1\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "work", on=root) == 1
  await settle()
  side = engine.chain("side", source=root)
  await settle(300)
  mine = [a for a in said(log, "rung") if a[3] == side]
  assert [(a[5], a[4]) for a in mine] == engine.ask("program", root, root)[1]
  assert mine and all(a[1] != a[5] for a in mine)


async def test_a_rung_that_awaits_an_act_nobody_settles_waits_until_the_operator_cancels_it() -> None:
  """A rung that awaits an act nobody settles waits until the operator cancels it, and holds nothing else of the chain."""
  sand = Sand(stands=STANDS, auto=False)
  _, root = life(sand)
  stuck = engine.rung("x = bash('never')\nclose(await x)", on=root)
  other = engine.rung("k = 2\nclose(k)", on=root)
  await settle()
  assert stuck not in engine.outcomes and await other == 2 and engine.modules[root]["k"] == 2
  engine.cancel(stuck)
  await settle()
  assert isinstance(engine.outcomes[stuck], CancelledError)


async def test_the_lineage_a_rung_names_its_acts_under() -> None:
  """The lineage a rung names its acts under is the lineage of its own name, and of the name of the one it retells for a rung that retells."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  assert await engine.prompt(int, "run it", on=root) == 1
  await settle()
  step, command = said(log, "rung")[0][1], said(log, "bash")[0][1]
  assert engine.lineage(command) == engine.lineage(step) + ".1"
  twin = engine.chain("twin", source=root)
  await settle(300)
  retold = next(a for a in said(log, "rung") if a[3] == twin)
  assert retold[5] == step and engine.lineage(retold[1]) != engine.lineage(step)
  assert [a[1] for a in said(log, "bash")] == [command]
