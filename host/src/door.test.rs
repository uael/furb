use super::Door;

#[test]
fn each_door_is_the_word_that_a_model_writes_for_its_act() {
  let text = |one: &str| one.to_owned();
  let doors = [
    Door::Chain { label: text("bench") },
    Door::Fork { label: text("copy"), source: text("chain1"), kept: None },
    Door::Fork {
      label: text("rewind"),
      source: text("chain1"),
      kept: Some(vec![text("prompt1"), text("prompt3")]),
    },
    Door::Cd { path: text("~/src/it's") },
    Door::Stand { actor: text("claude-cli:sonnet/high") },
    Door::Edit { prompt: text("prompt2"), program: text("x = 1\nclose(\"done\")") },
    Door::Grant { usd: Some(2.0), share: None },
  ];
  let words: Vec<String> = doors.iter().map(Door::word).collect();
  assert_eq!(
    words,
    [
      "chain('bench')",
      "chain('copy', source='chain1')",
      "chain('rewind', source='chain1', filter=take('prompt1', 'prompt3'))",
      r#"cd("~/src/it's")"#,
      "actor = 'claude-cli:sonnet/high'",
      r#"write(Text('prompt2', 'x = 1\nclose("done")'))"#,
      "await skill('grant')\nrung('grant(usd=2.0, share=None)')",
    ]
  );
}
