"""prompt, the one channel of the engine: a message to an actor, which wants a response of a shape."""

from asyncio import CancelledError

from conftest import STANDS, Py, Sand, attr, life, lived, plain, relived, said, settle, sown, tags
from furb import engine
from furb.engine import OPERATOR, WORLD, Act, Exit, Refused, Text

COST = (80000, 0, 0, 0, 1.5)
"""One answer of a model: a dollar and a half, and a fifth of the window of the actor the suite stands on."""


async def test_a_prompt_it_makes_the_rung_of_one_turn_of_its_model() -> None:
  """A prompt: it makes the rung of one turn of its model, makes another while the rung it made gives no value, and is done with the value, so a rung whose word is refused and a rung whose word raises are asked again alike."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["BAD = 1", "raise ValueError('boom')", "close(7)"]
  assert await engine.prompt(int, "try", on=root) == 7
  assert len(said(log, "rung")) == 3 and len(said(log, "ask")) == 3


async def test_the_driver_gives_the_name_of_the_prompt() -> None:
  """The driver gives the name of the prompt, and the prompt is awaited for the shape."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  act = engine.prompt(int, "count", on=root)
  assert isinstance(act, Act) and act == said(log, "prompt")[0][1]
  assert await act == 7


async def test_the_engine_asks_the_model_again_after_a_refusal() -> None:
  """The engine asks the model again after a refusal."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["BAD = 1", "close(7)"]
  assert await engine.prompt(int, "try", on=root) == 7
  assert len(said(log, "ask")) == 2
  assert [tag[2] for tag in tags(engine.turns(on=root), "refused")] == ["BAD in rung, against int after 0 rungs"]


async def test_the_operator_prompts_a_model_to_make_the_model_work() -> None:
  """The operator prompts a model to make the model work."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nclose((await x).code)"]
  assert await engine.prompt(int, "run it", on=root) == 0
  assert [one[4] for one in said(log, "bash")] == ["echo hi"]


async def test_a_model_prompts_the_operator_to_tell_the_model_something_or_to_get_a_decision() -> None:
  """A model prompts the operator to tell the model something or to get a decision."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["p = prompt(int, 'how many?', to='operator')\nclose(await p)"]
  act = engine.prompt(int, "ask them", on=root)
  await settle()
  theirs = said(log, "prompt")[-1]
  assert (theirs[6], theirs[5]) == (OPERATOR, "how many?")
  engine.close(21, theirs[1])
  await settle()
  assert (await act) == 21


async def test_a_model_prompts_a_model_to_delegate() -> None:
  """A model prompts a model to delegate."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  sand.script[two] = ["close(2)"]
  sand.script[root] = [f"close(await prompt(int, 'count', 'n/low', on={two!r}))"]
  assert await engine.prompt(int, "delegate", on=root) == 2
  assert [one[4] for one in said(log, "ask")] == ["m/low", "n/low"]


async def test_a_prompt_to_a_model_is_a_ladder_of_rungs_in_the_globals_of_its_chain() -> None:
  """A prompt to a model is a ladder of rungs in the globals of its chain."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["a = 1", "b = a + 1", "close(b)"]
  act = engine.prompt(int, "count", on=root)
  assert await act == 2
  assert engine.read(act, on=root).content == "a = 1\nb = a + 1\nclose(b)"
  assert (engine.modules[root]["a"], engine.modules[root]["b"]) == (1, 2)


async def test_nothing_but_a_prompt_asks_a_model() -> None:
  """Nothing but a prompt asks a model."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  await engine.rung("k = 1", on=root)
  engine.bash("echo hi", on=root)
  await settle()
  assert said(log, "ask") == []
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert [one[1] for one in said(log, "ask")] == [said(log, "rung")[-1][1]]


