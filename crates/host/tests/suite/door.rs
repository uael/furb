//! The door of a person to a chain: the word that a model would write for each act of his.

use furb_host::Door;

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
    Door::Stand,
    Door::Actor { actor: text("claude-cli:sonnet/high") },
    Door::Bash { command: text("ls"), stdin: Some(String::new()) },
    Door::Bash { command: text("cat"), stdin: Some(text("hi\n")) },
    Door::Bash { command: text("cat"), stdin: None },
    Door::Read { path: text("a.txt") },
    Door::Write { path: text("prompt2"), content: text("x = 1\nclose(\"done\")") },
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
      "stand()",
      "actor = 'claude-cli:sonnet/high'",
      "bash('ls')",
      "bash('cat', stdin='hi\\n')",
      "bash('cat', stdin=None)",
      "read('a.txt')",
      r#"write(Text('prompt2', 'x = 1\nclose("done")'))"#,
      "await skill('grant')\nrung('grant(usd=2.0, share=None)')",
    ]
  );
}
