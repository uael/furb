"""The contract of a build is compared with the contract of the last release, and each break is named."""

import importlib.util
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

SPEC = importlib.util.spec_from_file_location(
  "contract", Path(__file__).resolve().parents[2] / "script" / "contract.py"
)
assert SPEC and SPEC.loader
contract = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(contract)


@pytest.fixture(autouse=True)
def alone(monkeypatch: pytest.MonkeyPatch) -> None:
  """A hook of git sets GIT_DIR and GIT_INDEX_FILE, which would send each command of a test to the repository."""
  for name in [x for x in os.environ if x.startswith("GIT_")]:
    monkeypatch.delenv(name)


POLICY = (
  '"""A minor may change the default of each policy: LIMIT, SHOW. A change of any other constant is a major."""\n'
)


def kinds(old: str | None, new: str | None) -> list[str]:
  return [f"{x.level}: {x.what}" for x in contract.diff("c.pyi", old, new)]


def test_a_definition_that_goes_is_a_break() -> None:
  assert kinds("def a() -> int: ...\ndef b() -> int: ...\n", "def a() -> int: ...\n") == ["break: the def goes"]


def test_a_definition_that_is_more_is_a_minor() -> None:
  assert kinds("def a() -> int: ...\n", "def a() -> int: ...\ndef b() -> int: ...\n") == ["minor: the def is more"]


def test_a_parameter_that_goes_is_a_break() -> None:
  assert kinds("def a(x: int, y: int) -> int: ...\n", "def a(x: int) -> int: ...\n") == ["break: the parameter y goes"]


def test_a_type_that_changes_is_a_break() -> None:
  got = kinds("def a(x: int) -> int: ...\n", "def a(x: str) -> int: ...\n")
  assert got == ["break: the type of x changes from int to str"]
  assert kinds("def a() -> int: ...\n", "def a() -> str: ...\n") == [
    "break: its type of return changes from int to str"
  ]


def test_an_order_that_changes_is_a_break() -> None:
  got = kinds("def a(x: int, y: int) -> int: ...\n", "def a(y: int, x: int) -> int: ...\n")
  assert got == ["break: the order of the positional parameters changes from ['x', 'y'] to ['y', 'x']"]


def test_a_keyword_that_is_more_with_no_default_is_a_break() -> None:
  got = kinds("def a(x: int) -> int: ...\n", "def a(x: int, y: int) -> int: ...\n")
  assert got == ["break: the parameter y is more and has no default"]


def test_a_keyword_that_is_more_with_a_default_at_the_end_is_a_minor() -> None:
  got = kinds("def a(x: int) -> int: ...\n", "def a(x: int, y: int = 1) -> int: ...\n")
  assert got == ["minor: the parameter y is more, with the default 1"]
  got = kinds("def a(x: int) -> int: ...\n", "def a(x: int, *, y: int = 1) -> int: ...\n")
  assert got == ["minor: the parameter y is more, with the default 1"]


def test_a_keyword_with_a_default_before_a_parameter_that_a_caller_passes_by_position_is_a_break() -> None:
  got = kinds("def a(x: int, z: int) -> int: ...\n", "def a(x: int, y: int = 1, z: int = 2) -> int: ...\n")
  assert got == [
    "minor: the parameter z gains the default 2",
    "break: the parameter y stands before a parameter that a caller passes by position",
  ]


def test_a_default_that_changes_or_goes_is_a_break_and_a_default_that_a_keyword_gains_is_a_minor() -> None:
  assert kinds("def a(x: int = 1) -> int: ...\n", "def a(x: int = 2) -> int: ...\n") == [
    "break: the default of x changes from 1 to 2"
  ]
  assert kinds("def a(x: int = 1) -> int: ...\n", "def a(x: int) -> int: ...\n") == ["break: the default of x goes"]
  assert kinds("def a(x: int) -> int: ...\n", "def a(x: int = 1) -> int: ...\n") == [
    "minor: the parameter x gains the default 1"
  ]


def test_a_def_that_becomes_async_is_a_break() -> None:
  assert kinds("def a() -> int: ...\n", "async def a() -> int: ...\n") == ["break: it becomes async"]


