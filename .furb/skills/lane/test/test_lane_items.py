"""LANE_ITEMS, the file of the rows of the lane."""

from skills.conftest import Bench


async def test_lane_items_is_a_file_that_git_excludes_until_the_root_binds_another(laning: Bench) -> None:
  """LANE_ITEMS is the file of the rows of the lane, .furb/lane.json, which git excludes, until the root binds another path, which a new load of the kit keeps."""
  assert laning.ns["LANE_ITEMS"] == ".furb/lane.json"
  await laning.word('LANE_ITEMS = "rows.json"')
  await laning.word(laning.text)
  assert laning.ns["LANE_ITEMS"] == "rows.json"