async def test_the_value_of_a_prompt_on_another_chain_comes_back_as_the_value() -> None:
  """The value of a prompt on another chain comes back as the value, and nothing is wired between the chains of one life."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  two = engine.chain("two")
  sand.script[two] = ["close(Text('p.txt', 'hi'))"]
  sand.script[root] = [f"v = await prompt(Text, 'a text', on={two!r})\nclose([type(v).__name__, v.content])"]
  assert await engine.prompt(list, "delegate", on=root) == ["Text", "hi"]


async def test_prompt_is_given_a_shape_a_message_and_an_actor_on_a_chain() -> None:
  """prompt is given a shape, a message and an actor, on a chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  engine.prompt(int, "how many?", "n/low", on=root)
  word = said(log, "prompt")[0]
  assert (word[4], word[5], word[6], word[3]) == ("int", "how many?", "n/low", root)


async def test_prompt_gives_the_prompt_which_is_awaited_for_the_shape() -> None:
  """prompt gives the prompt, which is awaited for the shape."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["close('seven')"]
  act = engine.prompt(str, "a word", on=root)
  got = await act
  assert isinstance(act, Act) and got == "seven"


async def test_the_response_of_a_prompt_has_the_shape() -> None:
  """The response of a prompt has the shape."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["close(7)"]
  got = await engine.prompt(int, "count", on=root)
  assert isinstance(got, int) and got == 7


async def test_none_is_a_shape_of_its_own() -> None:
  """None is a shape of its own."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["k = 1\nclose(None)"]
  act = engine.prompt(None, "work", on=root)
  assert await act is None
  assert said(log, "prompt")[0][4] == "None"
  assert attr(tags(engine.turns(on=root), "opened")[2], "shape") == "None"


async def test_without_a_message_the_actor_reads_the_transcript_alone() -> None:
  """Without a message, the actor reads the transcript alone."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  assert await engine.prompt(int, on=root) == 7
  opened = [tag for tag in tags(said(log, "ask")[0][5], "opened") if str(attr(tag, "id")).startswith("prompt://")]
  assert [attr(tag, "message") for tag in opened] == [""]


async def test_a_model_answers_any_shape() -> None:
  """A model answers any shape."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["close(Text('p.txt', 'hi'))"]
  got = await engine.prompt(Text, "a text", on=root)
  assert got == Text("p.txt", "hi")


async def test_a_model_asked_with_the_shape_none_reads_the_message_works_and_closes_with_nothing() -> None:
  """A model asked with the shape None reads the message, works, and closes with nothing."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["x = bash('echo hi')\nawait x\nclose(None)"]
  assert await engine.prompt(None, "run a command", on=root) is None
  assert [one[4] for one in said(log, "bash")] == ["echo hi"]


async def test_a_rung_need_not_wait_for_a_prompt_of_shape_none() -> None:
  """A rung need not wait for a prompt of shape None."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  sand.script[root] = [f"prompt(None, 'look at this', to='operator', on={two!r})\nclose(1)"]
  assert await engine.prompt(int, "tell them", on=root) == 1
  await settle()
  theirs = said(log, "prompt")[-1]
  assert theirs[3] == two and theirs[1] not in engine.outcomes


async def test_the_actor_left_unsaid_is_the_default_actor_of_the_chain() -> None:
  """The actor left unsaid is the default actor of the chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(1)"]
  assert await engine.prompt(int, "count", on=root) == 1
  assert [one[4] for one in said(log, "ask")] == ["m/low"] == [engine.modules[root]["actor"]]


async def test_a_prompt_to_a_model_runs_in_steps_until_the_prompt_completes() -> None:
  """A prompt to a model runs in steps until the prompt completes."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "b = a + 1", "close(b + 1)"]
  assert await engine.prompt(int, "count", on=root) == 3
  assert len(said(log, "ask")) == 3 and len(said(log, "run")) == 3


async def test_the_opened_tag_of_a_prompt_tells_the_shape_as_python_shows_the_expression() -> None:
  """The opened tag of a prompt tells the shape as python shows the expression."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  engine.prompt(int, "how many?", to=OPERATOR, on=root)
  engine.prompt(Text, "a text", to="m/low", on=root)
  opened = [tag for tag in tags(engine.turns(on=root), "opened") if str(attr(tag, "id")).startswith("prompt://")]
  assert [attr(tag, "shape") for tag in opened] == ["int", "Text"]


