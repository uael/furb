"""rung, the run of one word on a chain."""

from asyncio import CancelledError

import pytest

from conftest import STANDS, Py, Sand, gated, kept, life, of, paragraphs, ran, said, settle, sown, watched
from furb import engine
from furb.engine import WORLD, Exit, Refused, Text


async def test_the_run_of_a_word_on_a_chain() -> None:
  """The run of a word on a chain: a word its caller wrote, which it tells, since nothing else did; or, with no word, a turn of a model, which its chain asks for at the turn it gives it and which the World answers, of which it tells nothing, since that turn stands as the turn it is."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  laid = engine.rung("k = 1", on=root)
  await laid
  sand.script[root] = ["close(k + 1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [(a[1], a[2], a[3]) for a in said(log, "tell") if a[1] in (laid, step)] == [
    (laid, laid, [f"#{laid}", "k = 1"]),
    (laid, laid, [f"#{laid} closed"]),
    (step, root, [f"#{step} advance on {act}"]),
  ]
  assert [(a[1], a[2]) for a in said(log, "ask")] == [(step, root)]
  assert [(a[1], a[2]) for a in said(log, "answer")] == [(step, WORLD)]
  assert engine.turns(on=root)[1] == ("assistant", "close(k + 1)", (0, 0, 0, 0, 0.0), ["signed 12"])


async def test_a_rung_is_an_act_the_run_of_one_word_in_the_globals_of_its_chain() -> None:
  """A rung is an act: the run of one word in the globals of its chain, which the chain has the Kernel run."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("k = 21", on=root)
  assert await act is None
  made = said(log, "rung")[0]
  assert made[1] == act and engine.acts[act] == made
  assert (made[3], made[4]) == (root, "k = 21")
  assert [a[1] for a in said(log, "run")] == [act]
  assert ran(log) == ["k = 21"] and engine.modules[root]["k"] == 21


async def test_the_engine_tells_what_a_step_raised() -> None:
  """The engine tells what a step raised."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["raise ValueError('boom')", "close(1)"]
  act = engine.prompt(int, "try", on=root)
  assert await act == 1
  first, second = [a[1] for a in said(log, "rung") if a[2] == act]
  assert engine.turns(on=root)[2][1] == f"#{first} raised ValueError('boom')\n\n#{second} advance on {act}"


async def test_a_step_that_raised_nothing_and_debugged_nothing_tells_nothing() -> None:
  """A step that raised nothing and debugged nothing tells nothing."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a for a in said(log, "tell") if a[2] == step] == []
  assert of(engine.turns(on=root), step) == [f"#{step} advance on {act}"]


async def test_the_open_of_a_rung_with_a_word_is_its_header_and_then_that_word() -> None:
  """The open of a rung with a word is its header and then that word, as its caller wrote it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.rung("<S1>hi</S1>\nk = S1", on=root)
  await act
  assert said(log, "tell")[2] == ("tell", act, act, [f"#{act}", "<S1>hi</S1>\nk = S1"])
  assert of(engine.turns(on=root), act) == [f"#{act}\n<S1>hi</S1>\nk = S1", f"#{act} closed"]
  assert engine.modules[root]["k"] == "hi"


async def test_a_rung_with_no_word_tells_nothing_where_it_is_made() -> None:
  """A rung with no word tells nothing where it is made, since the chain tells it as the last line of the turn it asks for it with."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.prompt(int, "count", "m/high", on=root)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [(a[2], a[3]) for a in said(log, "tell") if a[1] == step] == [(root, [f"#{step} advance on {act}"])]
  (asked,) = said(log, "ask")
  assert asked[1] == step and asked[5][-1][1].split("\n")[-1] == f"#{step} advance on {act}"


