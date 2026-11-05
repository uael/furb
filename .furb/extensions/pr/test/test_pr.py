"""pr fills the template of the worktree and opens or edits the pull request with gh, on the real engine."""

import pytest

from conftest import said
from extensions.conftest import Bench
from furb.engine import Refused

CHECKS = ["Mine.", "One change.", "No comment.", "Plain.", "None known.", "Checked."]


def body(made: Bench) -> str:
  return str([x for x in said(made.log, "bash") if "pr create" in str(x[4]) or "pr edit" in str(x[4])][-1][5])


async def test_pr_checks_with_prcheck_and_opens_or_edits_the_pr_with_the_template_filled(pulling: Bench) -> None:
  """pr(title, why, outcome, checks) checks them with prcheck, and opens the pull request of the branch of the worktree at where, or edits the one that stands, with the template of that worktree filled as its body."""
  await pulling.value(f'await pr("chore(pr): Open it", "Why.", ["One."], {CHECKS!r})')
  assert [x for x in pulling.gh() if x.startswith("pr create")] == ["pr create --base main --head  --title chore(pr): Open it --body-file -"]
  await pulling.value(f'await pr("chore(pr): Edit it", "Why.", ["One."], {CHECKS!r}, number=7)')
  assert "pr edit 7 --title chore(pr): Edit it --body-file -" in pulling.gh()
  assert body(pulling).startswith("🤖\n\n## Why\n\nWhy.\n\n## Outcome\n\n- One.\n\n---\n\n- [x] I am accountable")
  with pytest.raises(Refused, match="The title is wrong"):
    await pulling.value(f'await pr("docs: No area", "Why.", ["One."], {CHECKS!r})')
  assert len([x for x in pulling.gh() if x.startswith("pr ")]) == 4


async def test_pr_writes_the_head_line_and_the_sections_and_no_comment(pulling: Bench) -> None:
  """The body starts with a line of 🤖, Fixes #fixes and Part of #part_of, fills Why and Outcome, fills Breaking changes or deletes it, and holds no comment of the template."""
  await pulling.value(f'await pr("feat(pr)!: Break it", "Why.", ["One.", "Two."], {CHECKS!r}, breaking=["The door moves."], fixes=3, part_of=2)')
  made = body(pulling)
  assert made.startswith("🤖 Fixes #3. Part of #2.\n\n## Why\n\nWhy.\n\n## Outcome\n\n- One.\n- Two.\n\n## Breaking changes\n\n- The door moves.\n\n---")
  assert "<!--" not in made and "-->" not in made
  await pulling.value(f'await pr("feat(pr): Keep it", "Why.", ["One."], {CHECKS!r})')
  assert "## Breaking changes" not in body(pulling)


async def test_pr_ticks_each_box_and_leaves_a_not_open(pulling: Bench) -> None:
  """It ticks each box of the checklist from checks in order, and a check that starts with not: leaves its box open under a line Not yet: with what is left."""
  checks = [*CHECKS[:2], "not: the README needs a second pass", *CHECKS[3:]]
  await pulling.value(f'await pr("chore(pr): Tick it", "Why.", ["One."], {checks!r})')
  boxes = body(pulling).split("---\n\n", 1)[1].splitlines()
  assert [x[:6] for x in boxes if x.startswith("- [")] == ["- [x] ", "- [x] ", "- [ ] ", "- [x] ", "- [x] ", "- [x] "]
  assert boxes[3] == "  Not yet: the README needs a second pass"


async def test_pr_refuses_checks_that_do_not_fit_the_checklist(pulling: Bench) -> None:
  """It refuses checks that do not give one line to each box of the checklist."""
  for checks in (CHECKS[:5], [*CHECKS[:5], "not:"]):
    with pytest.raises(Refused, match="checks needs 6 lines, one for each box of the checklist"):
      await pulling.value(f'await pr("chore(pr): Tick it", "Why.", ["One."], {checks!r})')
  assert not [x for x in pulling.gh() if x.startswith("pr ")]


async def test_pr_gives_each_label_a_flag_and_a_break_its_label(pulling: Bench) -> None:
  """It gives each label one flag after the body, and a breaking change the label breaking-change."""
  await pulling.value(f'await pr("feat(pr)!: Label it", "Why.", ["One."], {CHECKS!r}, breaking=["It moves."], labels="--label w:x")')
  assert [x for x in pulling.gh() if x.startswith("pr create")][-1].endswith("--body-file - --label w:x --label breaking-change")
  assert {"label create w:x --force", "label create breaking-change --force"} <= set(pulling.gh())
  await pulling.value(f'await pr("chore(pr): Label it", "Why.", ["One."], {CHECKS!r}, labels=["w:y", "it\'s"])')
  assert [x for x in pulling.gh() if x.startswith("pr create")][-1].endswith("--body-file - --label w:y --label it's")
