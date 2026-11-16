//! The reader of a record, through what the crate gives a host: a record written as a life keeps
//! it, and what [`retold`] and its kin tell of it, with no life.

use std::{fs, path::PathBuf, process, thread};

use serde_json::{Value, value::RawValue};

use crate::{view::named, yard::Yard};
use furb::{
  value::Object,
  wire::parsed,
  world::{retold, retold_live, retold_with},
};

/// The facts of lines of JSON, as a record keeps them.
fn facts(lines: &[&str]) -> Vec<Object> {
  lines.iter().map(|one| parsed(one).expect("a fact")).collect()
}

/// Each fact as python shows it.
fn shown(facts: &[Object]) -> Vec<String> {
  facts.iter().map(|one| one.as_ref().py_repr()).collect()
}

/// A record of its own for one test, which does not stand yet.
fn place(name: &str) -> Yard {
  Yard::new(format!("furb-reader-{name}-{}", process::id())).within("record.jsonl")
}

/// The facts that a record that kept these facts tells, as python shows them.
fn told(name: &str, kept: &[&str]) -> Vec<String> {
  let at = place(name);
  fs::write(&at, kept.iter().flat_map(|one| ["[", one, "]\n"]).collect::<String>())
    .expect("the record is written");
  shown(&retold(&at).expect("the record is told").facts().expect("its facts"))
}

/// The opening of a life: its root chain, a prompt of the operator on it, and the reply that rung1
/// asked for.
const OPENS: [&str; 3] = [
  r#"["chain", "chain1", "operator", "", "root", ""]"#,
  r#"["prompt", "prompt1", "operator", "chain1", "str", "Go.", ""]"#,
  r#"["reply", "reply1", "rung1", "chain1", "opus"]"#,
];

/// The done of reply1 with a word, as a record keeps a tuple.
fn answered(word: &str) -> String {
  format!(
    r#"["done", "reply1", "provider", {{"is": "tuple", "args": [["assistant", "{word}", [1, 2, 3, 4, 0.5], null]]}}]"#
  )
}

#[test]
fn a_kept_rung_takes_its_place_on_the_open_prompt_with_the_run_of_the_word_that_the_gate_let_run() {
  let done = answered("x = 1");
  let kept = [
    OPENS[0],
    OPENS[1],
    OPENS[2],
    done.as_str(),
    r#"["gate", "gate1", "chain1", "chain1", "x = 1"]"#,
    r#"["done", "gate1", "gate", []]"#,
    r#"["bash", "bash1", "rung1", "chain1", "ls", "", 600]"#,
    r#"["reply", "reply2", "rung2", "chain1", "opus"]"#,
  ];
  let told = told("told1", &kept);
  assert_eq!(told[2], "['rung', 'rung1', 'prompt1', 'chain1', '', '', '']");
  assert_eq!(told[3], "['reply', 'reply1', 'rung1', 'chain1', 'opus']");
  assert_eq!(
    told[7..],
    [
      "['run', 'run1', 'chain1', 'chain1', 'rung1', 'x = 1', '']",
      "['bash', 'bash1', 'rung1', 'chain1', 'ls', '', 600]",
      "['done', 'run1', 'reader', None]",
      "['done', 'rung1', 'reader', None]",
      "['rung', 'rung2', 'prompt1', 'chain1', '', '', '']",
      "['reply', 'reply2', 'rung2', 'chain1', 'opus']",
    ]
    .map(str::to_owned)
  );
}

#[test]
fn a_chain_that_the_record_does_not_hold_is_made_by_the_rung_of_its_first_prompt() {
  let kept = [
    OPENS[0],
    OPENS[1],
    OPENS[2],
    r#"["prompt", "prompt2", "rung1", "chain2", "str", "Help.", ""]"#,
  ];
  assert_eq!(
    told("told2", &kept)[4..],
    [
      "['chain', 'chain2', 'rung1', 'chain1', 'Help.', '']",
      "['prompt', 'prompt2', 'rung1', 'chain2', 'str', 'Help.', '']",
    ]
    .map(str::to_owned)
  );
}

