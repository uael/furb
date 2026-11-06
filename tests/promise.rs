//! The promise of a stable release, on the public API alone: each record of each folder of
//! `tests/corpus` opens on this build with no refusal and no drift, gives back each value of each
//! module and what each definition gives, reads with the reader of a record and no life, and takes
//! one more turn on a model that answers from a script. `tests/corpus/README.md` says the rule of
//! the folders.

#![expect(
  clippy::unwrap_used,
  clippy::expect_used,
  reason = "a helper of a test fails the test where it fails"
)]
#![expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the act of a rung, as the tests of the crate do"
)]

#[path = "common/yard.rs"]
mod yard;

use std::{
  fs,
  path::{Path, PathBuf},
  sync::Arc,
};

use furb::{
  Engine, Object,
  life::{Answer, Opening},
  verbs, world,
};

/// The folder of the corpus, with a folder for each release and `next` for the coming one.
fn corpus() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("corpus")
}

/// A directory of its own for one test, and empty.
fn place(name: &str) -> yard::Yard {
  yard::Yard::new(format!("furb-promise-{name}-{}", std::process::id()))
}

/// A life on the record of a directory, with each default kit unless `kits` is false, whose one
/// model reads the last message of a request alone: it gives a name to the kit that names a
/// session, the word of the message of a script that the message names last, and close(None) else.
fn scripted(at: &Path, kits: bool, script: &[(&str, &str)]) -> Opening {
  let said: Vec<(String, String)> =
    script.iter().map(|(asked, word)| ((*asked).to_owned(), (*word).to_owned())).collect();
  let answer = Answer(Arc::new(move |request, _| {
    let text = request["messages"].as_array().and_then(|all| all.last()).map(ToString::to_string);
    let text = text.unwrap_or_default();
    let last =
      said.iter().filter_map(|(asked, word)| Some((text.rfind(asked.as_str())?, word))).max();
    let word = if text.contains("Give a name to a session") && request["chain"] != "chain1" {
      "close('The corpus')".to_owned()
    } else {
      last.map_or_else(|| "close(None)".to_owned(), |(_, word)| word.clone())
    };
    let turn = [Object::string("assistant"), Object::string(word), Object::none(), Object::none()];
    Box::pin(async move { Ok(Object::tuple(turn)) })
  }));
  let path = |name: &str| Some(at.join(name).display().to_string());
  Opening {
    directory: at.display().to_string(),
    record: path("record.jsonl"),
    keeps: Some(true),
    config: path("config"),
    roster: Some(vec!["claude-cli:opus".to_owned()]),
    answer: Some(answer),
    extensions: (!kits).then_some(false),
    ..Opening::default()
  }
}

/// What a word that a rung runs on the root closes with.
fn closed(engine: &mut Engine, word: &str) -> Object {
  let word = Some(word.to_owned());
  let rung =
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() }).unwrap();
  futures::executor::block_on(rung).unwrap()
}

/// Each chain of a life, and for each its values and what each of its definitions gives, as python
/// shows them: each name that a word bound, but the names of the engine and of each kit, since a
/// kit and a default may change in a minor.
const MODULES: &str = "(lambda kits: [(c, sorted((k, repr(v)) for k, v in module(c).items() if k not in globals() \
                       and k not in kits and k not in ('__name__', 'actor') and type(v).__name__ != 'function'), \
                       sorted((k, repr(v())) for k, v in module(c).items() if k not in globals() and k not in kits \
                       and type(v).__name__ == 'function')) for c in [f'chain{n}' for n in range(1, 99) \
                       if get(f'chain{n}')]])({n for e in transcript('chain1', 'enable') for n in \
                       re.findall(r'^(?:async def |def |class |type )?(\\w+)', e[4], re.M)})";

/// The chains of a life, each with its values and what each of its definitions gives.
fn modules(engine: &mut Engine) -> String {
  engine.word(MODULES, vec![]).unwrap().as_ref().py_repr()
}

/// The word of a model that waits an hour in the first life that runs it, so a checkpoint cuts it,
/// and closes in a later life.
const CUT: &str = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0)\nclose(f'round {rounds}')";