async def test_the_raised_header_tells_the_exception_as_python_shows_it() -> None:
  """The raised header tells the exception as python shows it, which says its type and its message."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  (act,) = [a[1] for a in said(log, "rung")]
  assert of(engine.turns(on=root), act) == [f"#{act}\nraise ValueError('boom')", f"#{act} raised ValueError('boom')"]


async def test_a_rung_that_retells_another_rung_names_its_acts_under_that_one() -> None:
  """A rung that retells another rung names its acts under that one, so it makes the same acts and shares them."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose(1)"]
  act = engine.prompt(int, "run it", on=root)
  assert await act == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  twin = engine.chain("twin", source=root)
  await settle(300)
  retold = [a for a in said(log, "rung") if a[3] == twin]
  program = engine.ask("program", root)[1]
  assert isinstance(program, dict)
  assert [a[5] for a in retold] == list(program) and step in program
  assert said(log, "bash") == [("bash", "bash1", step, root, "echo hi", False, 600.0)]
  assert engine.modules[twin]["x"] == engine.modules[root]["x"] == "bash1"


async def test_a_cancel_of_a_rung_is_the_kernels_to_do() -> None:
  """A cancel of a rung is the Kernel's to do, since the Kernel is the one running the word."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose((await x).code)"]
  act = engine.prompt(int, "go", on=root)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  engine.cancel(step)
  await settle()
  assert [(a[2], type(a[3]).__name__) for a in said(log, "ran") if a[1] == step] == [(step, "CancelledError")]
  assert isinstance(engine.peek(step, on=root), CancelledError)
  engine.cancel(act)


async def test_the_word_of_a_model_is_python_code_and_nothing_else() -> None:
  """The word of a model is python code and nothing else."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1 + 1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[4] for a in said(log, "run") if a[1] == step] == ["close(1 + 1)"]
  assert [turn[1] for turn in engine.turns(on=root) if turn[0] == "assistant"] == ["close(1 + 1)"]


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
  log, root = life(sand)
  two = engine.chain("two")
  sand.script[root] = [f"await rung('helper = 2', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "delegate", on=root) == 1
  await settle()
  (helper,) = [a[1] for a in said(log, "rung") if a[4] == "helper = 2"]
  assert paragraphs(engine.turns(on=two)) == [
    f"#{two} two\n{two}: Act[object] = Act({two!r})",
    f"#{two} stands {STANDS!r}",
    f"#{helper}\nhelper = 2",
    f"#{helper} closed",
  ]
  assert of(engine.turns(on=root), helper) == []


async def test_a_word_its_caller_wrote_stands_in_a_user_turn_as_python_under_the_header_of_its_rung() -> None:
  """A word its caller wrote stands in a user turn as python, under the header of its rung."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.rung("k = 1", on=root)
  await act
  role, py, usage, blocks = engine.turns(on=root)[-1]
  assert (role, usage, blocks) == ("user", None, None)
  assert py.split("\n\n")[2:] == [f"#{act}\nk = 1", f"#{act} closed"]
  compile(py, "<turn>", "exec")


async def test_rung_is_given_a_word_and_runs_it_on_a_chain_in_the_globals_of_that_chain() -> None:
  """rung is given a word and runs it on a chain in the globals of that chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  await engine.rung("k = 1", on=two)
  assert engine.modules[two]["k"] == 1 and "k" not in engine.modules[root]
  assert [(a[4], a[3]) for a in said(log, "rung")] == [("k = 1", two)]


async def test_the_gate_reads_the_word_its_caller_wrote_like_any_word() -> None:
  """The gate reads the word its caller wrote like any word."""
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
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
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
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (binding,) = [a[1] for a in said(log, "rung") if a[2] == root]
  wrote = f"{root}: Act[object] = Act({root!r})\n{act}: Act[int] = Act({act!r})"
  assert [a[1:] for a in said(log, "ready")] == [
    (laid, laid, "k = 1"),
    (binding, binding, wrote),
    (step, step, "close(k + 1)"),
  ]
  assert [q[4] for q in engine.asked.values() if q[0] == "gate"] == ["k = 1", "close(k + 1)"]
  assert [a[1] for a in said(log, "run")] == [laid, binding, step]


async def test_the_chain_has_the_kernel_begin_it_in_the_module_of_that_chain() -> None:
  """The chain has the Kernel begin it in the module of that chain, and the run carries it forward at the done of every act the word waits for, so that the engine owns the order of it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nout = await x\nclose(out.code)"]
  act = engine.prompt(int, "run it", on=root)
  assert await act == 0
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  moves = [a for a in log if (a[1] == step and a[0] in ("run", "wants", "sent", "ran")) or a[:2] == ("done", "bash1")]
  assert [(a[0], a[3]) for a in moves[:2]] == [("run", root), ("wants", "bash1")]
  assert [(a[0], type(a[3]).__name__) for a in moves[2:]] == [
    ("done", "Exit"),
    ("sent", "Exit"),
    ("ran", "CancelledError"),
  ]
  assert moves[2][3] == moves[3][3]
  out = engine.modules[root]["out"]
  assert isinstance(out, Exit) and out.code == 0


async def test_it_is_done_with_what_the_word_gave() -> None:
  """It is done with what the word gave, nothing for a word that ran to its end and the exception for a raise, which it tells with its type and its message, which its close then holds none of."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  ended = engine.rung("k = 1", on=root)
  await ended
  answered = engine.rung("close(21)", on=root)
  await answered
  hurt = engine.rung("raise ValueError('boom')", on=root)
  with pytest.raises(ValueError, match="boom"):
    await hurt
  assert [type(a[3]).__name__ for a in said(log, "ran")] == ["NoneType", "CancelledError", "ValueError"]
  assert engine.outcomes[ended] is None and engine.outcomes[answered] == 21
  assert isinstance(engine.outcomes[hurt], ValueError)
  assert of(engine.turns(on=root), hurt) == [f"#{hurt}\nraise ValueError('boom')", f"#{hurt} raised ValueError('boom')"]


async def test_of_the_queries_its_word_asked_it_tells_nothing() -> None:
  """Of the queries its word asked it tells nothing, since they tell themselves."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nclose(len(t.lines))"]
  act = engine.prompt(int, "read it", on=root)
  assert await act == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  told = engine.turns(on=root)
  assert told[2][1] == f"#read a.txt\n# /w/a.txt, 0 known\n# 1 one\n\n#{act} closed 1"
  assert of(told, step) == [f"#{step} advance on {act}"]


async def test_an_answer_with_no_text_is_a_word_like_any_other() -> None:
  """An answer with no text is a word like any other, so the gate reads it, the run gives no value, and the model is asked again."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["", "close(1)"]
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  first, second = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [q[4] for q in engine.asked.values() if q[0] == "gate"] == ["", "close(1)"]
  assert [(a[1], a[4]) for a in said(log, "run") if a[1] in (first, second)] == [(first, ""), (second, "close(1)")]
  assert engine.outcomes[first] is None and [a[1] for a in said(log, "ask")] == [first, second]


async def test_a_replay_makes_a_rung_of_its_own_retelling_each_rung_of_the_donor_it_keeps() -> None:
  """A replay makes a rung of its own retelling each rung of the donor it keeps."""
  sand = Sand(files={"/w/a.txt": "one\n"}, stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["k = 1\nclose(1)", "close(None)"]
  assert await engine.prompt(int, "work", on=root) == 1
  await settle()
  side = engine.chain("side", source=root)
  await settle(300)
  mine = [a for a in said(log, "rung") if a[3] == side]
  assert {a[5]: a[4] for a in mine} == engine.ask("program", root)[1]
  assert mine and all(a[1] != a[5] for a in mine)


async def test_a_rung_that_retells_names_the_rung_the_record_holds() -> None:
  """A rung that retells names the rung the record holds and never another rung that retells it, so a second replay makes the same acts and asks the World nothing twice."""
  sand = sown()
  log, root = life(sand)
  act = engine.prompt(int, "read it", to="operator", on=root)
  engine.write(Text(act, "t = read('a.txt')"), on=root)
  await settle()
  engine.write(Text(act, "t = read('a.txt')\nn = len(t.lines)"), on=root)
  await settle()
  engine.write(Text(act, "t = read('a.txt')\nn = len(t.lines)\nm = n + 1"), on=root)
  await settle()
  laid = said(log, "rung")
  assert [a[5] for a in laid] == ["", laid[0][1], "", laid[0][1], laid[2][1], ""]
  assert [a[0] for a in sand.calls].count("read") == 1
  assert engine.modules[root]["m"] == 3


async def test_a_rung_that_retells_is_done_with_nothing() -> None:
  """A rung that retells is done with nothing when its word answers, runs to its end or is cancelled, whatever the Kernel makes of the word, and with what that word raised."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["k = 1\nclose(21)", "close(None)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 21
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (binding,) = [a[1] for a in said(log, "rung") if a[2] == root]
  hurt = engine.rung("raise ValueError('boom')", on=root)
  with pytest.raises(ValueError, match="boom"):
    await hurt
  stopped = engine.rung("raise CancelledError()", on=root)
  with pytest.raises(CancelledError):
    await stopped
  twin = engine.chain("twin", source=root)
  await settle(300)
  theirs = {a[5]: type(engine.outcomes[a[1]]).__name__ for a in said(log, "rung") if a[3] == twin}
  assert theirs == {binding: "NoneType", step: "NoneType", hurt: "ValueError", stopped: "NoneType"}
  other = sown()
  mine: list[tuple] = []
  over = engine.boot(kernel=kept(Py().kernel()), probe=watched(mine), world=other.hears())
  other.script[over] = ["k = 1\nclose(21)", "close(None)"]
  assert await engine.prompt(int, "count", on=over) == 21
  await settle()
  side = engine.chain("side", source=over)
  await settle(300)
  assert [engine.outcomes[a[1]] for a in said(mine, "rung") if a[3] == side] == [None, None]


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


async def test_a_rung_that_retells_says_each_question_it_makes_as_the_rung_it_retells() -> None:
  """A rung that retells says each question it makes as the rung it retells, so the question it makes at a place is the one that rung made there."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nx = bash('echo hi')\nclose(1)", "close(None)"]
  act = engine.prompt(int, "run it", on=root)
  assert await act == 1
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert said(log, "read") == [("read", f"read@{step}.1", step, root, "a.txt")]
  twin = engine.chain("twin", source=root)
  await settle(300)
  (retold,) = [a[1] for a in said(log, "rung") if a[3] == twin and a[5] == step]
  assert engine.asked[f"read@{step}.1"] == ("read", f"read@{step}.1", step, twin, "a.txt")
  assert said(log, "bash") == [("bash", "bash1", step, root, "echo hi", False, 600.0)]
  assert retold != step and engine.modules[twin]["x"] == "bash1"


async def test_what_a_rung_that_retells_asks_is_named_under_the_one_it_retells() -> None:
  """What a rung that retells asks is named under the one it retells, so the life answers it with what it answered then and the World is asked nothing twice."""
  sand = sown()
  log, root = life(sand)
  sand.script[root] = ["t = read('a.txt')\nclose(len(t.lines))", "close(None)"]
  act = engine.prompt(int, "read it", on=root)
  assert await act == 2
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  twin = engine.chain("twin", source=root)
  await settle(300)
  asked = [one[1] for one in engine.asked.values() if one[0] == "read"]
  assert asked == [f"read@{step}.1"] and engine.under(asked[0], step)
  assert [a[0] for a in sand.calls].count("read") == 1
  assert engine.modules[twin]["t"] == engine.modules[root]["t"]
