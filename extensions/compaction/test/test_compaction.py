"""compaction, the doctrine of the extension."""

from pathlib import Path

from extensions.conftest import compacting
from furb import engine


async def test_compaction_is_the_doctrine_of_the_extension(tmp_path: Path) -> None:
  """compaction is the doctrine of the extension, a quote that says what each part does and how a project changes the defaults."""
  sand, root, _ = compacting(tmp_path)
  sand.script[root] = ["close(compaction)"]
  doctrine = await engine.prompt(str, "doctrine", on=root)
  parts = ["compact()", "latest()", "quiet()", "recent(n)", "chosen()", "together(", "compacted()", "SHARE", "FILTERS"]
  assert [part for part in [*parts, "CHOOSER", "LABEL", "extension of its own"] if part not in doctrine] == []
