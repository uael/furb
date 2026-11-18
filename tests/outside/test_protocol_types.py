import importlib.util
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "crates" / "xtask" / "python" / "protocol.py"


def test_the_types_of_the_protocol_for_python_are_what_the_schema_makes() -> None:
  spec = importlib.util.spec_from_file_location("protocol_types", SCRIPT)
  assert spec is not None and spec.loader is not None
  script = importlib.util.module_from_spec(spec)
  spec.loader.exec_module(script)
  assert script.TYPES.read_text() == script.generated(), "run cargo xtask protocol"
