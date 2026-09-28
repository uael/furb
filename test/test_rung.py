"""rung, the run of one word on a chain."""

from asyncio import CancelledError

import pytest

from conftest import (
  WORLD,
  Py,
  acts,
  born,
  chained,
  counted,
  dones,
  fresh,
  gated,
  heads,
  kept,
  of,
  opened,
  paragraphs,
  prompted,
  ran,
  said,
  settle,
  sown,
  stalled,
  takes,
  watched,
  world_says,
  written,
)
from furb import engine
from furb.engine import Exit, Refused, Text


async def test_the_run_of_a_word_on_a_chain() -> None:
  """The run of a word on a chain: a word its caller wrote, which it tells, since nothing else did; or, with no word, a turn of a model, which its chain asks for at the turn it gives it and which the World answers, of which it tells nothing, since that turn stands as the turn it is."""
  log, root, laid, act, step = await counted()
  assert [(a[1], a[2], a[3]) for a in said(log, "tell") if a[1] in (laid, step)] == [
    (laid, laid, [written(laid, "k = 1")]),
    (step, root, [f"#{step} advance on {act}"]),
  ]
  assert [(a[2], a[3]) for a in said(log, "reply")] == [(step, root)]
  assert [(a[1], a[2]) for a in said(log, "done") if a[1].startswith("reply")] == [("reply1", WORLD)]
  assert engine.turns(on=root)[1] == ("assistant", "close(k + 1)", (0, 0, 0, 0, 0.0), ["signed 12"])


async def test_a_rung_is_an_act_the_run_of_one_word_in_the_globals_of_its_chain() -> None:
  """A rung is an act: the run of one word in the globals of its chain, which the chain has the Kernel run."""
  _, log, root = born()
  act = engine.rung("k = 21", on=root)
  assert await act is None
  made = said(log, "rung")[0]
  assert made[1] == act and engine.get(act) == made
  assert (made[3], made[4]) == (root, "k = 21")
  assert [a[4] for a in said(log, "run")] == [f"{act}_told", act]
  assert ran(log) == [fresh(root, written(act, "k = 21")), "k = 21"] and engine.module(root)["k"] == 21


async def test_the_engine_tells_what_a_step_raised() -> None:
  """The engine tells what a step raised."""
  _, log, root = born("raise ValueError('boom')", "close(1)")
  act = engine.prompt(int, "try", on=root)
  assert await act == 1
  first, second = [a[1] for a in said(log, "rung") if a[2] == act]
  raised = f"#{first} raised\n{first}_raised = ValueError('boom')"
  assert engine.turns(on=root)[2][1] == f"{raised}\n\n#{second} advance on {act}"


