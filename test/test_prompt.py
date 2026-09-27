"""prompt, the one channel of the engine: a message to an actor, which wants a response of a shape."""

from asyncio import CancelledError

from conftest import COST, STANDS, WORLD, Sand, born, dones, heads, lived, paragraphs, plain, ran, relived, said, settle
from furb import engine
from furb.engine import OPERATOR, Act, Exit, Refused, Text


async def test_a_prompt_it_makes_the_rung_of_one_turn_of_its_model() -> None:
  """A prompt: it makes the rung of one turn of its model, makes another while the rung it made gives no value, and is done with the value, so a rung whose word is refused and a rung whose word raises are asked again alike."""
  _, log, root = born("k = BAD", "raise ValueError('boom')", "close(7)")
  act = engine.prompt(int, "try", on=root)
  assert await act == 7
  steps = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[2] for a in said(log, "reply")] == steps
  assert [type(engine.peek(one)).__name__ for one in steps] == ["Refused", "ValueError", "CancelledError"]


async def test_the_driver_gives_the_name_of_the_prompt() -> None:
  """The driver gives the name of the prompt, and the prompt is awaited for the shape."""
  _, log, root = born("close(7)")
  act = engine.prompt(int, "count", on=root)
  assert isinstance(act, Act) and act == said(log, "prompt")[0][1]
  assert await act == 7


async def test_the_engine_asks_the_model_again_after_a_refusal() -> None:
  """The engine asks the model again after a refusal."""
  _, log, root = born("k = BAD", "close(7)")
  act = engine.prompt(int, "try", on=root)
  assert await act == 7
  first, second = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[2] for a in said(log, "reply")] == [first, second]
  assert engine.turns(on=root)[2][1] == (
    f"#{first} refused\n# line 1: error[unresolved-reference] Name `BAD` used when not defined\n\n"
    f"#{first} closed Refused()\n\n#{second} advance on {act}"
  )


async def test_the_operator_prompts_a_model_to_make_the_model_work() -> None:
  """The operator prompts a model to make the model work."""
  _, log, root = born("x = bash('echo hi')\nclose((await x).code)")
  assert await engine.prompt(int, "run it", on=root) == 0
  assert [one[4] for one in said(log, "bash")] == ["echo hi"]


async def test_a_model_prompts_the_operator_to_tell_the_model_something_or_to_get_a_decision() -> None:
  """A model prompts the operator to tell the model something or to get a decision."""
  _, log, root = born("p = prompt(int, 'how many?', to='operator')\nclose(await p)")
  act = engine.prompt(int, "ask them", on=root)
  await settle()
  theirs = said(log, "prompt")[-1]
  assert (theirs[6], theirs[5]) == (OPERATOR, "how many?")
  engine.close(21, theirs[1])
  await settle()
  assert (await act) == 21


async def test_a_model_prompts_a_model_to_delegate() -> None:
  """A model prompts a model to delegate."""
  sand, log, root = born()
  two = engine.chain("two")
  sand.script[two] = ["close(2)"]
  sand.script[root] = [f"close(await prompt(int, 'count', 'n/low', on={two!r}))"]
  assert await engine.prompt(int, "delegate", on=root) == 2
  assert [one[4] for one in said(log, "reply")] == ["m/low", "n/low"]