/// The script of the model of each life of the corpus. To the first message, the root writes,
/// reads, runs a command, waits, reads the clock and chance, and asks a chain of its own. The word
/// of the message `Corpus: wait.` is the one that the checkpoint cuts, and the message `Corpus:
/// ask.` stays open.
const SCRIPT: [(&str, &str); 6] = [
  (
    "Corpus: work in the directory.",
    "write(Text(\"notes.md\", \"one\\ntwo\\n\"))\nnotes = read(\"notes.md\")\nsaid = await bash(\"echo hi\")\n\
     await wait(0.01, __name__)\nhelper = chain(\"helper\")\ngot = await prompt(int, \"Corpus: give 2.\", on=helper)\nclose(got + \
     len(notes.lines) + int(clock() > 0) + int(0 <= chance() < 1))",
  ),
  ("Corpus: give 2.", "close(2)"),
  ("Corpus: wait.", CUT),
  ("Corpus: say done.", "close('done')"),
  ("Corpus: ask.", "await wait(3600, __name__)\nclose('late')"),
  ("Corpus: one more.", "close('more')"),
];

/// The life of a record of the corpus: a first life that works, defines, and leaves a prompt open
/// on a word that its checkpoint cuts, then a tail that takes a prompt, defines on what the
/// checkpoint holds, and leaves a prompt open.
fn lived(at: &Path, kits: bool) {
  let mut first = scripted(at, kits, &SCRIPT).boot::<String>([]).unwrap().0;
  let asks = "close(await prompt(int, 'Corpus: work in the directory.'))";
  assert_eq!(closed(&mut first, asks).as_ref().as_int(), Some(6));
  let defines = "SIZE = 6\nqueue = [1, 'two']\ndef size():\n  return SIZE\nwork = chain('work')\n\
                 waiting = prompt(str, 'Corpus: wait.', on=work)\nawait wait(0.3, __name__)\nclose(str(waiting))";
  closed(&mut first, defines);
  first.checkpoint().unwrap();
  let mut later = scripted(at, kits, &SCRIPT).boot::<String>([]).unwrap().0;
  woke(&mut later);
  let goes =
    "after = size() + 1\nqueue.append(after)\nclose(await prompt(str, 'Corpus: say done.'))";
  assert_eq!(closed(&mut later, goes).as_ref().as_str(), Some("done"));
  let leaves = "more = chain('more')\nasked = prompt(str, 'Corpus: ask.', on=more)\nawait wait(0.3, __name__)\n\
                close(str(asked))";
  closed(&mut later, leaves);
}

/// Wakes the root of a life born from a checkpoint, which pauses it when the checkpoint carries a
/// reply of its model, as an operator wakes it.
fn woke(engine: &mut Engine) {
  engine.word("over('chain1') or wake('chain1')", vec![]).unwrap();
}

/// Makes the facts after the last checkpoint of a record name another engine, as a build of another
/// engine names them.
fn crossed(record: &Path) {
  let text = fs::read_to_string(record).unwrap();
  let made = |engine: &str| format!("[\"made\",\"chain1\",\"store\",\"{engine}\",null");
  let cut = text
    .rfind("[[\"checkpoint\"")
    .and_then(|at| text[at..].find('\n').map(|end| at + end))
    .expect("the record holds a checkpoint");
  let theirs = text[cut..].replacen(&made(&world::engine()), &made(&"0".repeat(64)), 1);
  assert_ne!(theirs, text[cut..], "the tail names the engine of this build");
  fs::write(record, format!("{}{theirs}", &text[..cut])).unwrap();
}