#[test]
fn a_chain_that_the_record_does_not_hold_takes_the_first_line_of_its_first_prompt_as_its_label() {
  let long = format!("\\n  {}  \\nSecond line.", "a".repeat(100));
  let first = format!(r#"["prompt", "prompt2", "operator", "chain2", "str", "{long}", ""]"#);
  let kept = [
    OPENS[0],
    first.as_str(),
    r#"["prompt", "prompt3", "operator", "chain2", "str", "Later.", ""]"#,
  ];
  let told = told("told3", &kept);
  assert_eq!(told[1], format!("['chain', 'chain2', 'operator', '', '{}…', '']", "a".repeat(80)));
}

#[test]
fn a_chain_that_the_record_does_not_hold_and_that_no_prompt_names_takes_its_own_name_as_its_label()
{
  let kept = [OPENS[0], r#"["bash", "bash1", "rung3", "chain2", "ls", "", 600]"#];
  assert_eq!(told("told4", &kept)[1], "['chain', 'chain2', 'operator', '', 'chain2', '']");
}

#[test]
fn a_chain_that_the_record_does_not_hold_stands_under_the_chain_that_its_maker_ran_on() {
  let kept = [
    OPENS[0],
    r#"["prompt", "prompt2", "rung5", "chain2", "str", "Help.", ""]"#,
    r#"["bash", "bash1", "rung5", "chain3", "ls", "", 600]"#,
  ];
  assert_eq!(told("told5", &kept)[1], "['chain', 'chain2', 'rung5', 'chain3', 'Help.', '']");
}

#[test]
fn a_word_that_the_gate_refused_tells_its_findings_and_runs_not() {
  let done = answered("BAD");
  let kept = [
    OPENS[0],
    OPENS[1],
    OPENS[2],
    done.as_str(),
    r#"["gate", "gate1", "chain1", "chain1", "BAD"]"#,
    r#"["done", "gate1", "gate", ["line 1: BAD is unbound"]]"#,
  ];
  assert_eq!(
    told("told6", &kept)[7..],
    [
      "['tell', 'rung1', 'reader', [\"#rung1 refused\\nrung1_findings = 'line 1: BAD is unbound'\"]]",
      "['done', 'rung1', 'reader', {'is': 'Refused', 'args': []}]",
    ]
    .map(str::to_owned)
  );
}

#[test]
fn a_prompt_that_the_operator_closed_is_done_with_its_value_and_ends_its_rung() {
  let kept = [
    OPENS[0],
    OPENS[1],
    OPENS[2],
    r##"["close", "prompt1", "console", "done", ["#prompt1 closed"]]"##,
    r#"["cancel", "chain1", "operator"]"#,
  ];
  assert_eq!(
    told("told7", &kept)[4..],
    [
      "['close', 'prompt1', 'console', 'done', ['#prompt1 closed']]",
      "['done', 'prompt1', 'reader', 'done']",
      "['done', 'rung1', 'reader', None]",
      "['cancel', 'chain1', 'operator']",
      "['done', 'chain1', 'reader', {'is': 'CancelledError', 'args': []}]",
    ]
    .map(str::to_owned)
  );
}

#[test]
fn a_reader_of_a_life_that_runs_reads_the_log_that_agrees_as_far_as_its_last_checkpoint() {
  // The record took its entry 4 and the log did not take the line of it yet, as in a run in flight.
  let at = place("live");
  let record = [
    r#"[["made","chain1","store","abc"]]"#,
    r#"[["stand","stand1","operator","chain1"]]"#,
    r#"[["checkpoint","chain1","store",{}]]"#,
    r#"[["cd","cd1","operator","chain1","/tmp"]]"#,
  ];
  fs::write(&at, record.join("\n") + "\n").unwrap();
  let log = format!("{}.log", at.display());
  let lines = [
    r#"{"entries":1}"#,
    r#"["stand","stand1","operator","chain1"]"#,
    r#"["keep","",2]"#,
    r##"["tell","chain1","chain1",["#chain1"]]"##,
    r#"["keep","",3]"#,
  ];
  fs::write(&log, lines.join("\n") + "\n").unwrap();
  assert!(!retold(&at).unwrap().logged, "the whole log stands behind the whole record");
  let read = retold_live(&at).unwrap();
  assert_eq!((read.logged, &read.file, read.entries), (true, &PathBuf::from(&log), 4));
  assert_eq!(
    shown(&read.facts().unwrap()),
    ["['stand', 'stand1', 'operator', 'chain1']", "['tell', 'chain1', 'chain1', ['#chain1']]"]
      .map(str::to_owned)
  );
  // A log that counts no checkpoint of the record agrees with nothing that stands still.
  fs::write(&log, lines[..3].join("\n") + "\n").unwrap();
  assert!(!retold_live(&at).unwrap().logged);
  // A log that lacks a question that the record asked before its checkpoint is no log of it.
  fs::write(&log, [lines[0], lines[2], lines[4]].join("\n") + "\n").unwrap();
  assert!(!retold_live(&at).unwrap().logged);
}

#[test]
fn a_record_names_its_engine_and_reads_its_log_while_the_log_agrees() {
  let at = place("retold");
  fs::write(
    &at,
    "[[\"made\",\"chain1\",\"store\",\"abc\"]]\n[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n",
  )
  .unwrap();
  let read = retold(&at).unwrap();
  assert_eq!(read.made.as_deref(), Some("abc"));
  assert!(!read.logged);
  assert_eq!(
    shown(&read.facts().unwrap()),
    [
      "['made', 'chain1', 'store', 'abc']",
      "['chain', 'chain1', 'operator', '', 'chain1', '']",
      "['stand', 'stand1', 'operator', 'chain1']",
    ]
    .map(str::to_owned)
  );
  fs::write(
    format!("{}.log", at.display()),
    "{\"entries\":2}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n[\"tell\",\"chain1\",\"chain1\",[\"#chain1\"]]\n",
  )
  .unwrap();
  let read = retold(&at).unwrap();
  assert!(read.logged);
  assert_eq!(read.file, PathBuf::from(format!("{}.log", at.display())));
  assert_eq!(
    shown(&read.facts().unwrap()),
    ["['stand', 'stand1', 'operator', 'chain1']", "['tell', 'chain1', 'chain1', ['#chain1']]"]
      .map(str::to_owned)
  );
}

#[test]
fn a_log_agrees_with_a_record_that_ends_with_a_checkpoint_that_no_life_wrote() {
  let at = place("sealed");
  let stand = "[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n";
  let checkpoint = "[[\"checkpoint\",\"chain1\",\"store\",{}]]\n";
  let log = "{\"entries\":0}\n[\"stand\",\"stand1\",\"operator\",\"chain1\"]\n[\"keep\",\"\",1]\n";
  fs::write(format!("{}.log", at.display()), log).unwrap();
  // The record of a life, then the checkpoints that a build of the major 2 of the schema kept
  // after it: the log holds no line of keep for them, and it holds each fact of the record.
  fs::write(&at, stand).unwrap();
  assert!(retold(&at).unwrap().logged);
  fs::write(&at, format!("{stand}{checkpoint}{checkpoint}")).unwrap();
  let read = retold(&at).unwrap();
  assert!(read.logged, "a checkpoint is no fact, so the log lacks none");
  assert_eq!(
    shown(&read.facts().unwrap()),
    ["['stand', 'stand1', 'operator', 'chain1']".to_owned()]
  );
  // An entry of another kind that the log does not count is a fact that the log lacks.
  fs::write(&at, format!("{stand}{checkpoint}{stand}")).unwrap();
  assert!(!retold(&at).unwrap().logged);
}

#[test]
fn a_kept_rung_with_a_word_runs_it_once_the_gate_lets_it_run() {
  let kept = [
    OPENS[0],
    r#"["rung", "rung1", "operator", "chain1", "x = 1", "", ""]"#,
    r#"["gate", "gate1", "chain1", "chain1", "x = 1"]"#,
    r#"["done", "gate1", "gate", []]"#,
  ];
  assert_eq!(told("told11", &kept)[4], "['run', 'run1', 'chain1', 'chain1', 'rung1', 'x = 1', '']");
}

#[test]
fn a_rung_takes_no_place_on_a_prompt_that_it_made() {
  let kept = [
    OPENS[0],
    r#"["prompt", "prompt1", "rung1", "chain1", "str", "Ask.", ""]"#,
    r#"["bash", "bash1", "rung1", "chain1", "ls", "", 600]"#,
  ];
  assert_eq!(told("told12", &kept)[2], "['rung', 'rung1', 'chain1', 'chain1', '', '', '']");
}

#[test]
fn a_second_read_of_a_record_that_did_not_change_takes_the_file_of_what_it_tells_as_it_stands() {
  let at = place("cached");
  fs::write(&at, "[[\"made\",\"chain1\",\"store\",\"abc\"]]\n").unwrap();
  let read = retold(&at).unwrap();
  let told = PathBuf::from(format!("{}.told", at.display()));
  assert_eq!(read.file, told);
  let head = fs::read_to_string(&told).unwrap().lines().next().unwrap().to_owned();
  fs::write(&told, format!("{head}\n[\"tell\",\"chain1\",\"chain1\",[\"kept\"]]\n")).unwrap();
  let again = retold(&at).unwrap();
  assert_eq!(again.made.as_deref(), Some("abc"));
  assert_eq!((again.entries, &again.last), (read.entries, &read.last));
  assert_eq!(again.entries, 1);
  assert!(again.last.is_some());
  assert_eq!(shown(&again.facts().unwrap()), ["['tell', 'chain1', 'chain1', ['kept']]"]);
  fs::write(&at, "[[\"made\",\"chain1\",\"store\",\"abd\"]]\n").unwrap();
  let changed = retold(&at).unwrap();
  assert_eq!(changed.made.as_deref(), Some("abd"));
  assert_eq!(shown(&changed.facts().unwrap()), ["['made', 'chain1', 'store', 'abd']"]);
}

#[test]
fn a_record_that_keeps_a_tuple_and_a_whole_float_tells_them_as_the_engine_gives_them() {
  let at = place("plain");
  let done = answered("x = 1").replace("0.5]", r#"{"is": "float", "args": ["2"]}]"#);
  fs::write(
    &at,
    OPENS.iter().chain([&done.as_str()]).flat_map(|one| ["[", *one, "]\n"]).collect::<String>(),
  )
  .unwrap();
  let read = retold(&at).unwrap();
  let text = fs::read_to_string(&read.file).unwrap();
  let line = text.lines().find(|one| one.starts_with(r#"["done","reply1""#)).unwrap();
  assert_eq!(line, r#"["done","reply1","provider",["assistant","x = 1",[1,2,3,4,2.0],null]]"#);
}

#[test]
fn a_segment_that_keeps_each_question_gets_no_chain_rung_or_run_of_the_reader_but_the_dones_of_the_engine()
 {
  let done = answered("x = 1");
  let kept = [
    r#"["made", "chain1", "store", "e", null, "1.1", "0.0.0", "", ""]"#,
    OPENS[0],
    OPENS[1],
    r#"["rung", "rung1", "prompt1", "chain1", "", "", ""]"#,
    OPENS[2],
    done.as_str(),
    r#"["gate", "gate1", "chain1", "chain1", "x = 1"]"#,
    r#"["done", "gate1", "gate", []]"#,
    r#"["run", "run1", "chain1", "chain1", "rung1", "x = 1", ""]"#,
    r#"["chain", "chain2", "rung1", "chain1", "more", ""]"#,
    r#"["prompt", "prompt2", "rung1", "chain2", "str", "Help.", ""]"#,
    r#"["close", "prompt1", "operator", 1]"#,
  ];
  let told = told("told15", &kept);
  // The reader tells the turn of reply1 as JSON holds a tuple.
  let mut plain = kept;
  plain[5] = r#"["done", "reply1", "provider", ["assistant", "x = 1", [1, 2, 3, 4, 0.5], null]]"#;
  assert_eq!(told[..kept.len()], shown(&facts(&plain)), "the reader says no question of its own");
  assert_eq!(
    told[kept.len()..],
    [
      "['done', 'prompt1', 'reader', 1]",
      "['done', 'run1', 'reader', None]",
      "['done', 'rung1', 'reader', None]"
    ]
    .map(str::to_owned)
  );
}

/// A rung of the operator on the root, whose word names the session, as a record keeps it.
fn renamed(rung: &str, name: &str) -> String {
  let word = Value::String(named::word(name));
  format!(r#"[["rung", "{rung}", "operator", "chain1", {word}, "", ""]]"#)
}

#[test]
fn a_record_with_no_log_names_its_session_by_the_last_word_of_the_operator_that_names_it() {
  let at = place("said");
  let lines = [OPENS[0].to_owned()].map(|one| format!("[{one}]"));
  let kept = [&lines[0], &renamed("rung7", "First"), &renamed("rung8", r#"say "it" \ twice"#)];
  fs::write(&at, kept.map(|one| format!("{one}\n")).concat()).unwrap();
  let read = retold(&at).unwrap();
  assert!(!read.logged);
  assert_eq!((read.said.as_deref(), read.given.as_deref()), (Some(r#"say "it" \ twice"#), None));
  assert_eq!(read.name(), Some(r#"say "it" \ twice"#));
  let again = retold(&at).unwrap();
  assert_eq!(again.said, read.said);
  assert_ne!(fs::read_to_string(&again.file).unwrap(), "");
  assert!(!PathBuf::from(format!("{}.told.body", at.display())).exists());
}

#[test]
fn a_record_whose_log_agrees_names_its_session_by_the_dones_on_its_root_where_the_rename_wins() {
  let at = place("named");
  fs::write(
    &at,
    "[[\"made\",\"chain1\",\"store\",\"abc\"]]\n[[\"stand\",\"stand1\",\"operator\",\"chain1\"]]\n",
  )
  .unwrap();
  let log = [
    r#"{"entries":2}"#,
    r#"["stand","stand1","operator","chain1"]"#,
    r#"["name_session","name_session1","run1","chain1"]"#,
    r#"["done","name_session1","run1","Given"]"#,
    r#"["rename_session","rename_session1","run2","chain2"]"#,
    r#"["done","rename_session1","run2","Off the root"]"#,
  ];
  fs::write(format!("{}.log", at.display()), log.map(|one| format!("{one}\n")).concat()).unwrap();
  let read = retold(&at).unwrap();
  assert!(read.logged);
  assert_eq!((read.name(), read.said.as_deref()), (Some("Given"), None));
  let more = [
    r#"["rename_session","rename_session2","run3","chain1"]"#,
    r#"["done","rename_session2","run3","Mine"]"#,
  ];
  let text: String = log.iter().chain(&more).flat_map(|one| [*one, "\n"]).collect();
  fs::write(format!("{}.log", at.display()), text).unwrap();
  let read = retold(&at).unwrap();
  assert_eq!((read.name(), read.given.as_deref()), (Some("Mine"), Some("Given")));
}

/// A checkpoint is told as the JSON of its text, which must be the JSON of the value that the
/// record parses it into, as a fact of another kind with the same words is told: each checkpoint of
/// the records of the corpus, and facts that hold each mark of the record, are told the same both
/// ways.
#[test]
fn a_checkpoint_is_told_as_the_same_json_from_its_text_as_from_its_value() {
  let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("corpus");
  let mut texts: Vec<String> = [
    r#"["checkpoint","chain1","store",[1,{"is":"tuple","args":[[2,{"is":"tuple","args":[[]]}]]}]]"#,
    r#"["checkpoint","chain1","store",{"is":"float","args":["3"]},{"is":"float","args":["inf"]}]"#,
    r#"["checkpoint","chain1","store",{"is":"float","args":["NaN"]},{"is":"float","args":["-0"]},2.5]"#,
    r#"["checkpoint","chain1","store",{"is":"int","args":["123456789012345678901234567890"]}]"#,
    r#"["checkpoint","chain1","store",{"is":"dict","args":[[[{"is":"tuple","args":[[1,2]]},"a"],[3,"b"]]]}]"#,
    r#"["checkpoint","chain1","store",{"is":"Text","path":"a.txt","content":"x"},{"is":"Refused","args":["no"]}]"#,
    r#"["checkpoint","chain1","store",{"is":"tuple","args":[1]},{"is":"tuple","args":[[1]],"more":2}]"#,
    r#"["checkpoint","chain1","store",{"args":[[1]],"other":{"is":"tuple","args":[[{"is":"float","args":["1"]}]]}}]"#,
  ]
  .map(str::to_owned)
  .to_vec();
  for folder in
    fs::read_dir(corpus).unwrap().map(|one| one.unwrap().path()).filter(|one| one.is_dir())
  {
    for record in fs::read_dir(folder).unwrap().map(|one| one.unwrap().path()) {
      if record.extension().is_some_and(|named| named == "jsonl") {
        let text = fs::read_to_string(&record).unwrap();
        let held = text.lines().filter(|one| one.starts_with(r#"[["checkpoint","#));
        texts.extend(
          held.map(|one| serde_json::from_str::<Vec<&RawValue>>(one).unwrap()[0].get().to_owned()),
        );
      }
    }
  }
  assert!(texts.len() > 10, "the corpus holds checkpoints");
  let at = place("checkpoints");
  let twin = |text: &str| text.replacen(r#"["checkpoint","#, r#"["note","#, 1);
  let record: String = texts
    .iter()
    .flat_map(|text| ["[", text, "]\n[", &twin(text), "]\n"].map(str::to_owned))
    .collect();
  fs::write(&at, record).unwrap();
  let read = retold(&at).unwrap();
  let mut words = Vec::new();
  read
    .each(&mut |line| {
      let mut fact: Vec<Value> = serde_json::from_str(line).unwrap();
      if fact[0] == "checkpoint" || fact[0] == "note" {
        words.push((fact[0].clone(), fact.split_off(1)));
      }
    })
    .unwrap();
  assert_eq!(words.len(), 2 * texts.len());
  for (pair, text) in words.chunks(2).zip(&texts) {
    assert_eq!((&pair[0].0, &pair[1].0), (&Value::from("checkpoint"), &Value::from("note")));
    assert_eq!(pair[0].1, pair[1].1, "{text}");
  }
}

/// Two readers that read one record at once tell it once: the second waits for the first, then
/// takes the file that the first wrote, so it passes no byte of the record.
#[test]
fn two_readers_of_one_record_at_once_tell_it_once() {
  let at = place("once");
  let mut record = vec![r#"[["made","chain1","store","abc"]]"#.to_owned()];
  record.extend((1..200_000).map(|n| format!(r#"[["stand","stand{n}","operator","chain1"]]"#)));
  fs::write(&at, record.join("\n") + "\n").unwrap();
  let first = thread::spawn({
    let at = at.to_path_buf();
    move || retold(at).unwrap()
  });
  let folder = at.parent().unwrap();
  let telling = || {
    fs::read_dir(folder)
      .unwrap()
      .any(|one| one.unwrap().file_name().to_string_lossy().contains(".told.body."))
  };
  while !telling() {
    assert!(!first.is_finished(), "the first reader is still telling when the second comes");
    thread::yield_now();
  }
  let mut passed = Vec::new();
  let second = retold_with(&at, &mut |done, of| passed.push((done, of))).unwrap();
  let first = first.join().unwrap();
  assert_eq!(passed, [], "the second reader took what the first told");
  assert_eq!((first.file, first.entries), (second.file, second.entries));
}

/// A read tells how far it stands: the bytes of the record that its two passes passed, of twice the
/// bytes of the record, which never go back and end at the whole.
#[test]
fn a_read_of_a_record_tells_the_bytes_that_its_two_passes_passed() {
  let at = place("passed");
  let record = [
    r#"[["made","chain1","store","abc"]]"#,
    r#"[["stand","stand1","operator","chain1"]]"#,
    r#"[["checkpoint","chain1","store",{"chains":[]}]]"#,
    r#"[["cd","cd1","operator","chain1","/tmp"]]"#,
  ];
  fs::write(&at, record.join("\n") + "\n").unwrap();
  let size = fs::metadata(&at).unwrap().len();
  let mut told = Vec::new();
  retold_with(&at, &mut |done, of| told.push((done, of))).unwrap();
  let lines: Vec<u64> = record
    .iter()
    .scan(0, |at, line| Some(*at + line.len() as u64 + 1).inspect(|end| *at = *end))
    .collect();
  let mut want: Vec<(u64, u64)> = lines.iter().map(|end| (*end, 2 * size)).collect();
  want.extend(lines.iter().map(|end| (size + end, 2 * size)));
  want.push((2 * size, 2 * size));
  assert_eq!(told, want);
  told.clear();
  retold_with(&at, &mut |done, of| told.push((done, of))).unwrap();
  assert_eq!(told, [], "a read that takes the file of what the record tells passes no byte");
}