async def test_a_rung_whose_word_closes_nothing_ends_its_step() -> None:
  """A rung whose word closes nothing ends its step, and the engine asks the model again."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  assert await engine.prompt(int, "count", on=root) == 2
  assert [one[3] is None for one in said(log, "ran")] == [True, False]
  assert len(said(log, "ask")) == 2


async def test_the_completion_of_a_prompt_cancels_nothing_under_the_prompt() -> None:
  """The completion of a prompt cancels nothing under the prompt but the words it ran."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(1)"]
  act = engine.prompt(int, "start one", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert (await act) == 1
  assert isinstance(engine.peek(said(log, "rung")[0][1], on=root), CancelledError)
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  got = engine.peek(command, on=root)
  assert isinstance(got, Exit) and got.code == 0


async def test_the_operator_prompts_on_any_chain_by_the_id_of_the_chain() -> None:
  """The operator prompts on any chain, by the id of the chain."""
  sand = Sand(stands=STANDS)
  log, _ = life(sand)
  two = engine.chain("two")
  sand.script[two] = ["close(2)"]
  assert await engine.prompt(int, "count", on=two) == 2
  assert [one[3] for one in said(log, "prompt")] == [two]


async def test_a_rung_prompts_on_any_chain_by_the_id_of_the_chain() -> None:
  """A rung prompts on any chain, by the id of the chain."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  two = engine.chain("two")
  sand.script[two] = ["close(2)"]
  sand.script[root] = [f"close(await prompt(int, 'count', on={two!r}))"]
  assert await engine.prompt(int, "delegate", on=root) == 2
  assert [one[3] for one in said(log, "prompt")] == [root, two]


async def test_a_prompt_to_the_operator_completes_when_the_operator_closes_the_prompt() -> None:
  """A prompt to the operator completes when the operator closes the prompt."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert act not in engine.outcomes
  engine.close(21, act)
  await settle()
  assert (await act) == 21


async def test_the_response_of_a_prompt_on_a_chain_with_a_source_comes_to_the_act_the_caller_holds() -> None:
  """The response of a prompt on a chain with a source comes to the act that the caller holds."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  await engine.rung("k = 21", on=root)
  twin = engine.chain("twin", source=root)
  await settle()
  sand.script[twin] = ["close(k)"]
  act = engine.prompt(int, "what is k", on=twin)
  assert await act == 21 and act.startswith("prompt://")


async def test_it_is_the_ladder_of_its_rungs_and_of_the_words_written_to_it() -> None:
  """It is the ladder of its rungs and of the words written to it, so a write of its name makes a rung of what is written under it, and a read of its name the chain answers, which holds every word of every ladder for as long as the chain lives."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  sand.script[root] = ["a = 1"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  engine.write(Text(act, "b = 2"), on=root)
  await settle()
  assert engine.read(act, on=root).content == "a = 1\nb = 2"
  assert engine.modules[root]["b"] == 2
  engine.close(3, act)
  await settle()
  assert engine.read(act, on=root).content == "a = 1\nb = 2"


async def test_a_word_written_to_it_answers_it_not_whoever_wrote_it() -> None:
  """A word written to it answers it not, whoever wrote it, so it must give nothing, and the gate reads it against no shape, as it reads every word a caller wrote."""
  sand, py = Sand(stands=STANDS), Py()
  _, root = life(sand, kernel=py)
  sand.script[root] = ["a = 1"]
  act = engine.prompt(int, "count", on=root)
  await settle()
  engine.write(Text(act, "close(5)"), on=root)
  await settle()
  assert act not in engine.outcomes
  assert [shape for _, _, shape in py.gates] == ["int", ""]


async def test_a_prompt_to_the_operator_asks_no_model() -> None:
  """A prompt to the operator asks no model: the World is shown it, and it waits to be closed; one the record holds is shown no more, since the close it waits for stands there already."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert said(log, "ask") == [] and [one[1] for one in sand.calls if one[0] == "start"] == [act]
  engine.close(21, act)
  await settle()
  after = Sand(stands=STANDS)
  _, over = await relived(after, plain(sand.record))
  assert over == root
  assert [one for one in after.calls if one[0] == "start"] == [] and engine.outcomes[act] == 21


async def test_its_close_tells_what_closed_it_from_outside() -> None:
  """Its close tells what closed it from outside, which the one that closed it says, and nothing of what its rung gave, which the rung has told."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  act = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  engine.close(21, act)
  await settle()
  shut = tags(engine.turns(on=root), "closed")
  assert [(attr(tag, "over"), tag[2]) for tag in shut] == [(act, "21")]
  sand.script[root] = ["close(7)"]
  assert await engine.prompt(int, "count", on=root) == 7
  shut = tags(engine.turns(on=root), "closed")
  assert [dict(tag[1]) for tag in shut] == [{"over": act}, {"over": said(log, "prompt")[-1][1]}]


async def test_a_paused_prompt_makes_no_rung_until_the_wake() -> None:
  """A paused prompt makes no rung until the wake, and a cancel of it is over its rung too, which ends itself."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["a = 1", "close(a + 1)"]
  act = engine.prompt(int, "count", on=root)
  engine.pause(act)
  await settle()
  assert len(said(log, "rung")) == 1 and act not in engine.outcomes
  engine.wake(act)
  await settle()
  assert (await act) == 2 and len(said(log, "rung")) == 2
  sand.script[root] = ["await wait(9)\nclose(1)"]
  other = engine.prompt(int, "wait", on=root)
  await settle()
  engine.cancel(other)
  await settle()
  assert isinstance(engine.outcomes[other], CancelledError)
  assert isinstance(engine.peek(said(log, "rung")[-1][1], on=root), CancelledError)


async def test_the_world_closes_with_a_refusal_a_prompt_it_cannot_put_to_the_operator() -> None:
  """The World closes with a refusal a prompt it cannot put to the operator; which shapes the operator answers is the World's law."""
  sand = Sand(stands=STANDS)
  _, root = life(sand)
  act = engine.prompt(Text, "a text?", to=OPERATOR, on=root)
  await settle()
  assert isinstance(engine.outcomes[act], Refused)
  fine = engine.prompt(int, "how many?", to=OPERATOR, on=root)
  await settle()
  assert fine not in engine.outcomes
  engine.close(21, fine)


async def test_the_shape_left_unsaid_is_none() -> None:
  """The shape left unsaid is None, which the acknowledgment uses, and any value responds to it."""
  sand = sown()
  log, root = await lived(sand)
  assert [one[4] for one in said(log, "prompt")] == ["int", "None"]
  act = engine.prompt(None, "look at this", to=OPERATOR, on=root)
  await settle()
  engine.close("anything", act)
  await settle()
  assert (await act) == "anything"


async def test_a_prompt_takes_any_shape_and_the_gate_reads_the_word_against_the_name_of_it() -> None:
  """A prompt takes any shape, and the gate reads the word of a rung against the name of it."""
  sand, py = Sand(stands=STANDS), Py()
  _, root = life(sand, kernel=py)
  sand.script[root] = ["close(Text('p.txt', 'hi'))", "close([1, 2])", "close([])"]
  assert await engine.prompt(Text, "a text", on=root) == Text("p.txt", "hi")
  assert await engine.prompt(list[int], "some numbers", on=root) == [1, 2]
  # The name of a shape is the word a chain would say, and the globals of a chain hold no module, so a shape that
  # holds a name of the engine is said as the engine says it and never under the module it was defined in.
  assert await engine.prompt(list[Text], "texts", on=root) == []
  assert [shape for _, _, shape in py.gates] == ["Text", "list[int]", "list[Text]"]


async def test_a_prompt_carries_the_name_of_its_shape_as_a_word() -> None:
  """A prompt carries the name of its shape as a word, and takes the name as well as the shape, so the record replays it."""
  sand = Sand(stands=STANDS)
  log, root = life(sand)
  sand.script[root] = ["close(7)"]
  assert await engine.prompt(int, "count", on=root) == 7
  sand.script[root] = ["close(8)"]
  assert await engine.prompt("int", "count again", on=root) == 8
  await settle()
  assert [one[4] for one in said(log, "prompt")] == ["int", "int"]
  named = [one[1] for one in said(log, "prompt")]
  again, over = await relived(Sand(stands=STANDS), plain(sand.record))
  assert over == root and [one[1] for one in said(again, "prompt")] == named


async def test_the_acknowledgment_carries_no_shape_and_a_message_that_names_the_act_that_is_done() -> None:
  """The acknowledgment carries no shape and a message that names the act that is done."""
  sand = sown()
  log, _ = await lived(sand)
  command = said(log, "bash")[0][1]
  assert [(one[4], one[5]) for one in said(log, "prompt")] == [("int", "read and run"), ("None", f"{command} is done")]


async def test_the_turns_of_the_chain_hold_the_result_of_the_command_the_acknowledgment_names() -> None:
  """The turns of the chain hold the result of the command that the acknowledgment names."""
  sand = sown()
  log, root = await lived(sand)
  command = said(log, "bash")[0][1]
  assert said(log, "prompt")[-1][5] == f"{command} is done"
  shut = [tag for tag in tags(engine.turns(on=root), "closed") if ("id", command) in tag[1]]
  assert [attr(tag, "code") for tag in shut] == [0]


async def test_a_cancelled_result_is_no_orphan() -> None:
  """A cancelled result is no orphan."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(1)"]
  act = engine.prompt(int, "start one", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert (await act) == 1
  engine.cancel(command)
  await settle()
  assert isinstance(engine.outcomes[command], CancelledError)
  assert [one[5] for one in said(log, "prompt")] == ["start one"]


async def test_the_response_of_an_acknowledgment_is_no_orphan() -> None:
  """The response of an acknowledgment is no orphan."""
  sand = sown()
  log, root = await lived(sand)
  command = said(log, "bash")[0][1]
  assert [one[5] for one in said(log, "prompt")] == ["read and run", f"{command} is done"]
  await settle(200)
  assert len(said(log, "prompt")) == 2 and engine.turns(on=root)[-1][0] == "user"


async def test_when_an_act_a_rung_of_the_chain_made_is_done_the_chain_prompts_nothing() -> None:
  """When an act a rung of the chain made is done, no ask has shown it, no prompt it heard on itself is open and no word of the chain is running, the chain prompts nothing, so that the model sees it."""
  sand = Sand(stands=STANDS, auto=False)
  log, root = life(sand)
  sand.script[root] = ["x = bash('slow')\nclose(1)"]
  act = engine.prompt(int, "start one", on=root)
  await settle()
  command = said(log, "bash")[0][1]
  assert (await act) == 1
  assert [one[5] for one in said(log, "prompt")] == ["start one"]
  engine.send("exited", command, 0, by=WORLD)
  await settle()
  assert [one[5] for one in said(log, "prompt")] == ["start one", f"{command} is done"]


async def test_a_pause_stands_over_the_close_that_answers_a_prompt_too() -> None:
  """A pause stands over the close that answers a prompt too, so what a word gave waits for the wake."""
  sand = Sand(stands=STANDS, cost=COST)
  log, root = life(sand)
  engine.grant(usd=1.0, on=root)
  sand.script[root] = ["close(5)"]
  act = engine.prompt(int, "spend", on=root)
  await settle()
  assert [one[3] for one in said(log, "close")] == [5] and act not in engine.outcomes
  engine.wake(root)
  await settle()
  assert (await act) == 5
