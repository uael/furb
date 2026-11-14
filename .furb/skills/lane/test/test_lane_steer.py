"""lane_steer, the questions that the measures of the lane raise."""

from conftest import said
from skills.conftest import Bench


async def test_lane_steer_asks_a_model_for_rows_from_the_measures(laning: Bench) -> None:
  """lane_steer(tree, ground, since) runs measures.sh of the kit in tree, and asks model, the actor of ground by default so that it reads the prefix of ground from its cache, on a chain that goes on from ground and reads the contract of the kit, for at most three rows: a question of the owner, the measure that makes him ask it, and the change of the kit that answers it."""
  kit = laning.tmp / ".furb" / "skills" / "lane"
  kit.mkdir(parents=True)
  (kit / "measures.sh").write_text('echo "head: $1 $2"\n', encoding="utf-8")
  laning.words["steer"] = ['close([("Why?", "head", "a change")])']
  got = await laning.value(f'await lane_steer({str(laning.tmp)!r}, {laning.root!r}, "the items")')
  assert got == [("Why?", "head", "a change")]
  [asked] = [x for x in said(laning.log, "prompt") if x[4] == "list[tuple[str, str, str]]"]
  steer = asked[3]
  assert laning.engine.get(steer)[4:6] == ("steer", laning.root)
  assert asked[6] == laning.engine.module(laning.root)["actor"] == "claude-cli:sonnet/high"
  assert str(asked[5]).startswith("head: . main\nopen jobs past their time: none\n\nThe items since the last time:\nthe items\n")
  assert [x[4] for x in said(laning.log, "read") if x[3] == steer] == ["skills://lane/lane.pyi"]