def test_an_overload_that_goes_is_a_break_and_one_more_is_a_minor() -> None:
  one = "@overload\ndef a(x: int) -> int: ...\n"
  two = "@overload\ndef a(x: str) -> str: ...\n"
  assert kinds(one + two, one) == ["break: the overload (pos x: str) -> str goes or changes"]
  assert kinds(one, one + two) == ["minor: the overload (pos x: str) -> str is more"]


def test_the_value_of_a_constant_that_changes_is_a_break_unless_the_line_of_the_policy_names_it() -> None:
  old = POLICY + "LIMIT: int = 1\nOTHER: int = 1\n"
  new = POLICY + "LIMIT: int = 2\nOTHER: int = 2\n"
  assert kinds(old, new) == ["minor: the policy changes from 1 to 2", "break: its value changes from 1 to 2"]


def test_the_line_of_the_release_names_the_policies_and_one_pull_request_cannot_add_a_name_and_change_it() -> None:
  old = POLICY + "OTHER: int = 1\n"
  new = POLICY.replace("LIMIT, SHOW", "OTHER") + "OTHER: int = 2\n"
  assert kinds(old, new) == ["break: its value changes from 1 to 2"]
  assert kinds("OTHER: int = 1\n", new) == ["minor: the policy changes from 1 to 2"]


def test_a_constant_that_goes_or_changes_its_type_is_a_break() -> None:
  assert kinds("A: int = 1\nB: int = 2\n", "A: int = 1\n") == ["break: the const goes"]
  assert kinds("A: int = 1\n", "A: str = 1\n") == ["break: its type changes from int to str"]


def test_an_alias_that_changes_is_a_break() -> None:
  assert kinds("type A = int\n", "type A = int | str\n") == ["break: the alias changes from int to int | str"]


def test_a_field_a_method_and_a_base_of_a_class_that_go_are_a_break() -> None:
  old = "class A(Base):\n  x: int\n  def m(self) -> int: ...\n"
  new = "class A:\n  pass\n"
  assert kinds(old, new) == ["break: the base Base goes", "break: the field goes", "break: the def goes"]


def test_a_field_of_a_dataclass_with_no_default_is_a_break_and_with_a_default_is_a_minor() -> None:
  old = "@dataclass\nclass A:\n  x: int\n"
  assert kinds(old, old + "  y: int\n") == ["break: the field is more and has no default"]
  assert kinds(old, old + "  y: int = 1\n") == ["minor: the field is more"]
  assert kinds("class A:\n  x: int\n", "class A:\n  x: int\n  y: int\n") == ["minor: the field is more"]


def test_a_sentence_that_changes_is_a_wording_and_a_sentence_more_is_a_minor() -> None:
  old = 'def a() -> int:\n  """A one.\n  A two.\n  """\n'
  assert kinds(old, 'def a() -> int:\n  """A one.\n  A three.\n  """\n') == [
    "wording: a sentence changes or goes: A two.",
    "minor: a sentence is more: A three.",
  ]
  assert kinds(old, 'def a() -> int:\n  """A one.\n  A two.\n  A three.\n  """\n') == [
    "minor: a sentence is more: A three."
  ]


def test_the_sentence_of_a_constant_and_a_sentence_that_no_definition_holds_count_too() -> None:
  assert kinds('A: int = 1\n"""A is one."""\n', 'A: int = 1\n"""A is two."""\n') == [
    "wording: a sentence changes or goes: A is one.",
    "minor: a sentence is more: A is two.",
  ]
  assert kinds('A: int = 1\n"""A is one."""\n"""Free."""\n', 'A: int = 1\n"""A is one."""\n') == [
    "wording: a sentence changes or goes: Free."
  ]


def test_a_contract_that_is_more_is_a_minor_and_one_that_goes_is_a_break() -> None:
  assert kinds(None, "def a() -> int: ...\n") == ["minor: the contract is more"]
  assert kinds("def a() -> int: ...\n", None) == ["break: the contract goes"]
  assert kinds(None, None) == []


