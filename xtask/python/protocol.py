"""Make the types of the protocol for a process of Python, src/furb/protocol.py, from host/protocol.schema.json.

`bun run protocol` runs it after it makes the same types for TypeScript: run that after a change of the protocol. A
test fails while either file is not what the schema makes.

Usage: cargo xtask protocol
"""

import json
import re
import subprocess
import sys
import warnings
from pathlib import Path

from datamodel_code_generator import DataModelType, InputFileType, PythonVersion, generate

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "host" / "protocol.schema.json"
TYPES = ROOT / "src" / "furb" / "protocol.py"
BANNER = '''"""The types of the one protocol of the host, made from host/protocol.schema.json by xtask/python/protocol.py.

Run `cargo xtask protocol` after a change of the protocol.
"""
'''


VIEWS = {"view.mount": "Mount", "view.event": "ViewEvent", "view.dispose": "Dispose"}


def frame(body: str, kind: str) -> str:
  """The name of the class of the frame of a kind."""
  found = re.search(
    rf'class (Frame\d+)\(TypedDict[^)]*\):(?:\n(?!class ).*)*?\n +frame: Literal\[[\'"]{re.escape(kind)}[\'"]\]', body
  )
  assert found, f"the schema holds the frame {kind}"
  return found[1]


def generated() -> str:
  """The types of the protocol, as datamodel-code-generator makes them from the schema, formatted by ruff."""
  schema = json.loads(SCHEMA.read_text())
  with warnings.catch_warnings():
    # The generator warns of the numbered names of the variants of a union, which the types of TypeScript share.
    warnings.simplefilter("ignore")
    made = generate(
      json.dumps(schema),
      input_file_type=InputFileType.JsonSchema,
      output_model_type=DataModelType.TypingTypedDict,
      target_python_version=PythonVersion.PY_314,
      use_union_operator=True,
      use_standard_collections=True,
      disable_timestamp=True,
      use_schema_description=True,
      use_field_description=True,
      formatters=[],
    )
  assert isinstance(made, str), "the schema makes one module"
  # A closed TypedDict needs typing_extensions until Python 3.15, which no process of a skill should need to install:
  # a field that the schema does not name is refused by the daemon, not by these types.
  # A value that the schema leaves open is any JSON, which a process reads as an object and narrows.
  body = re.sub(r"\bAny\b", "object", made.split("\n\n", 1)[1].replace(", closed=True", ""))
  body = body.replace("from typing_extensions import TypedDict\n", "").replace("object, ", "TypedDict, ", 1)
  # The frames of a view keep a name of their own, as in TypeScript, since the numbers of a union move with the schema.
  aliases = "\n".join(f"type {name} = {frame(body, kind)}" for kind, name in VIEWS.items())
  made = f"{BANNER}\n{body.strip()}\n\n{aliases}\n\nPROTOCOL = {int(schema['x-protocol'])}\n"
  for ruff in (["check", "--fix", "--select", "I"], ["format"]):
    command = [sys.executable, "-m", "ruff", *ruff, "--stdin-filename", str(TYPES), "-"]
    made = subprocess.run(command, input=made, capture_output=True, text=True, check=True, cwd=ROOT).stdout  # noqa: S603
  return made


if __name__ == "__main__":
  TYPES.write_text(generated())