async def test_a_step_that_raised_nothing_and_debugged_nothing_tells_nothing() -> None:
  """A step that raised nothing and debugged nothing tells nothing."""
  _, log, root = born("k = 1", "close(1)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  steps = [a[1] for a in said(log, "rung") if a[2] == act]
  assert len(steps) == 2 and [a for a in said(log, "tell") if a[2] in steps] == []
  assert [of(engine.turns(on=root), step) for step in steps] == [[f"#{step} advance on {act}"] for step in steps]


async def test_the_open_of_a_rung_with_a_word_tells_that_word_as_a_command_tells_its_command() -> None:
  """The open of a rung with a word tells that word as a command tells its command, bound as rungN_word, so no word that ran runs again."""
  sand, log, root = born()
  one = engine.rung("n = 1", on=root)
  await one
  word = "<s:hi>hi</s:hi>\nn = n + 1"
  act = engine.rung(word, on=root)
  await act
  assert [a[3] for a in said(log, "tell") if a[1] in (one, act)] == [
    [written(one, "n = 1")],
    [f"#{act}\n<s:{act}_word>\n{word}</s:{act}_word>"],
  ]
  sand.script[root] = ["close(n)"]
  asking = engine.prompt(int, "count", on=root)
  assert await asking == 2
  (step,) = [a[1] for a in said(log, "rung") if a[2] == asking]
  told = [
    fresh(root, written(one, "n = 1")),
    written(act, word),
    f"{prompted(asking, 'int', 'count')}\n\n#{step} advance on {asking}",
  ]
  assert [a[5] for a in said(log, "run") if a[4].endswith("_told")] == [engine.unquoted(one) for one in told]
  assert engine.module(root)[f"{act}_word"] == word and engine.module(root)["hi"] == "hi"


async def test_a_rung_with_no_word_tells_nothing_where_it_is_made() -> None:
  """A rung with no word tells nothing where it is made, since the chain tells it as the last line of the turn it asks for it with."""
  sand, log, root = born()
  act = engine.prompt(int, "count", "m/high", on=root)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [(a[2], a[3]) for a in said(log, "tell") if a[1] == step] == [(root, [f"#{step} advance on {act}"])]
  (asked,) = said(log, "reply")
  assert asked[2] == step and sand.turns[asked[1]][-1][1].split("\n")[-1] == f"#{step} advance on {act}"


async def test_a_rung_with_no_word_and_no_actor_takes_the_default_actor_of_its_chain_when_it_is_made() -> None:
  """A rung with no word and no actor takes the default actor of its chain when it is made, and writes it into its actor word, so its reply and its ledger read the one actor."""
  _, log, root = born(cost=(80000, 0, 0, 0, 0.0))
  engine.grant(usd=10.0, on=root)
  bare = engine.rung(on=root)
  await settle()
  await engine.rung("actor = 'n/low'", on=root)
  (asked,) = said(log, "reply")
  world_says("done", asked[1], ("assistant", "k = 1", (80000, 0, 0, 0, 0.0), None))
  await settle()
  assert [a[6] for a in said(log, "rung") if a[1] == bare] == ["m/low"] == [a[4] for a in said(log, "reply")]
  assert [one for one in paragraphs(engine.turns(on=root)) if one.startswith("#grant1 ledger")] == [
    "#grant1 ledger\ngrant1_spent = 0.0\ngrant1_filled = 0.2"
  ]


async def test_the_raised_paragraph_binds_the_exception_as_rungn_raised() -> None:
  """The raised paragraph binds the exception as rungN_raised, as python shows it, which says its type and its message."""
  _, log, root = born()
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)
  (act,) = [a[1] for a in said(log, "rung")]
  assert of(engine.turns(on=root), act) == [
    f"#{act}\n{act}_word = \"raise ValueError('boom')\"",
    f"#{act} raised\n{act}_raised = ValueError('boom')",
  ]


async def test_a_word_that_raises_a_cancellederror_tells_cancelled_and_no_raised_header() -> None:
  """A word that raises a CancelledError tells cancelled and no raised header, since a cancel is no fault of the word."""
  _, _, root = born()
  act = engine.rung("w = wait(5)\ncancel(w)\nawait w", on=root)
  with pytest.raises(CancelledError):
    await act
  assert [head for head in heads(engine.turns(on=root)) if head.startswith(f"#{act}")] == [
    f"#{act}",
    f"#{act} cancelled",
  ]


async def test_a_rung_that_retells_another_rung_names_its_acts_under_that_one() -> None:
  """A rung that retells another rung names its acts under that one, so it makes the same acts and shares them."""
  _, log, root = born("x = bash('echo hi')\nclose(1)")
  act = engine.prompt(int, "run it", on=root)
  assert await act == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  twin = await chained("twin", root, 300)
  retold = [a for a in said(log, "rung") if a[3] == twin]
  program = engine.program(root)
  assert [a[5] for a in retold] == list(program) and step in program
  assert said(log, "bash") == [("bash", "bash1", step, root, "echo hi", False, 600.0)]
  assert engine.module(twin)["x"] == engine.module(root)["x"] == "bash1"


async def test_a_cancel_of_a_rung_is_the_kernels_to_do() -> None:
  """A cancel of a rung is the Kernel's to do, since the Kernel is the one running the word."""
  _, log, root, act, _, _ = await stalled()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  engine.cancel(step)
  await settle()
  (run,) = [a[1] for a in said(log, "run") if a[4] == step]
  assert [(a[2], type(a[3]).__name__) for a in dones(log, "run") if a[1] == run] == [(run, "CancelledError")]
  assert isinstance(engine.peek(step), CancelledError)
  engine.cancel(act)
  _, log, root = born()
  assert await engine.rung("r = rung('ran = 1')\ncancel(r)", on=root) is None
  await settle()
  inner = engine.module(root)["r"]
  (run,) = [a[1] for a in said(log, "run") if a[4] == inner]
  assert isinstance(engine.peek(run), CancelledError) and "ran" not in engine.module(root)
  assert await engine.rung("b = bash('echo hi')", on=root) is None
  await settle()
  assert [a[5] for a in said(log, "prompt")] == ["bash1 done"]


