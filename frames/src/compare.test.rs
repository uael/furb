use super::*;

const BEST: &str = "| # | Scenario | 1st | 2nd | furb |
| --- | --- | --- | --- | --- |
| 05 | prompt-box | OpenCode `d` | Claude Code `c` | near |

## 05 prompt-box

| Aspect | Claude Code | Codex | OpenCode | Pi | furb |
| --- | --- | --- | --- | --- | --- |
| 1 hierarchy | 7: rules | 6: bare | 8: a bar | 6: rules | 8: the hue bar |

## 06 prompt-history
";

#[test]
fn the_winners_and_their_scores_come_from_the_judgment() {
  let found = winners(BEST, "05-prompt-box");
  let each: Vec<(&str, usize, &str)> =
    found.iter().map(|one| (one.folder, one.rank, one.step.as_str())).collect();
  assert_eq!(each, [("opencode", 1, "d"), ("claude", 2, "c")]);
  assert_eq!(
    scores(BEST, "05-prompt-box", &found),
    "\nscores:\n- 1 hierarchy: OpenCode 8: a bar | Claude Code 7: rules | furb 8: the hue bar\n"
  );
}

#[test]
fn frames_stand_side_by_side_each_in_its_width() {
  let (one, two) = (Column::of("a".to_owned(), "日x"), Column::of("b".to_owned(), "y\nz"));
  assert_eq!((one.width, two.width), (3, 1));
  assert_eq!(beside(&[&one, &two]), "a   │ b\n日x │ y\n    │ z\n");
}
