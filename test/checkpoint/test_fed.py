"""fed, the entries that follow the tip of a life, said again."""

import pytest

from conftest import WORD, Sand, born, given, kernel, lived, plain, relived, settle, stalled, viewed
from furb import checkpoint, engine
from furb.engine import Exit, Refused, idle


def lived_on(root: str) -> dict[str, dict]:
  """What a word reads of a life, less the view of what its journal said again."""
  seen = viewed(root)
  return {**seen, "facts": {key: facts for key, facts in seen["facts"].items() if key[1] != "journal"}}


async def test_fed_gives_the_journal_of_the_life_the_entries_that_follow_its_tip() -> None:
  """fed gives the journal of the life the entries that follow its tip, which the journal says again at once as it says a record again, so a life that stood at a place of a record and is fed the entries after that place stands where a life that boots on the whole record stands."""
  sand, _, root = await lived()
  record, first = list(sand.record), lived_on(root)
  _, over = await relived(Sand(), record)
  whole = engine.turns(on=over), engine.peek("prompt1"), engine.module(over)["k"]
  stood, _, again = born(WORD)
  at = len(stood.record)
  assert again == root and plain(stood.record) == plain(record[:at]) and at < len(record)
  checkpoint.fed(vars(engine), given(record[at:]))
  await settle()
  assert lived_on(again) == first
  assert (engine.turns(on=again), engine.peek("prompt1"), engine.module(again)["k"]) == whole


async def test_the_outside_is_asked_nothing_that_a_fed_entry_answers() -> None:
  """The outside is asked nothing that a fed entry answers, and the journal keeps no fed entry again."""
  sand, _, root = await lived()
  record = list(sand.record)
  stood, _, again = born(WORD)
  at, asked = len(stood.record), len(stood.calls)
  checkpoint.fed(vars(engine), given(record[at:]))
  await settle()
  assert again == root and engine.peek("prompt1") == 2
  assert len(stood.calls) == asked and stood.script[again] == [WORD] and len(stood.record) == at
  said = [a[:2] for a in engine.transcript(again, "journal")]
  assert said == [e[0][:2] for e in plain(record[at:]) if not engine.question(e[0]) and e[0][0] != "joined"]
  more = engine.bash("echo more", on=again)
  await settle()
  assert more == "bash2" and [a[4] for a in stood.calls if a[0] == "bash"] == ["echo more"]
  assert len(stood.record) > at


async def test_a_fed_answer_of_an_act_that_the_life_holds_lands_at_its_place() -> None:
  """A fed answer of an act that the life holds and that is not over lands at its place, so a word that awaits that act goes on."""
  sand, _, root, act, _, command = await stalled()
  at = len(sand.record)
  sand.exits(command, 0)
  assert await act == 0
  await settle()
  record, first = list(sand.record), lived_on(root)
  stood, _, again, asked, _, held = await stalled()
  assert (again, held) == (root, command) and plain(stood.record) == plain(record[:at]) and at < len(record)
  assert engine.peek(held, ...) is ...
  calls = len(stood.calls)
  checkpoint.fed(vars(engine), given(record[at:]))
  await settle()
  assert engine.peek(asked) == 0 and isinstance(ended := engine.peek(held), Exit) and ended.code == 0
  assert lived_on(again) == first and len(stood.calls) == calls and len(stood.record) == at


async def test_fed_on_a_life_that_no_journal_of_the_checkpointer_keeps_is_refused() -> None:
  """fed on a life that no journal of the checkpointer keeps is refused, since nothing takes a tail there."""
  sand, _, root = await lived()
  record = list(sand.record)
  over = engine.boot(lambda *tables: idle("journal"), **kernel(), world=Sand().hears())  # noqa: ARG005
  with pytest.raises(Refused, match=r"^no journal of the checkpointer keeps this life, so nothing takes a tail$"):
    checkpoint.fed(vars(engine), given(record[2:]))
  assert over == root and engine.transcript(over, "prompt") == []