async def test_the_word_of_a_model_is_python_code_and_nothing_else() -> None:
  """The word of a model is python code and nothing else."""
  _, log, root = born("close(1 + 1)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[5] for a in said(log, "run") if a[4] == step] == ["close(1 + 1)"]
  assert [turn[1] for turn in engine.turns(on=root) if turn[0] == "assistant"] == ["close(1 + 1)"]


async def test_a_step_that_raised_keeps_what_it_bound_before_the_raise() -> None:
  """A step that raised keeps what it bound before the raise."""
  _, _, root = born("a = 1\nraise ValueError('boom')", "close(a + 1)")
  assert await engine.prompt(int, "try", on=root) == 2
  assert engine.module(root)["a"] == 1


async def test_a_rung_may_await_at_its_top_level() -> None:
  """A rung may await at its top level."""
  _, _, root = born()
  assert await engine.rung("out = await bash('echo hi')\nclose(out.code)", on=root) == 0


async def test_a_rung_answers_the_prompt_of_its_chain_with_close() -> None:
  """A rung answers the prompt of its chain with close, wherever in its word the close is said."""
  _, _, root = born()
  assert await engine.rung("close(21)", on=root) == 21


async def test_the_kernel_gives_nothing_for_a_word_that_ran_to_its_end() -> None:
  """The Kernel gives nothing for a word that ran to its end, since a word that answers says a close and stops there."""
  _, log, root = born()
  await engine.rung("close(21)", on=root)
  assert [isinstance(a[3], CancelledError) for a in dones(log, "run")] == [False, True]


async def test_the_kernel_gives_what_a_word_raised() -> None:
  """The Kernel gives what a word raised, and a top-level return is no python, which the gate refuses as it refuses any word that is not python."""
  _, log, root = born()
  ended = engine.rung("k = 1", on=root)
  assert await ended is None
  assert [a[3] for a in dones(log, "run")] == [None, None]
  hurt = engine.rung("raise ValueError('boom')", on=root)
  with pytest.raises(ValueError, match="boom"):
    await hurt
  assert isinstance(dones(log, "run")[3][3], ValueError)
  with pytest.raises(SyntaxError, match="'return' outside function"):
    compile("return 1", "<rung>", "exec")
  broken = engine.rung("k = (", on=root)
  with pytest.raises(Refused):
    await broken
  raised = f"#{hurt} raised\n{hurt}_raised = ValueError('boom')"
  told = [fresh(root, written(ended, "k = 1")), written(hurt, "raise ValueError('boom')")]
  assert gated(log)[-1] == "k = (" and ran(log) == [
    told[0],
    "k = 1",
    told[1],
    "raise ValueError('boom')",
    f"{raised}\n\n{written(broken, 'k = (')}",
  ]


async def test_the_word_of_a_rung_runs_to_its_next_await_and_continues_when_the_close_it_awaits_comes() -> None:
  """The word of a rung runs to its next await and continues when the close it awaits comes."""
  _, log, root = born()
  act = engine.rung("p = prompt(int, 'how many?', to='operator')\nk = 1\nclose(await p)", on=root)
  await settle()
  _, theirs, *_ = said(log, "prompt")[0]
  assert engine.module(root)["k"] == 1 and engine.peek(act, ...) is ...
  engine.close(21, theirs)
  await settle()
  assert (await act) == 21


async def test_the_word_of_a_rung_answers_its_prompt_with_close() -> None:
  """The word of a rung answers its prompt with close, which carries the value the prompt is done with."""
  _, _, root = born("close(21)")
  assert await engine.prompt(int, "how many?", on=root) == 21


async def test_the_turns_of_the_chain_of_another_prompt_tell_the_rung_of_a_word_its_caller_wrote() -> None:
  """The turns of the chain of another prompt tell the rung of a word its caller wrote."""
  sand, log, root = born()
  two = engine.chain("two")
  sand.script[root] = [f"await rung('helper = 2', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "delegate", on=root) == 1
  await settle()
  (helper,) = [a[1] for a in said(log, "rung") if a[4] == "helper = 2"]
  assert paragraphs(engine.turns(on=two)) == [opened(two, "two"), takes(two), written(helper, "helper = 2")]
  assert of(engine.turns(on=root), helper) == []


async def test_rung_is_given_a_word_and_runs_it_on_a_chain_in_the_globals_of_that_chain() -> None:
  """rung is given a word and runs it on a chain in the globals of that chain."""
  _, log, root = born()
  two = engine.chain("two")
  await engine.rung("k = 1", on=two)
  assert engine.module(two)["k"] == 1 and "k" not in engine.module(root)
  assert [(a[4], a[3]) for a in said(log, "rung")] == [("k = 1", two)]


async def test_the_gate_reads_the_word_its_caller_wrote_like_any_word() -> None:
  """The gate reads the word its caller wrote like any word."""
  _, log, root = born()
  laid = engine.rung("k = 1", on=root)
  await laid
  assert gated(log) == ["k = 1"]
  bad = engine.rung("k = BAD", on=root)
  with pytest.raises(Refused):
    await bad
  told = [fresh(root, written(laid, "k = 1")), written(bad, "k = BAD")]
  assert gated(log) == ["k = 1", "k = BAD"] and ran(log) == [told[0], "k = 1", told[1]]


async def test_a_rung_with_a_word_completes_with_what_that_word_raises() -> None:
  """A rung with a word completes with what that word raises, and with nothing when the word runs to its end."""
  _, _, root = born()
  assert await engine.rung("k = 1", on=root) is None
  with pytest.raises(ValueError, match="boom"):
    await engine.rung("raise ValueError('boom')", on=root)


async def test_a_close_ends_the_rung_that_runs_in_a_prompt_at_its_next_await() -> None:
  """A close ends the rung that runs in a prompt at its next await."""
  _, log, _, act, _, _ = await stalled()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  engine.close(21, act)
  await settle()
  assert (await act) == 21 and isinstance(engine.peek(step), CancelledError)


async def test_a_chain_with_a_source_and_its_origin_share_the_one_act() -> None:
  """A chain with a source and its origin share the one act, the record's."""
  sand, log, root = born("x = bash('echo hi')\nclose(1)")
  assert await engine.prompt(int, "run it", on=root) == 1
  _, command, *_ = said(log, "bash")[0]
  twin = await chained("twin", root, 300)
  assert [a[1] for a in said(log, "bash")] == [command]
  assert [a[1] for a in said(sand.calls, "bash")] == [command]
  assert engine.module(twin)["x"] == engine.module(root)["x"] == command


async def test_a_chain_with_a_source_awaits_what_its_origin_started() -> None:
  """A chain with a source awaits what its origin started, through the name of the act."""
  sand, log, root = born("x = bash('slow')\nclose(1)", auto=False)
  assert await engine.prompt(int, "start one", on=root) == 1
  _, command, *_ = said(log, "bash")[0]
  twin = await chained("twin", root, 300)
  sand.script[twin] = ["close((await x).code)"]
  act = engine.prompt(int, "await it", on=twin)
  await settle()
  assert engine.peek(act, ...) is ...
  sand.exits(command, 0)
  await settle()
  assert (await act) == 0 and [a[1] for a in said(log, "bash")] == [command]


async def test_it_says_its_word_may_run_as_soon_as_it_holds_one() -> None:
  """It says its word may run as soon as it holds one, whichever way that word came, and what the chain makes of the word is the chain's."""
  log, _, laid, _, step = await counted()
  assert [a[1:] for a in said(log, "ready")] == [(laid, laid, "k = 1"), (step, step, "close(k + 1)")]
  assert [q[4] for q in acts(log).values() if q[0] == "gate"] == ["k = 1", "close(k + 1)"]
  assert [a[4] for a in said(log, "run")] == [f"{laid}_told", laid, f"{step}_told", step]


async def test_the_chain_has_the_kernel_begin_it_in_the_module_of_that_chain() -> None:
  """The chain has the Kernel begin it in the module of that chain, and the run carries it forward at the done of every act the word waits for, so that the engine owns the order of it."""
  _, log, root = born("x = bash('echo hi')\nout = await x\nclose(out.code)")
  act = engine.prompt(int, "run it", on=root)
  assert await act == 0
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  (run,) = [a for a in said(log, "run") if a[4] == step]
  (wants,) = [a for a in said(log, "wants") if a[2] == run[1]]
  assert (run[2], run[3], wants[4]) == (root, root, "bash1")
  moves = [a for a in log if a[0] in ("started", "done") and a[1] in (run[1], wants[1], "bash1")]
  assert [(a[0], a[1], a[2]) for a in moves] == [
    ("started", run[1], run[1]),
    ("started", "bash1", WORLD),
    ("started", wants[1], step),
    ("done", "bash1", WORLD),
    ("done", wants[1], step),
    ("done", run[1], run[1]),
  ]
  assert moves[3][3] == moves[4][3] and isinstance(moves[5][3], CancelledError)
  out = engine.module(root)["out"]
  assert isinstance(out, Exit) and out.code == 0


async def test_it_is_done_with_what_the_word_gave() -> None:
  """It is done with what the word gave, nothing for a word that ran to its end and the exception for a raise, which it tells with its type and its message, which its close then holds none of."""
  _, log, root = born()
  ended = engine.rung("k = 1", on=root)
  await ended
  answered = engine.rung("close(21)", on=root)
  await answered
  hurt = engine.rung("raise ValueError('boom')", on=root)
  with pytest.raises(ValueError, match="boom"):
    await hurt
  kinds = ["NoneType", "NoneType", "NoneType", "CancelledError", "NoneType", "ValueError"]
  assert [type(a[3]).__name__ for a in dones(log, "run")] == kinds
  assert engine.peek(ended) is None and engine.peek(answered) == 21
  assert isinstance(engine.peek(hurt), ValueError)
  assert of(engine.turns(on=root), hurt) == [
    f"#{hurt}\n{hurt}_word = \"raise ValueError('boom')\"",
    f"#{hurt} raised\n{hurt}_raised = ValueError('boom')",
  ]


async def test_of_the_acts_its_word_made_and_read_at_once_it_tells_nothing() -> None:
  """Of the acts its word made and read at once it tells nothing, since such an act tells of itself or not at all."""
  _, log, root = born("t = read('a.txt')\nclock()\nclose(len(t.lines))", files={"/w/a.txt": "one\n"})
  act = engine.prompt(int, "read it", on=root)
  assert await act == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  told = engine.turns(on=root)
  assert told[2][1] == f"#read1\nread1_path = 'a.txt'\nread1_text = 'one'\n\n#{act} closed\n{act}_value = 1"
  assert of(told, step) == [f"#{step} advance on {act}"]


async def test_an_answer_with_no_text_is_a_word_like_any_other() -> None:
  """An answer with no text is a word like any other, so the gate reads it, the run gives no value, and the model is asked again."""
  _, log, root = born("", "close(1)")
  act = engine.prompt(int, "work", on=root)
  assert await act == 1
  first, second = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [q[4] for q in acts(log).values() if q[0] == "gate"] == ["", "close(1)"]
  assert [(a[4], a[5]) for a in said(log, "run") if a[4] in (first, second)] == [(first, ""), (second, "close(1)")]
  assert engine.peek(first) is None and [a[2] for a in said(log, "reply")] == [first, second]


async def test_a_replay_makes_a_rung_of_its_own_retelling_each_rung_of_the_donor_it_keeps() -> None:
  """A replay makes a rung of its own retelling each rung of the donor it keeps."""
  _, log, root = born("k = 1\nclose(1)", "close(None)", files={"/w/a.txt": "one\n"})
  assert await engine.prompt(int, "work", on=root) == 1
  await settle()
  side = engine.chain("side", source=root)
  await settle(300)
  mine = [a for a in said(log, "rung") if a[3] == side]
  assert {a[5]: a[4] for a in mine} == engine.program(root)
  assert mine and all(a[1] != a[5] for a in mine)


async def test_a_rung_that_retells_names_the_rung_that_the_word_first_ran_in() -> None:
  """A rung that retells names the rung that the word first ran in and never another rung that retells it, so a second replay makes the same acts and asks the World nothing twice."""
  sand, log, root = born()
  act = engine.prompt(int, "read it", to="operator", on=root)
  engine.write(Text(act, "t = read('a.txt')"), on=root)
  await settle()
  engine.write(Text(act, "t = read('a.txt')\nn = len(t.lines)"), on=root)
  await settle()
  engine.write(Text(act, "t = read('a.txt')\nn = len(t.lines)\nm = n + 1"), on=root)
  await settle()
  laid = said(log, "rung")
  first, again, _, second, *_ = [a[1] for a in laid]
  told = f"{first}_told"
  assert [a[5] for a in laid] == ["", told, first, "", f"{again}_told", told, first, f"{second}_told", second, ""]
  assert [a[0] for a in sand.calls].count("read") == 1
  assert engine.module(root)["m"] == 3


async def test_a_rung_that_retells_is_done_with_nothing() -> None:
  """A rung that retells is done with nothing when its word answers, runs to its end or is cancelled, whatever the Kernel makes of the word, and with what that word raised."""
  _, log, root = born("k = 1\nclose(21)", "close(None)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 21
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  hurt = engine.rung("raise ValueError('boom')", on=root)
  with pytest.raises(ValueError, match="boom"):
    await hurt
  stopped = engine.rung("raise CancelledError()", on=root)
  with pytest.raises(CancelledError):
    await stopped
  twin = await chained("twin", root, 300)
  theirs = {a[5]: type(engine.peek(a[1])).__name__ for a in said(log, "rung") if a[3] == twin}
  told = dict.fromkeys([f"{one}_told" for one in (step, hurt, stopped)], "NoneType")
  assert theirs == {**told, step: "NoneType", hurt: "ValueError", stopped: "NoneType"}
  other = sown()
  mine: list[tuple] = []
  over = engine.boot(kernel=kept(Py().kernel()), gate=Py().gating(), probe=watched(mine), world=other.hears())
  other.script[over] = ["k = 1\nclose(21)", "close(None)"]
  assert await engine.prompt(int, "count", on=over) == 21
  await settle()
  side = engine.chain("side", source=over)
  await settle(300)
  assert [engine.peek(a[1]) for a in said(mine, "rung") if a[3] == side] == [None, None]


async def test_a_rung_that_awaits_an_act_nobody_settles_waits_until_the_operator_cancels_it() -> None:
  """A rung that awaits an act nobody settles waits until the operator cancels it, and holds nothing else of the chain."""
  _, _, root = born(auto=False)
  stuck = engine.rung("x = bash('never')\nclose(await x)", on=root)
  other = engine.rung("k = 2\nclose(k)", on=root)
  await settle()
  assert engine.peek(stuck, ...) is ... and await other == 2 and engine.module(root)["k"] == 2
  engine.cancel(stuck)
  await settle()
  assert isinstance(engine.peek(stuck), CancelledError)


async def test_a_rung_that_retells_says_each_question_it_makes_as_the_rung_it_retells() -> None:
  """A rung that retells says each question it makes as the rung it retells, so the question it makes at a place is the one that rung made there."""
  _, log, root = born("t = read('a.txt')\nx = bash('echo hi')\nclose(1)", "close(None)")
  act = engine.prompt(int, "run it", on=root)
  assert await act == 1
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert said(log, "read") == [("read", "read1", step, root, "a.txt")]
  twin = await chained("twin", root, 300)
  (retold,) = [a[1] for a in said(log, "rung") if a[3] == twin and a[5] == step]
  assert [a for a in acts(log).values() if a[0] == "read"] == [("read", "read1", step, root, "a.txt")]
  assert said(log, "bash") == [("bash", "bash1", step, root, "echo hi", False, 600.0)]
  assert retold != step and engine.module(twin)["x"] == "bash1"


async def test_what_a_rung_that_retells_asks_is_named_under_the_one_it_retells() -> None:
  """What a rung that retells asks is named under the one it retells, so the life answers it with what it answered then and the World is asked nothing twice."""
  sand, log, root = born("t = read('a.txt')\nclose(len(t.lines))", "close(None)")
  act = engine.prompt(int, "read it", on=root)
  assert await act == 2
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  twin = await chained("twin", root, 300)
  asked = [one[1] for one in acts(log).values() if one[0] == "read"]
  assert asked == ["read1"] and engine.under(asked[0], step)
  assert [a[0] for a in sand.calls].count("read") == 1
  assert engine.module(twin)["t"] == engine.module(root)["t"]


async def test_a_rung_takes_its_own_act() -> None:
  """A rung takes its own act, since the engine is the one that runs it."""
  sand, log, root = born()
  act = engine.rung("k = 1", on=root)
  await act
  assert [(a[1], a[2]) for a in said(log, "started") if a[1] == act] == [(act, act)]
  assert [a for a in sand.calls if a[1] == act] == []


async def test_a_rung_with_no_word_holds_the_word_of_its_model_when_the_reply_for_it_is_done() -> None:
  """A rung with no word holds the word of its model when the reply for it is done, and it is done with the refusal a reply came to."""
  _, log, root = born()
  bare = engine.rung(on=root)
  await settle()
  (asked,) = said(log, "reply")
  assert [a for a in said(log, "ready") if a[1] == bare] == []
  world_says("done", asked[1], ("assistant", "k = 1", (0, 0, 0, 0, 0.0), None))
  assert await bare is None
  assert [a[3] for a in said(log, "ready") if a[1] == bare] == ["k = 1"] and engine.module(root)["k"] == 1
  other = engine.rung(on=root)
  await settle()
  again = said(log, "reply")[-1]
  world_says("done", again[1], Refused("no model"))
  with pytest.raises(Refused, match=r"^no model$"):
    await other
  assert [a for a in said(log, "ready") if a[1] == other] == []