/// The words of the record of the language: each definition that a word of a model uses, with no
/// argument to call it by, since the module is shown by calling each function of it. The first
/// four sit in the checkpoint, and the last three in the tail after it.
const LANGUAGE: [&str; 7] = [
  "def square(n: int = 3) -> int:\n  return n * n\nclass Runner:\n  async def twice(self, n: int = 21) -> int:\n    await wait(0, __name__)\n    \
   return n * 2\nasync_got = await Runner().twice()\ndef fib_list():\n  a, b, out = 0, 1, []\n  while a < 50:\n    \
   out.append(a)\n    a, b = b, a + b\n  return out\nclose(None)",
  "from dataclasses import dataclass\nclass Tally:\n  def __init__(self, start: int):\n    self.n = start\n  \
   def bump(self) -> int:\n    self.n += 1\n    return self.n\n@dataclass\nclass Point:\n  x: int\n  y: int = 0\n  \
   def norm1(self) -> int:\n    return abs(self.x) + abs(self.y)\ndef classes():\n  c = Tally(3)\n  c.bump()\n  \
   return [c.n, Point(2, -5).norm1(), Point(1)]\nclose(None)",
  "def shape_of(v=None):\n  match v:\n    case {\"kind\": \"dot\", \"at\": [x, y]}:\n      \
   return f\"dot {x},{y}\"\n    case [first, *rest] if rest:\n      return f\"seq {first}+{len(rest)}\"\n    \
   case int() | float() as num:\n      return f\"num {num}\"\n    case Point(x=0):\n      \
   return \"origin column\"\n    case _:\n      return \"other\"\ndef matched():\n  \
   return [shape_of({\"kind\": \"dot\", \"at\": [1, 2]}), shape_of([1, 2, 3]), shape_of(2.5), \
   shape_of(Point(0, 4)), shape_of(\"s\")]\nclose(None)",
  "def comprehended():\n  squares = [n * n for n in range(6) if n % 2 == 0]\n  \
   pairs = {k: v for k, v in zip(\"abc\", range(3))}\n  uniq = {n % 3 for n in range(10)}\n  \
   lazy = sum(n for n in range(5))\n  return [squares, pairs, sorted(uniq), lazy, \
   [[i * j for j in range(3)] for i in range(2)]]\nclose(None)",
  "def formatted():\n  name, width = \"monty\", 9\n  \
   return [f\"{name!r:>{width}}\", f\"{3.14159:.2f}\", f\"{name=}\", f\"{1_000:,}\"]\n\
   def templated():\n  who = \"world\"\n  made = t\"hello {who!r:>9}\"\n  \
   return [[x if isinstance(x, str) else (x.value, x.conversion, x.format_spec) for x in made], \
   len(made.strings)]\nclose(None)",
  "type Pair[T] = tuple[T, T]\ndef swap[T](p: Pair[T] | None = None) -> Pair[T] | None:\n  return (p[1], p[0]) if p else None\n\
   class Box[T]:\n  def __init__(self, item: T):\n    self.item = item\n  def get(self) -> T:\n    \
   return self.item\ndef generic():\n  return [swap((1, 2)), Box(\"x\").get(), Box(7).get()]\nclose(None)",
  "def logged(fn=None):\n  def inner(*args):\n    return (\"logged\", fn(*args) if fn else None)\n  \
   return inner\ndef repeat(times: int = 2):\n  def deco(fn):\n    def inner(*args):\n      \
   return [fn(*args) for _ in range(times)]\n    return inner\n  return deco\n@logged\n\
   def add(a: int = 1, b: int = 2) -> int:\n  return a + b\n@repeat(2)\ndef hi():\n  return \"hi\"\n\
   def decorated():\n  return [add(1, 2), hi()]\nclose(None)",
];

/// The life of the record of the language: no kit, so no name of a kit is in it. The words sit in
/// a checkpoint and in the tail after it, and the module gives back each of them.
fn spoken(at: &Path) {
  let mut first = scripted(at, false, &SCRIPT).boot::<String>([]).unwrap().0;
  for word in &LANGUAGE[..4] {
    closed(&mut first, word);
  }
  first.checkpoint().unwrap();
  let mut later = scripted(at, false, &SCRIPT).boot::<String>([]).unwrap().0;
  woke(&mut later);
  for word in &LANGUAGE[4..] {
    closed(&mut later, word);
  }
}

/// A life that only inspects the record of a directory, or why it refused.
fn inspected(at: &Path) -> Result<Engine, furb::Fault> {
  Ok(Opening { inspecting: Some(true), ..scripted(at, true, &SCRIPT) }.boot::<String>([])?.0)
}

/// A life that only inspects the record of a directory from its first entry, born again at each of its checkpoints,
/// or why it refused.
fn inspected_whole(at: &Path) -> Result<Engine, furb::Fault> {
  let opening =
    Opening { inspecting: Some(true), whole: Some(true), ..scripted(at, true, &SCRIPT) };
  Ok(opening.boot::<String>([])?.0)
}

/// What each chain of a life holds, as python shows it: each fact that it heard, but its module, which [`MODULES`]
/// shows with no address of the heap, and the outcome of each act among them.
const STATE: &str = "[(c, [x for x in transcript(c) if x[0] != 'module'], [(x[1], peek(x[1], ...)) for x in transcript(c) if question(x)]) \
                     for c in [f'chain{n}' for n in range(1, 99) if get(f'chain{n}')]]";

/// The state of a life at its tip: what each chain holds and each of its modules.
fn state(engine: &mut Engine) -> String {
  let held = engine.word(STATE, vec![]).unwrap().as_ref().py_repr();
  format!("{held}\n{}", modules(engine))
}

/// Proves that a record opens from its last checkpoint and from its first entry to one state.
fn whole(name: &str, at: &Path) {
  let fast = state(&mut inspected(at).unwrap());
  let born = inspected_whole(at)
    .unwrap_or_else(|why| panic!("{name}: an open from the first entry is refused: {why}"));
  assert!(state(&mut { born }) == fast, "{name}: both opens reach one state");
}