async def test_a_prompt_to_a_model_is_a_ladder_of_rungs_in_the_globals_of_its_chain() -> None:
  """A prompt to a model is a ladder of rungs in the globals of its chain."""
  _, _, root = born("a = 1", "b = a + 1", "close(b)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  assert engine.read(act, on=root).content == "a = 1\nb = a + 1\nclose(b)"
  assert (engine.module(root)["a"], engine.module(root)["b"]) == (1, 2)


async def test_nothing_but_a_prompt_asks_a_model() -> None:
  """Nothing but a prompt asks a model."""
  sand, log, root = born()
  await engine.rung("k = 1", on=root)
  engine.bash("echo hi", on=root)
  await settle()
  assert said(log, "reply") == []
  sand.script[root] = ["close(1)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  assert [one[2] for one in said(log, "reply")] == [a[1] for a in said(log, "rung") if a[2] == act]


async def test_the_value_of_a_prompt_on_another_chain_comes_back_as_the_value() -> None:
  """The value of a prompt on another chain comes back as the value, and nothing is wired between the chains of one life."""
  sand, _, root = born()
  two = engine.chain("two")
  sand.script[two] = ["close(Text('p.txt', 'hi'))"]
  sand.script[root] = [f"v = await prompt(Text, 'a text', on={two!r})\nclose([type(v).__name__, v.content])"]
  assert await engine.prompt(list, "delegate", on=root) == ["Text", "hi"]


async def test_prompt_is_given_a_shape_a_message_and_an_actor_on_a_chain() -> None:
  """prompt is given a shape, a message and an actor, on a chain."""
  _, log, root = born()
  engine.prompt(int, "how many?", "n/low", on=root)
  word = said(log, "prompt")[0]
  assert (word[4], word[5], word[6], word[3]) == ("int", "how many?", "n/low", root)


async def test_prompt_gives_the_prompt_which_is_awaited_for_the_shape() -> None:
  """prompt gives the prompt, which is awaited for the shape."""
  _, _, root = born("close('seven')")
  act = engine.prompt(str, "a word", on=root)
  got = await act
  assert isinstance(act, Act) and got == "seven"


async def test_the_response_of_a_prompt_has_the_shape() -> None:
  """The response of a prompt has the shape."""
  _, _, root = born("close(7)")
  got = await engine.prompt(int, "count", on=root)
  assert isinstance(got, int) and got == 7


async def test_none_is_a_shape_of_its_own() -> None:
  """None is a shape of its own."""
  _, log, root = born("k = 1\nclose(None)")
  act = engine.prompt(None, "work", on=root)
  assert await act is None
  assert said(log, "prompt")[0][4] == "None"
  assert paragraphs(engine.turns(on=root))[2] == f"#{act} work\n{act}: Act[None] = Act({act!r})"


async def test_without_a_message_the_actor_reads_the_transcript_alone() -> None:
  """Without a message, the actor reads the transcript alone."""
  sand, log, root = born("close(7)")
  act = engine.prompt(int, on=root)
  assert await act == 7
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert said(log, "prompt")[0][5] == ""
  assert paragraphs(sand.turns[said(log, "reply")[0][1]])[2:] == [
    f"#{act}\n{act}: Act[int] = Act({act!r})",
    f"#{step} advance on {act}",
  ]


async def test_a_model_answers_any_shape() -> None:
  """A model answers any shape."""
  _, _, root = born("close(Text('p.txt', 'hi'))")
  got = await engine.prompt(Text, "a text", on=root)
  assert got == Text("p.txt", "hi")


async def test_a_model_asked_with_the_shape_none_reads_the_message_works_and_closes_with_nothing() -> None:
  """A model asked with the shape None reads the message, works, and closes with nothing."""
  _, log, root = born("x = bash('echo hi')\nawait x\nclose(None)")
  assert await engine.prompt(None, "run a command", on=root) is None
  assert [one[4] for one in said(log, "bash")] == ["echo hi"]


async def test_a_rung_need_not_wait_for_a_prompt_of_shape_none() -> None:
  """A rung need not wait for a prompt of shape None."""
  sand, log, root = born()
  two = engine.chain("two")
  sand.script[root] = [f"prompt(None, 'look at this', to='operator', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "tell them", on=root) == 1
  await settle()
  theirs = said(log, "prompt")[-1]
  assert theirs[3] == two and engine.peek(theirs[1], ...) is ...


async def test_the_actor_left_unsaid_is_the_default_actor_of_the_chain() -> None:
  """The actor left unsaid is the default actor of the chain when the prompt is made, which the prompt writes into the actor word of every rung it makes, so every life asks the one actor the record holds."""
  sand, log, root = born("actor = 'n/low'", "close(1)")
  assert await engine.prompt(int, "count", on=root) == 1
  assert (
    [one[4] for one in said(log, "reply")] == ["m/low", "m/low"] == [one[6] for one in said(log, "rung") if not one[4]]
  )
  assert engine.module(root)["actor"] == "n/low"
  later = Sand(stands=[STANDS[0], "/w", "n/low"])
  again, _ = await relived(later, plain(sand.record))
  assert [one[6] for one in said(again, "rung") if not one[4]] == ["m/low", "m/low"]
  assert [one[4] for one in said(again, "reply")] == ["m/low", "m/low"] and said(later.calls, "reply") == []
  alone, log, root = born(stands=[[[OPERATOR, [], 200000]], "/w", OPERATOR])
  shown = engine.prompt(str, "what now?", on=root)
  await settle()
  assert said(log, "reply") == [] and [a[1] for a in said(alone.calls, "prompt")] == [shown]
  engine.close("go", shown)
  assert await shown == "go"


async def test_a_prompt_to_a_model_runs_in_steps_until_the_prompt_completes() -> None:
  """A prompt to a model runs in steps until the prompt completes."""
  _, log, root = born("a = 1", "b = a + 1", "close(b + 1)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 3
  steps = [a[1] for a in said(log, "rung") if a[2] == act]
  assert [a[2] for a in said(log, "reply")] == steps
  assert [a[5] for a in said(log, "run") if a[4] in steps] == ["a = 1", "b = a + 1", "close(b + 1)"]


async def test_the_binding_of_a_prompt_gives_the_shape_as_python_shows_the_expression() -> None:
  """The binding of a prompt gives the shape as python shows the expression, as prompt1: Act[int] = Act('prompt1')."""
  _, _, root = born()
  engine.prompt(int, "how many?", to=OPERATOR, on=root)
  engine.prompt(list[Text], "some texts?", to=OPERATOR, on=root)
  engine.prompt(Text | None, "a text", to="m/low", on=root)
  await settle()
  bindings = [
    line for one in paragraphs(engine.turns(on=root)) for line in one.split("\n") if line.startswith("prompt")
  ]
  assert bindings == [
    "prompt1: Act[int] = Act('prompt1')",
    "prompt2: Act[list[Text]] = Act('prompt2')",
    "prompt3: Act[Text | None] = Act('prompt3')",
  ]


async def test_a_rung_whose_word_closes_nothing_ends_its_step() -> None:
  """A rung whose word closes nothing ends its step, and the engine asks the model again."""
  _, log, root = born("a = 1", "close(a + 1)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  first, second = [a[1] for a in said(log, "rung") if a[2] == act]
  runs = {a[1]: a[4] for a in said(log, "run") if a[4] in (first, second)}
  assert [(runs[a[1]], type(a[3]).__name__) for a in dones(log, "run") if a[1] in runs] == [
    (first, "NoneType"),
    (second, "CancelledError"),
  ]
  assert engine.peek(first) is None and [a[2] for a in said(log, "reply")] == [first, second]


async def test_the_completion_of_a_prompt_cancels_nothing_under_the_prompt() -> None:
  """The completion of a prompt cancels nothing under the prompt but the words it ran."""
  sand, log, root = born("x = bash('slow')\nclose(1)", auto=False)
  act = engine.prompt(int, "start one", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert (await act) == 1
  (step,) = [a[1] for a in said(log, "rung") if a[2] == act]
  assert isinstance(engine.peek(step), CancelledError)
  sand.exits(command, 0)
  await settle()
  got = engine.peek(command)
  assert isinstance(got, Exit) and got.code == 0


async def test_the_operator_prompts_on_any_chain_by_the_id_of_the_chain() -> None:
  """The operator prompts on any chain, by the id of the chain."""
  sand, log, _ = born()
  two = engine.chain("two")
  sand.script[two] = ["close(2)"]
  assert await engine.prompt(int, "count", on=two) == 2
  assert [one[3] for one in said(log, "prompt")] == [two]


async def test_a_rung_prompts_on_any_chain_by_the_id_of_the_chain() -> None:
  """A rung prompts on any chain, by the id of the chain."""
  sand, log, root = born()
  two = engine.chain("two")
  sand.script[two] = ["close(2)"]
  sand.script[root] = [f"close(await prompt(int, 'count', on={two!r}))"]
  assert await engine.prompt(int, "delegate", on=root) == 2
  assert [one[3] for one in said(log, "prompt")] == [root, two]


async def test_a_prompt_to_the_operator_completes_when_the_operator_closes_the_prompt() -> None:
  """A prompt to the operator completes when the operator closes the prompt."""
  _, _, root = born()
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert engine.peek(act, ...) is ...
  engine.close(21, act)
  await settle()
  assert (await act) == 21


async def test_the_response_of_a_prompt_on_a_chain_with_a_source_comes_to_the_act_the_caller_holds() -> None:
  """The response of a prompt on a chain with a source comes to the act that the caller holds."""
  sand, log, root = born()
  await engine.rung("k = 21", on=root)
  twin = engine.chain("twin", source=root)
  await settle()
  sand.script[twin] = ["close(k)"]
  act = engine.prompt(int, "what is k", on=twin)
  assert await act == 21 and engine.peek(act) == 21
  assert said(log, "prompt") == [("prompt", act, OPERATOR, twin, "int", "what is k", "")]


async def test_it_is_the_ladder_of_its_rungs_and_its_name_is_a_door_of_the_program() -> None:
  """It is the ladder of its rungs, and its name is the door of the program of that ladder, which holds the word of every rung of it for as long as the chain lives."""
  _, _, root = born("a = 1")
  act = engine.prompt(int, "count", on=root)
  await settle()
  assert engine.read(act, on=root).content == "a = 1"
  engine.write(Text(act, "a = 1\nb = 2"), on=root)
  await settle()
  assert engine.read(act, on=root).content == "a = 1\nb = 2"
  assert engine.module(root)["b"] == 2
  engine.close(3, act)
  await settle()
  assert engine.read(act, on=root).content == "a = 1\nb = 2"


async def test_a_word_written_to_its_door_is_a_rung_of_it() -> None:
  """A word written to its door is a rung of it, which answers it as the word of its model does."""
  _, log, root = born("a = 1")
  act = engine.prompt(int, "count", on=root)
  await settle()
  engine.write(Text(act, "a = 1\nclose(5)"), on=root)
  await settle()
  bind = f"{root}: Act[object] = Act({root!r})\n{act}: Act[int] = Act({act!r})"
  assert (await act) == 5 and ran(log) == [bind, "a = 1", bind, "a = 1", "close(5)"]
  assert [a[2] for a in said(log, "rung") if a[4] == "close(5)"] == [act]


async def test_a_prompt_to_the_operator_asks_no_model() -> None:
  """A prompt to the operator asks no model: the World takes it and shows it, and it waits to be closed; in a later life, one the record shows open is shown again only at a wake, and one the record shows closed is shown no more."""
  sand, log, root = born()
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert said(log, "reply") == [] and [one[1] for one in sand.calls if one[0] == "prompt"] == [act]
  assert [a[2] for a in said(log, "started") if a[1] == act] == [WORLD]
  still = Sand()
  _, over = await relived(still, plain(sand.record))
  assert over == root and said(still.calls, "prompt") == []
  engine.wake(over)
  await settle()
  assert [one[1] for one in said(still.calls, "prompt")] == [act]
  engine.close(21, act)
  await settle()
  after = Sand()
  _, over = await relived(after, [*plain(sand.record), *plain(still.record)])
  engine.wake(over)
  await settle()
  assert over == root and said(after.calls, "prompt") == [] and engine.peek(act) == 21


async def test_its_close_tells_what_closed_it_from_outside() -> None:
  """Its close tells what closed it from outside, which the one that closed it says, and nothing of what its rung gave, which the rung has told."""
  sand, log, root = born()
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(21, act)
  await settle()
  sand.script[root] = ["raise ValueError('boom')", "close(7)"]
  other = engine.prompt(int, "count", on=root)
  assert await other == 7
  hurt, step = [a[1] for a in said(log, "rung") if a[2] == other]
  assert [(a[1], a[2]) for a in said(log, "close")] == [(act, OPERATOR), (other, step)]
  told = heads(engine.turns(on=root))
  assert [one for one in told if one.split(" ")[1:2] == ["closed"]] == [f"#{act} closed 21", f"#{other} closed 7"]
  assert [(a[2], a[3]) for a in said(log, "tell") if a[1] == hurt][-1] == (hurt, [f"#{hurt} raised ValueError('boom')"])


async def test_a_paused_prompt_makes_no_rung_until_the_wake() -> None:
  """A paused prompt makes no rung until the wake, and a cancel of it is over its rung too, which ends itself."""
  sand, log, root = born("a = 1", "close(a + 1)")
  act = engine.prompt(int, "count", on=root)
  engine.pause(act)
  await settle()
  assert len([a for a in said(log, "rung") if a[2] == act]) == 1 and engine.peek(act, ...) is ...
  engine.wake(act)
  await settle()
  assert (await act) == 2 and len([a for a in said(log, "rung") if a[2] == act]) == 2
  sand.script[root] = ["await wait(9)\nclose(1)"]
  other = engine.prompt(int, "wait", on=root)
  await settle()
  engine.cancel(other)
  await settle()
  (step,) = [a[1] for a in said(log, "rung") if a[2] == other]
  assert isinstance(engine.peek(other), CancelledError)
  engine.pause(root)
  held = engine.prompt(int, "held", on=root)
  await settle()
  assert [a for a in said(log, "rung") if a[2] == held] == []
  sand.script[root] = ["close(3)"]
  engine.wake(root)
  assert await held == 3 and len([a for a in said(log, "rung") if a[2] == held]) == 1
  assert isinstance(engine.peek(step), CancelledError)


async def test_the_world_closes_with_a_refusal_a_prompt_it_cannot_put_to_the_operator() -> None:
  """The World closes with a refusal a prompt it cannot put to the operator; which shapes the operator answers is the World's law."""
  _, _, root = born()
  act = engine.prompt(Text, "a text?", to=OPERATOR, on=root)
  await settle()
  assert isinstance(engine.peek(act), Refused)
  fine = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert engine.peek(fine, ...) is ...
  engine.close(21, fine)


async def test_the_shape_left_unsaid_is_none() -> None:
  """The shape left unsaid is None, which the acknowledgment uses, and any value responds to it."""
  _, log, root = await lived()
  assert [one[4] for one in said(log, "prompt")] == ["int", "None"]
  act = engine.prompt(None, "look at this", to=OPERATOR, on=root)
  await settle()
  engine.close("anything", act)
  await settle()
  assert (await act) == "anything"


async def test_a_prompt_takes_any_shape_which_a_close_is_read_against_as_python_reads_an_instance() -> None:
  """A prompt takes any shape, which a close is read against as python reads an instance: of the shape, or of the origin of a generic one."""
  _, log, root = born(
    "close(Text('p.txt', 'hi'))", "close([1, 2])", "close([])", "close(None)", "close('no')", "close(2)"
  )
  assert await engine.prompt(Text, "a text", on=root) == Text("p.txt", "hi")
  assert await engine.prompt(list[int], "some numbers", on=root) == [1, 2]
  # The name of a shape is the word a chain would say, and the globals of a chain hold no module, so a shape that
  # holds a name of the engine is said as the engine says it and never under the module it was defined in.
  assert await engine.prompt(list[Text], "texts", on=root) == []
  assert await engine.prompt(Text | None, "a text or nothing", on=root) is None
  assert await engine.prompt(int, "a number", on=root) == 2
  (no,) = [a[1] for a in said(log, "ready") if a[3] == "close('no')"]
  raised = [one for one in heads(engine.turns(on=root)) if one.split(" ")[1:2] == ["raised"]]
  assert raised == [f"#{no} raised Refused(\"'no' not int\")"]


async def test_a_prompt_carries_the_name_of_its_shape_as_a_word() -> None:
  """A prompt carries the name of its shape as a word, and takes the name as well as the shape, so the journal makes it again."""
  sand, log, root = born("close(7)")
  assert await engine.prompt(int, "count", on=root) == 7
  sand.script[root] = ["close(8)"]
  assert await engine.prompt("int", "count again", on=root) == 8
  await settle()
  assert [one[4] for one in said(log, "prompt")] == ["int", "int"]
  named = [one[1] for one in said(log, "prompt")]
  again, over = await relived(Sand(), plain(sand.record))
  assert over == root and [one[1] for one in said(again, "prompt")] == named


async def test_the_acknowledgment_carries_no_shape_and_a_message_that_names_the_act_that_is_done() -> None:
  """The acknowledgment carries no shape and a message that names the act that is done."""
  _, log, _ = await lived()
  command = said(log, "bash")[0][1]
  assert [(one[4], one[5]) for one in said(log, "prompt")] == [("int", "read and run"), ("None", f"{command} done")]


async def test_the_turns_of_the_chain_hold_the_result_of_the_command_the_acknowledgment_names() -> None:
  """The turns of the chain hold the result of the command that the acknowledgment names."""
  _, log, root = await lived()
  command = said(log, "bash")[0][1]
  ack = said(log, "prompt")[-1][1]
  assert said(log, "prompt")[-1][5] == f"{command} done"
  told = paragraphs(engine.turns(on=root))
  result = told.index(f"#{command} exited 0\n# {command}/stdout, 0 known\n# 1 ran echo hi")
  assert result < told.index(f"#{ack} {command} done\n{ack}: Act[None] = Act({ack!r})")


async def test_a_cancelled_result_is_no_orphan() -> None:
  """A cancelled result is no orphan."""
  _, log, root = born("x = bash('slow')\nclose(1)", auto=False)
  act = engine.prompt(int, "start one", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert (await act) == 1
  engine.cancel(command)
  await settle()
  assert isinstance(engine.peek(command), CancelledError)
  assert [one[5] for one in said(log, "prompt")] == ["start one"]


async def test_the_response_of_an_acknowledgment_is_no_orphan() -> None:
  """The response of an acknowledgment is no orphan."""
  _, log, root = await lived()
  command = said(log, "bash")[0][1]
  assert [one[5] for one in said(log, "prompt")] == ["read and run", f"{command} done"]
  await settle(200)
  assert len(said(log, "prompt")) == 2 and engine.turns(on=root)[-1][0] == "user"


async def test_when_an_act_a_rung_of_the_chain_made_is_done_the_chain_prompts_nothing() -> None:
  """When an act that a rung of the chain made is done while the word of that rung does not run, no reply has shown it, no prompt it heard on itself is open and no word of the chain is running, the chain prompts nothing, so that the model sees it."""
  sand, log, root = born("x = bash('slow')\nclose(1)", auto=False)
  act = engine.prompt(int, "start one", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert (await act) == 1
  assert [one[5] for one in said(log, "prompt")] == ["start one"]
  sand.exits(command, 0)
  await settle()
  assert [one[5] for one in said(log, "prompt")] == ["start one", f"{command} done"]


async def test_a_pause_stands_over_the_close_that_answers_a_prompt_too() -> None:
  """A pause stands over the close that answers a prompt too, so what a word gave waits for the wake."""
  sand, log, root = born(cost=COST)
  engine.grant(usd=1.0, on=root)
  sand.script[root] = ["close(5)"]
  act = engine.prompt(int, "spend", on=root)
  await settle()
  assert [one[3] for one in said(log, "close")] == [5] and engine.peek(act, ...) is ...
  engine.wake(root)
  await settle()
  assert (await act) == 5


async def test_the_name_of_a_shape_is_the_word_a_chain_says_it_by() -> None:
  """The name of a shape is the word a chain says it by, so a shape that holds a class of the engine or of the chain names it as the chain does, under no module."""
  _, log, root = born(
    "class Foo:\n  pass\nclose([await prompt(list[Foo], 'items'), await prompt(Foo | None, 'maybe')])",
    "close([])",
    "close(None)",
  )
  assert await engine.prompt(list, "work", on=root) == [[], None]
  # The chain acknowledges each act a rung made once it is done, with a prompt of no shape, which is not one of these.
  assert [one[4] for one in said(log, "prompt") if one[4] != "None"] == ["list", "list[Foo]", "Foo | None"]
  assert [one for one in heads(engine.turns(on=root)) if one.split(" ")[1:2] in (["raised"], ["refused"])] == []


async def test_a_prompt_tells_its_message_and_its_binding_where_it_is_made() -> None:
  """A prompt tells its message and its binding where it is made, as every act tells its open, whether a model or the operator answers it."""
  _, log, root = born("a = 1", "close(2)", "close(3)")
  first = engine.prompt(int, "count\nto three", on=root)
  second = engine.prompt(int, "count again", on=root)
  mine = f"#{first} count\n# to three\n{first}: Act[int] = Act({first!r})"
  theirs = f"#{second} count again\n{second}: Act[int] = Act({second!r})"
  assert [(a[1], a[2]) for a in said(log, "tell") if a[1] in (first, second)] == [(first, first), (second, second)]
  told = next(a for a in said(log, "tell") if a[1] == first)
  assert log.index(said(log, "prompt")[0]) < log.index(told) < log.index(said(log, "prompt")[1])
  assert (await first, await second) == (3, 2)
  users = [turn for turn in engine.turns(on=root) if turn[0] == "user"]
  assert [[x for x in paragraphs([turn]) if x in (mine, theirs)] for turn in users] == [[mine], [theirs], [], []]
  act = engine.prompt(int, "how many?\nsay one", to=OPERATOR, on=root)
  await settle()
  made = log.index(engine.get(act))
  assert log[made + 1] == ("tell", act, act, [f"#{act} how many?\n# say one", f"{act}: Act[int] = Act({act!r})"])
  assert [a for a in said(log, "tell") if a[1] == act] == [log[made + 1]] and len(said(log, "reply")) == 3


async def test_a_prompt_to_a_model_takes_its_own_act() -> None:
  """A prompt to a model takes its own act, since the engine is the one that asks the model."""
  sand, log, root = born("close(1)")
  act = engine.prompt(int, "count", on=root)
  assert await act == 1
  assert [(a[1], a[2]) for a in said(log, "started") if a[1] == act] == [(act, act)]
  assert [a for a in sand.calls if a[1] == act] == []
  shown = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert [(a[1], a[2]) for a in said(log, "started") if a[1] == shown] == [(shown, WORLD)]