def git(root: Path, *args: str) -> None:
  subprocess.run(  # noqa: S603
    [
      shutil.which("git") or "git",
      "-c",
      "user.name=t",
      "-c",
      "user.email=t@t",
      "-c",
      "commit.gpgsign=false",
      "-c",
      "tag.gpgsign=false",
      *args,
    ],
    cwd=root,
    check=True,
    capture_output=True,
  )


def tagged(root: Path, text: str, tag: str = "v0.1.0") -> None:
  if not (root / ".git").exists():
    git(root, "init", "-q")
  engine = root / "src" / "furb" / "engine.pyi"
  engine.parent.mkdir(parents=True, exist_ok=True)
  engine.write_text(text, encoding="utf-8")
  git(root, "add", "-A")
  git(root, "commit", "-q", "-m", tag)
  git(root, "tag", tag)


def edit(root: Path, text: str) -> None:
  (root / "src" / "furb" / "engine.pyi").write_text(text, encoding="utf-8")


def test_a_run_with_no_release_finds_no_break(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
  assert contract.main([], root=tmp_path) == 0
  assert "No release to compare with" in capsys.readouterr().out


def test_a_break_fails_unless_the_title_holds_a_bang(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
  tagged(tmp_path, "def a(x: int) -> int: ...\n")
  edit(tmp_path, "def a() -> int: ...\n")
  assert contract.main([], root=tmp_path) == 1
  assert contract.main(["--title", "feat(engine): Drop a parameter"], root=tmp_path) == 1
  assert contract.main(["--title", "feat(engine)!: Drop a parameter"], root=tmp_path) == 0
  out = capsys.readouterr().out
  assert "break: src/furb/engine.pyi a: the parameter x goes" in out
  assert "Against v0.1.0: 1 breaks, 0 sentences that change, 0 minors." in out
  assert "The title holds !, so it declares 1 breaks." in out


def test_a_sentence_that_changes_passes_with_the_label_wording(tmp_path: Path) -> None:
  tagged(tmp_path, 'def a() -> int:\n  """A one."""\n')
  edit(tmp_path, 'def a() -> int:\n  """A two."""\n')
  assert contract.main(["--title", "fix(engine): Say it better"], root=tmp_path) == 1
  assert contract.main(["--title", "fix(engine): Say it better", "--wording"], root=tmp_path) == 0


def test_a_minor_passes_and_the_last_release_is_the_one_that_it_compares_with(tmp_path: Path) -> None:
  tagged(tmp_path, "def a() -> int: ...\ndef b() -> int: ...\n")
  tagged(tmp_path, "def a() -> int: ...\n", "v0.2.0")
  edit(tmp_path, "def a() -> int: ...\ndef c() -> int: ...\n")
  assert contract.main([], root=tmp_path) == 0
  assert contract.main(["--base", "v0.1.0"], root=tmp_path) == 1


def test_a_kit_that_goes_since_the_release_is_a_break_and_a_kit_that_is_more_is_a_minor(
  tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
  tagged(tmp_path, "def a() -> int: ...\n")
  kit = tmp_path / "extensions" / "web" / "web.pyi"
  kit.parent.mkdir(parents=True)
  kit.write_text("def web_fetch(url: str) -> int: ...\n", encoding="utf-8")
  git(tmp_path, "add", "-A")
  git(tmp_path, "commit", "-q", "-m", "kit")
  git(tmp_path, "tag", "v0.2.0")
  kit.unlink()
  new = tmp_path / ".agents" / "skills" / "mcp" / "mcp.pyi"
  new.parent.mkdir(parents=True)
  new.write_text("def mcp_call() -> int: ...\n", encoding="utf-8")
  assert contract.main([], root=tmp_path) == 1
  out = capsys.readouterr().out
  assert "break: .agents/skills/web/web.pyi: the contract goes" in out
  assert "minor: .agents/skills/mcp/mcp.pyi: the contract is more" in out


def test_a_kit_that_moved_to_the_skills_with_the_same_interface_is_no_change(
  tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
  tagged(tmp_path, "def a() -> int: ...\n")
  kit = tmp_path / "extensions" / "web" / "web.pyi"
  kit.parent.mkdir(parents=True)
  kit.write_text("def web_fetch(url: str) -> int: ...\n", encoding="utf-8")
  git(tmp_path, "add", "-A")
  git(tmp_path, "commit", "-q", "-m", "kit")
  git(tmp_path, "tag", "v0.2.0")
  moved = tmp_path / ".agents" / "skills" / "web" / "web.pyi"
  moved.parent.mkdir(parents=True)
  kit.rename(moved)
  assert contract.main([], root=tmp_path) == 0
  assert ".agents/skills/web/web.pyi" not in capsys.readouterr().out


def test_a_break_that_the_base_holds_already_fails_no_later_pull_request(
  tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
  tagged(tmp_path, "def a(x: int) -> int: ...\ndef b(y: int) -> int: ...\n")
  edit(tmp_path, "def a() -> int: ...\ndef b(y: int) -> int: ...\n")
  git(tmp_path, "add", "-A")
  git(tmp_path, "commit", "-q", "-m", "base")
  assert contract.main([], root=tmp_path) == 1
  assert contract.main(["--since", "HEAD"], root=tmp_path) == 0
  edit(tmp_path, "def a() -> int: ...\ndef b() -> int: ...\n")
  capsys.readouterr()
  assert contract.main(["--since", "HEAD"], root=tmp_path) == 1
  out = capsys.readouterr().out
  assert "a: the parameter x goes" not in out
  assert "break: src/furb/engine.pyi b: the parameter y goes" in out
  assert "1 changes already stand on HEAD." in out


def schema(given: dict[str, list[str]]) -> str:
  """A schema of the protocol whose Node is the nodes given, each with its own fields, beside the id of every node."""
  one_of = [{"properties": {"node": {"const": name}, **{f: {} for f in fields}}} for name, fields in given.items()]
  return json.dumps({"definitions": {"Node": {"properties": {"id": {}}, "oneOf": one_of}}})


def nodes_of(old: dict[str, list[str]], new: dict[str, list[str]]) -> list[str]:
  return [f"{x.level}: {x.what}" for x in contract.node_diff(contract.SCHEMA, schema(old), schema(new))]


def test_a_node_of_a_view_that_goes_is_a_break() -> None:
  assert nodes_of({"text": ["runs"], "code": ["text"]}, {"text": ["runs"]}) == ["break: the node code goes"]


def test_a_field_of_a_node_that_goes_is_a_break() -> None:
  assert nodes_of({"code": ["text", "language"]}, {"code": ["text"]}) == [
    "break: the field language of the node code goes"
  ]
  assert (
    contract.node_diff(
      contract.SCHEMA,
      schema({"code": ["text"]}),
      json.dumps({"definitions": {"Node": {"oneOf": [{"properties": {"node": {"const": "code"}, "text": {}}}]}}}),
    )[0].what
    == "the field id of the node code goes"
  )


def test_a_node_or_a_field_more_is_a_minor() -> None:
  assert nodes_of({"text": ["runs"]}, {"text": ["runs", "wrap"], "link": ["url"]}) == [
    "minor: the node link is more",
    "minor: the field wrap of the node text is more",
  ]


def test_the_schema_of_the_protocol_holds_the_closed_set_of_the_nodes_of_a_view() -> None:
  held = contract.nodes((Path(__file__).resolve().parents[2] / contract.SCHEMA).read_text(encoding="utf-8"))
  assert sorted(held) == sorted(
    ["box", "text", "list", "table", "input", "button", "progress", "code", "diff", "markdown"]
  )
  assert all("id" in fields for fields in held.values())


def test_the_contract_holds_the_schema_of_the_protocol(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
  tagged(tmp_path, "def a() -> int: ...\n")
  (tmp_path / "host").mkdir()
  (tmp_path / contract.SCHEMA).write_text(schema({"text": ["runs"], "code": ["text"]}), encoding="utf-8")
  git(tmp_path, "add", "-A")
  git(tmp_path, "commit", "-q", "-m", "schema")
  git(tmp_path, "tag", "v0.3.0")
  (tmp_path / contract.SCHEMA).write_text(schema({"text": ["runs"]}), encoding="utf-8")
  assert contract.main([], root=tmp_path) == 1
  assert "break: host/protocol.schema.json: the node code goes" in capsys.readouterr().out