#[test]
#[ignore = "it makes tests/corpus/next again, which a change with ! may do before 1.0.0, as tests/corpus/README.md says"]
fn make_the_corpus() {
  let next = corpus().join("next");
  if next.exists() {
    fs::remove_dir_all(&next).unwrap();
  }
  fs::create_dir_all(&next).unwrap();
  for (name, kits, other) in
    [("work", true, false), ("crossed", false, true), ("language", false, false)]
  {
    let at = place(&format!("make-{name}"));
    if name == "language" {
      spoken(&at);
    } else {
      lived(&at, kits);
    }
    let record = at.join("record.jsonl");
    if other {
      crossed(&record);
    }
    scrubbed(&record, &at);
    // The module is read as the test reads it: from the record alone, with no log beside it.
    let alone = place(&format!("made-{name}"));
    fs::copy(&record, alone.join("record.jsonl")).unwrap();
    let shown = modules(&mut inspected(&alone).unwrap());
    fs::copy(&record, next.join(format!("{name}.jsonl"))).unwrap();
    fs::write(next.join(format!("{name}.modules")), format!("{shown}\n")).unwrap();
  }
}

/// Cuts the listing of each folder above the directory of a life, which the kit memory reads, to
/// the folder that leads down to it, so that no record holds the names of the files of the machine
/// that made it.
fn scrubbed(record: &Path, at: &Path) {
  let mut text = fs::read_to_string(record).unwrap();
  let mut listings: Vec<(String, String)> = text
    .lines()
    .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .filter(|entry| entry[0][0] == "done")
    .filter_map(|entry| {
      let path = entry[0][3]["path"].as_str()?.to_owned();
      let content = entry[0][3]["content"].as_str()?.to_owned();
      Some((path, content))
    })
    .filter(|(path, _)| Path::new(path) != at && at.starts_with(path))
    .collect();
  listings.sort();
  listings.dedup();
  for (path, content) in listings {
    let below = at.strip_prefix(&path).unwrap().components().next().unwrap();
    let below = format!("{}/", below.as_os_str().to_string_lossy());
    let quoted = |one: &str| serde_json::to_string(one).unwrap();
    let held = format!("\"path\":{},\"content\":{}", quoted(&path), quoted(&content));
    assert!(text.contains(&held), "the record holds the listing of {path} as JSON writes it");
    text =
      text.replace(&held, &format!("\"path\":{},\"content\":{}", quoted(&path), quoted(&below)));
  }
  fs::write(record, text).unwrap();
}

/// Each record of each folder of the corpus, as its folder and its name.
fn records() -> Vec<(String, PathBuf)> {
  let mut found = Vec::new();
  for folder in fs::read_dir(corpus()).unwrap().map(|one| one.unwrap().path()) {
    if !folder.is_dir() {
      continue;
    }
    let named = folder.file_name().unwrap().to_string_lossy().into_owned();
    for one in fs::read_dir(&folder).unwrap().map(|one| one.unwrap().path()) {
      if one.extension().is_some_and(|ext| ext == "jsonl") {
        found.push((named.clone(), one));
      }
    }
  }
  found.sort();
  found
}

/// Opens a record in a directory of its own, from its text, and proves the promise on it.
fn holds(name: &str, text: &str, shown: &str) {
  let at = place(name);
  let record = at.join("record.jsonl");
  fs::write(&record, text).unwrap();
  let mut engine = inspected(&at)
    .unwrap_or_else(|why| panic!("{name} is refused on this build, or drifts on it: {why}"));
  assert_eq!(
    modules(&mut engine),
    shown.trim_end(),
    "{name} gives back each value and each definition"
  );
  let chains: Vec<String> = engine
    .word("[f'chain{n}' for n in range(1, 99) if get(f'chain{n}')]", vec![])
    .unwrap()
    .as_ref()
    .items()
    .unwrap()
    .iter()
    .filter_map(|one| one.as_str().map(str::to_owned))
    .collect();
  drop(engine);
  whole(name, &at);
  assert_eq!(
    fs::read_to_string(&record).unwrap(),
    text,
    "{name}: a life that inspects writes nothing"
  );
  let told =
    world::retold(&record).unwrap_or_else(|why| panic!("{name}: the reader refuses it: {why}"));
  let facts: Vec<String> = told.facts().unwrap().iter().map(|one| one.as_ref().py_repr()).collect();
  for chain in &chains {
    let made = format!("'chain', '{chain}'");
    assert!(facts.iter().any(|one| one.contains(&made)), "{name}: the reader tells {chain}");
  }
  let mut later = scripted(&at, true, &SCRIPT).boot::<String>([]).unwrap().0;
  woke(&mut later);
  let turn = "close(await prompt(str, 'Corpus: one more.'))";
  assert_eq!(
    closed(&mut later, turn).as_ref().as_str(),
    Some("more"),
    "{name} takes one more turn"
  );
}

