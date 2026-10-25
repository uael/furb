"""imported, the chain made to go on from a chain of another record."""

from pathlib import Path

from conftest import settle
from extensions.conftest import importing
from furb import engine


async def test_imported_makes_a_chain_whose_rung_awaits_inherit(tmp_path: Path) -> None:
  """imported(record, source, label) makes a chain labeled label, or source of record when it is given no label, or takes the chain on, puts on it a rung that the chain makes, so that the rung outlives the word that called imported, whose word awaits inherit(record, source), and gives the chain."""
  _, root, heard = importing(tmp_path, ())
  word = "mine = chain('mine')\nclose([imported('old.jsonl', label='old'), imported('o.jsonl', 'chain2'), imported('m.jsonl', on=mine)])"
  made = await engine.rung(word, on=root)
  await settle()
  assert isinstance(made, list) and len(made) == 3
  assert [engine.get(one)[4] for one in made] == ["old", "chain2 of o.jsonl", "mine"]
  words = [
    [a[4] for a in engine.transcript(one) if a[0] == "rung" and str(a[4]).startswith("await inherit")] for one in made
  ]
  assert words == [
    ["await inherit('old.jsonl', 'chain1')"],
    ["await inherit('o.jsonl', 'chain2')"],
    ["await inherit('m.jsonl', 'chain1')"],
  ]
  assert sorted((a[3], a[4], a[5]) for a in heard) == sorted(
    [(made[0], "old.jsonl", "chain1"), (made[1], "o.jsonl", "chain2"), (made[2], "m.jsonl", "chain1")]
  )
