//! The ear of the compaction, proved on a record that a life of a test made: a carry gives the turns and the open
//! prompts of one of its chains, and a later life reads the carry from its own record.

use std::{
  collections::VecDeque,
  fs,
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
};

use sha2::{Digest, Sha256};

use crate::{
  Engine, Object,
  life::{Answer, Opening},
  verbs, wire, world,
};

/// A directory of its own for one test, and empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-compaction-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
}

/// A life of a test in a directory, on a record of that name there, whose model answers each request with the next
/// word of a script, and with a wait of an hour once the script runs out, so the prompt that it answers stays open.
fn scripted(at: &Path, record: &str, words: &[&str]) -> Opening {
  let said: VecDeque<String> = words.iter().map(|one| (*one).to_owned()).collect();
  let said = Arc::new(Mutex::new(said));
  let answer = Answer(Arc::new(move |_, _| {
    let word = said.lock().unwrap().pop_front().unwrap_or_else(|| "await wait(3600)".to_owned());
    let turn = Object::tuple([
      Object::string("assistant"),
      Object::string(word),
      Object::none(),
      Object::none(),
    ]);
    Box::pin(async move { Ok(turn) })
  }));
  let place = |name: &str| Some(at.join(name).display().to_string());
  Opening {
    directory: at.display().to_string(),
    record: place(record),
    keeps: Some(true),
    config: place("config"),
    roster: Some(vec!["claude-cli:opus".to_owned()]),
    answer: Some(answer),
    ..Opening::default()
  }
}

/// What a word that a rung runs on the root closes with.
fn closed(engine: &mut Engine, word: &str) -> Object {
  let word = Some(word.to_owned());
  let rung =
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() }).unwrap();
  match futures::executor::block_on(rung) {
    Ok(got) => got,
    Err(no) => {
      let told = engine.transcript(verbs::Transcript {
        on: Some("chain1".to_owned()),
        of: Some("tell".to_owned()),
      });
      panic!(
        "{no:?}: {:?}",
        told.map(|all| all
          .iter()
          .rev()
          .take(3)
          .map(|one| one.0.as_ref().py_repr())
          .collect::<Vec<_>>())
      );
    }
  }
}

/// A word that waits until a fact of a chain holds a text, for ten seconds at most.
fn until(chain: &str, text: &str) -> String {
  format!(
    "for _ in range(500):\n  if [x for x in transcript({chain:?}) if {text:?} in str(x)]:\n    break\n  await \
     wait(0.02)\nclose(1)"
  )
}

/// What a probe gives of the chain that an import made: the count of the entries of its record, the digest of the
/// last one and whether its log gave the facts, the prompts that it carried, each fact that the carry said, and the
/// first three words of the assistant turns of the chain.
const PROBE: &str = "(c,) = [x[1] for x in transcript('chain1', 'chain') if x[4] == 'old']\n(k,) = [x[1] for x in \
                     transcript(c) if x[0] == 'carry']\ngot = peek(k)\nassert isinstance(got, dict)\nclose([got['entries'], got['digest'], \
                     got['logged'], got['prompts'], [x for x in transcript(c) if x[2] == k], \
                     [t[1] for t in turns(on=c) if t[0] == 'assistant'][:4]])";

#[test]
fn a_carry_gives_the_turns_and_the_open_prompts_of_a_chain_and_a_later_life_reads_it_from_its_own_record()
 {
  let at = place("carry");
  let mut old = scripted(&at, "old.jsonl", &["close('hi')"]).boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut old, "close(await prompt(str, 'Say hi.'))").as_ref().as_str(), Some("hi"));
  let open = closed(&mut old, "close(prompt(str, 'Later.'))");
  closed(&mut old, &until("chain1", "wait(3600)"));
  drop(old);
  let record = world::kept(at.join("old.jsonl")).unwrap();
  let digest =
    format!("{:x}", Sha256::digest(wire::record(record.last().unwrap().as_ref()).to_string()));
  let mut new = scripted(&at, "new.jsonl", &["close('later')"]).boot::<String>([]).unwrap().0;
  let made = closed(&mut new, "close(imported('old.jsonl', label='old'))");
  let chain = made.as_ref().as_str().unwrap().to_owned();
  closed(&mut new, &until(&chain, "#carry"));
  closed(&mut new, &until(&chain, "later"));
  let got = closed(&mut new, PROBE).as_ref().py_repr();
  let open = open.as_ref().py_repr();
  let start = format!("[{}, '{digest}', True, [({open}, 'str', 'Later.', '')], [", record.len());
  assert!(got.starts_with(&start), "{got}");
  let words = "[\"read('skills://')\", \"close('hi')\", 'await wait(3600)', \"close('later')\"]]";
  assert!(got.ends_with(words), "the chain reads the carried turns, then its own: {got}");
  assert!(got.contains("('reply', 'reply0', 'carry1', "), "each carried reply is reply0: {got}");
  let word =
    format!("close([peek(x[1]) for x in transcript({chain:?}, 'prompt') if x[5] == 'Later.'])");
  assert_eq!(
    closed(&mut new, &word).as_ref().py_repr(),
    "['later']",
    "the carried prompt opens again"
  );
  drop(new);
  for end in ["", ".log"] {
    fs::remove_file(format!("{}{end}", at.join("old.jsonl").display())).unwrap();
  }
  let mut later = scripted(&at, "new.jsonl", &[]).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut later, PROBE).as_ref().py_repr(),
    got,
    "the later life reads the carry from its own record"
  );
}

#[test]
fn a_carry_of_a_record_that_does_not_stand_or_of_a_chain_that_it_does_not_hold_is_refused() {
  let at = place("refused");
  let mut life = scripted(&at, "record.jsonl", &[]).boot::<String>([]).unwrap().0;
  let word = "out = []\nfor where, of in [('none.jsonl', 'chain1'), ('record.jsonl', 'chain9')]:\n  try:\n    await \
              carry(where, of)\n  except Refused as no:\n    out.append(str(no))\nclose(out)";
  let got = closed(&mut life, word).as_ref().py_repr();
  let none = at.join("none.jsonl").display().to_string().replace('\\', "\\\\");
  let held = at.join("record.jsonl").display().to_string().replace('\\', "\\\\");
  assert!(got.starts_with(&format!("['No record at {none}.'")), "{got}");
  assert!(got.ends_with(&format!("'the record {held} holds no chain chain9']")), "{got}");
}

#[test]
#[ignore = "a measure, which a person runs on a copy of a real record, as RECORD=<path> CHAIN=<chain>"]
fn measure_a_carry_of_a_record() {
  let record = std::env::var("RECORD").expect("RECORD names a record");
  let chain = std::env::var("CHAIN").unwrap_or_else(|_| "chain1".to_owned());
  let t = std::time::Instant::now();
  let got = super::carry(Path::new(&record), &chain, "carry1", "chain1").unwrap();
  let count = |name: &str| {
    let pairs = got.as_ref().pairs().unwrap();
    let held = pairs.iter().find(|(key, _)| key.as_str() == Some(name)).map(|(_, one)| *one);
    held.and_then(|one| one.items()).map_or(0, |one| one.len())
  };
  eprintln!(
    "{record} {chain}: {:?}, {} turns, {} open prompts, {} bytes",
    t.elapsed(),
    count("turns"),
    count("prompts"),
    wire::record(got.as_ref()).to_string().len()
  );
}