#[test]
fn each_record_of_each_folder_of_the_corpus_opens_gives_back_its_modules_and_takes_one_more_turn() {
  let found = records();
  assert!(found.iter().any(|one| one.0 == "next"), "tests/corpus/next holds no record");
  for (folder, one) in found {
    let stem = one.file_stem().unwrap().to_string_lossy().into_owned();
    let shown = fs::read_to_string(one.with_extension("modules"))
      .unwrap_or_else(|_| panic!("{folder}/{stem} has no file .modules beside it"));
    holds(&format!("{folder}-{stem}"), &fs::read_to_string(&one).unwrap(), &shown);
  }
}

#[test]
fn a_record_whose_checkpoint_holds_a_field_that_no_reader_knows_still_opens() {
  let next = corpus().join("next");
  let text = fs::read_to_string(next.join("work.jsonl")).unwrap();
  let more =
    text.replacen("\"schema\":", "\"later\":[1,\"a field of a later build\"],\"schema\":", 1);
  assert_ne!(more, text, "the record holds a checkpoint");
  holds("field-more", &more, &fs::read_to_string(next.join("work.modules")).unwrap());
}

/// A life born again three times in a directory of its own, each life on the checkpoint of the one before it, and
/// its record.
fn thrice(name: &str) -> (yard::Yard, String) {
  let at = place(name);
  for n in 0..4 {
    let mut life = scripted(&at, false, &SCRIPT).boot::<String>([]).unwrap().0;
    woke(&mut life);
    let word = format!(
      "kept = globals().get('kept', []) + [{n}]\nlater = chain('later{n}')\nclose(len(kept))"
    );
    assert_eq!(
      closed(&mut life, &word).as_ref().as_int(),
      Some(n + 1),
      "life {n} goes on from the one before"
    );
    if n < 3 {
      life.checkpoint().unwrap();
    }
  }
  let text = fs::read_to_string(at.join("record.jsonl")).unwrap();
  assert_eq!(text.matches("[[\"checkpoint\"").count(), 3, "the record holds three births");
  (at, text)
}

#[test]
fn a_life_born_again_three_times_opens_from_its_first_entry_to_the_state_of_its_last_checkpoint() {
  let (at, _) = thrice("thrice");
  whole("thrice", &at);
}

#[test]
fn an_open_from_the_first_entry_refuses_a_checkpoint_that_no_life_gathers_or_that_a_life_went_on_after()
 {
  let (_, text) = thrice("thrice-made");
  let cut = text.find("[[\"checkpoint\"").unwrap();
  let end = cut + text[cut..].find('\n').unwrap() + 1;
  let other = text[cut..end].replacen("\"kept\":\"[0]\"", "\"kept\":\"[9]\"", 1);
  assert_ne!(other, text[cut..end], "the first checkpoint binds kept");
  let went = text[..cut].lines().last().unwrap();
  for (name, changed, why) in [
    ("drifted", format!("{}{other}{}", &text[..cut], &text[end..]), "drifts in chains"),
    ("went-on", format!("{}{went}\n{}", &text[..end], &text[end..]), "is no birth"),
  ] {
    let at = place(name);
    fs::write(at.join("record.jsonl"), changed).unwrap();
    inspected(&at).unwrap_or_else(|why| panic!("{name} opens from its last checkpoint: {why}"));
    let refused = inspected_whole(&at).err().map(|no| no.to_string()).unwrap_or_default();
    assert!(
      refused.contains(why),
      "{name}: the open from the first entry says {why}, and says {refused:?}"
    );
  }
}

#[test]
fn the_record_of_the_language_holds_each_definition_that_a_word_uses() {
  let shown = fs::read_to_string(corpus().join("next").join("language.modules")).unwrap();
  let used = [
    "Box",
    "Pair",
    "Point",
    "Runner",
    "Tally",
    "async_got",
    "classes",
    "comprehended",
    "decorated",
    "formatted",
    "generic",
    "matched",
    "templated",
  ];
  for name in used {
    assert!(shown.contains(&format!("('{name}', ")), "language.modules lacks {name}");
  }
}
