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
    skills: (!kits).then_some(false),
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
  let mut named = false;
  let mut theirs = String::new();
  for line in text[cut..].split_inclusive('\n') {
    if named || !line.contains(&made(&world::engine())) {
      theirs.push_str(line);
      continue;
    }
    named = true;
    // The tail names another engine and holds none of its texts, so it goes on on the engine of
    // this build, as a build of another engine whose texts the record never held.
    let mut entry: serde_json::Value = serde_json::from_str(line).unwrap();
    let fact = entry[0].as_array_mut().unwrap();
    fact[3] = serde_json::Value::String("0".repeat(64));
    fact.truncate(7);
    theirs.push_str(&entry.to_string());
    theirs.push_str(&line[line.trim_end_matches('\n').len()..]);
  }
  assert!(named, "the tail names the engine of this build");
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

/// A life that only inspects the record of a directory from its first entry, born again at each of
/// its checkpoints, or why it refused.
fn inspected_whole(at: &Path) -> Result<Engine, furb::Fault> {
  let opening =
    Opening { inspecting: Some(true), whole: Some(true), ..scripted(at, true, &SCRIPT) };
  Ok(opening.boot::<String>([])?.0)
}

/// What each chain of a life holds, as python shows it: each fact that it heard, but its module,
/// which [`MODULES`] shows with no address of the heap, and the outcome of each act among them.
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
    scrubbed(&record, &at);
    // The module is read as the test reads it: from the record alone, with no log beside it, and
    // before the tail names another engine, which this build cannot inspect.
    let alone = place(&format!("made-{name}"));
    fs::copy(&record, alone.join("record.jsonl")).unwrap();
    let shown = modules(&mut inspected(&alone).unwrap());
    if other {
      crossed(&record);
    }
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

/// A life born again three times in a directory of its own, each life on the checkpoint of the one
/// before it, and its record.
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

/// Each proof that `docs/promise.md` names as `path::name` stands in its file as a test of that
/// name that runs, so the page names no proof that is gone or skipped.
#[test]
fn each_proof_that_the_page_of_the_promise_names_stands_in_its_file() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR"));
  let page = fs::read_to_string(root.join("docs/promise.md")).unwrap();
  let named: Vec<&str> =
    page.split('`').skip(1).step_by(2).filter(|one| one.contains("::")).collect();
  assert!(named.len() > 20, "the page names {} proofs", named.len());
  for one in named {
    let (path, name) = one.rsplit_once("::").unwrap();
    let text = fs::read_to_string(root.join(path)).unwrap_or_else(|why| panic!("{one}: {why}"));
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
      .iter()
      .position(|line| {
        line.contains(&format!("fn {name}(")) || line.contains(&format!("def {name}("))
      })
      .unwrap_or_else(|| panic!("{path} holds no test {name}"));
    let marks: Vec<&str> = lines[..at]
      .iter()
      .rev()
      .map(|line| line.trim())
      .take_while(|line| line.starts_with("#[") || line.starts_with("///") || line.starts_with('@'))
      .collect();
    let runs = if Path::new(path).extension().is_some_and(|kind| kind == "rs") {
      marks.iter().any(|mark| mark.ends_with("test]"))
        && !marks.iter().any(|mark| mark.starts_with("#[ignore"))
    } else {
      name.starts_with("test_") && !marks.iter().any(|mark| mark.contains("skip"))
    };
    assert!(runs, "{path}: {name} is no test that runs");
  }
}

/// The seed of the random points of a test: `FURB_PROMISE_SEED` plays one again, and else the clock
/// draws one, which the test prints, so a failure says the seed that plays it again.
fn seeded(name: &str) -> u64 {
  let drawn = || {
    let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    since.map_or(1, |one| u64::try_from(one.as_micros() % 1_000_000).unwrap_or(0) + 1)
  };
  let seed = std::env::var("FURB_PROMISE_SEED").ok().and_then(|one| one.parse().ok());
  let seed = seed.unwrap_or_else(drawn);
  eprintln!("{name} plays the seed {seed}: FURB_PROMISE_SEED={seed} plays it again");
  seed
}

/// The next number that a seed draws, by xorshift, as the soak of the window draws its points.
fn draw(state: &mut u64) -> u64 {
  let mut one = (*state).max(1);
  one ^= one << 13;
  one ^= one >> 7;
  one ^= one << 17;
  *state = one;
  one
}

/// The word of the root of the n-th life of a test, which adds n to what the lives before it kept.
fn kept(n: u64) -> String {
  format!("kept = globals().get('kept', []) + [{n}]\nclose(len(kept))")
}

/// How many values the root of a life kept.
fn holds_kept(engine: &mut Engine) -> i64 {
  engine.word("len(module('chain1').get('kept', []))", vec![]).unwrap().as_ref().as_int().unwrap()
}

/// The life that the test of a kill runs in a process of its own, in the directory that
/// `FURB_PROMISE_KILLED` names: lives of [`kept`] one after the other, each second one born again
/// from its checkpoint, until the test kills the process.
#[test]
#[ignore = "the test of a kill runs it in a process of its own"]
fn a_life_that_a_test_kills() {
  let Ok(at) = std::env::var("FURB_PROMISE_KILLED") else { return };
  let at = Path::new(&at);
  // More lives than the few seconds before the kill can hold.
  for n in 0..10_000 {
    let mut life = scripted(at, false, &SCRIPT).boot::<String>([]).unwrap().0;
    woke(&mut life);
    closed(&mut life, &kept(n));
    if n % 2 == 0 {
      life.checkpoint().unwrap();
    }
  }
}

/// The process of a life that a test kills, which ends with its guard, so a test that fails before
/// its kill leaves no process.
struct Reaped(std::process::Child);

impl Drop for Reaped {
  fn drop(&mut self) {
    self.0.kill().ok();
    self.0.wait().ok();
  }
}

/// Kills the process of a life at the first look at its record at or after the fact `fact`, the
/// first life among them, while the life still runs, and prints the length of the record at the
/// kill: the seed fixes only the fact that the kill comes at or after, and that length plays the
/// kill again as a cut of the record.
fn killed(Reaped(child): &mut Reaped, record: &Path, fact: usize) {
  let begun = std::time::Instant::now();
  while fs::read_to_string(record).map_or(0, |text| text.matches('\n').count()) < fact {
    assert!(child.try_wait().unwrap().is_none(), "the life ended before its kill");
    assert!(
      begun.elapsed() < std::time::Duration::from_secs(120),
      "the life never reached fact {fact}"
    );
    std::thread::sleep(std::time::Duration::from_millis(2));
  }
  assert!(child.try_wait().unwrap().is_none(), "the life still runs at its kill");
  child.kill().unwrap();
  let ended = child.wait().unwrap();
  #[cfg(unix)]
  assert_eq!(
    std::os::unix::process::ExitStatusExt::signal(&ended),
    Some(9),
    "the kill ended the life"
  );
  #[cfg(not(unix))]
  assert!(!ended.success(), "the kill ended the life");
  let text = fs::read_to_string(record).unwrap();
  eprintln!(
    "the kill came at {} lines and {} bytes of the record",
    text.lines().count(),
    text.len()
  );
}

#[test]
fn a_life_killed_at_a_random_fact_opens_at_its_tip_with_a_checkpoint_and_without_one_and_goes_on() {
  let mut points = seeded("kill");
  let at = place("killed");
  let record = at.join("record.jsonl");
  let mut child = Reaped(
    std::process::Command::new(std::env::current_exe().unwrap())
      .args(["a_life_that_a_test_kills", "--exact", "--ignored", "--nocapture"])
      .env("FURB_PROMISE_KILLED", &*at)
      .stdout(std::process::Stdio::null())
      .spawn()
      .unwrap(),
  );
  killed(&mut child, &record, 1 + usize::try_from(draw(&mut points) % 300).unwrap());
  let text = fs::read_to_string(&record).unwrap();
  let expected = i64::try_from(
    text
      .lines()
      .filter(|one| one.starts_with("[[\"rung\",") && one.contains("kept = globals()"))
      .count(),
  )
  .unwrap();
  let mut opened = inspected(&at).unwrap_or_else(|why| panic!("the killed life opens: {why}"));
  assert_eq!(
    holds_kept(&mut opened),
    expected,
    "the killed life holds each word that its record holds"
  );
  let tip = text.rfind("[[\"checkpoint\"").unwrap_or(0);
  assert_eq!(
    missing(&mut opened, &text[tip..]),
    "[]",
    "each question of the record stands in the life"
  );
  let fast = placed(&mut opened, &[&at]);
  let whole = inspected_whole(&at)
    .unwrap_or_else(|why| panic!("the killed life opens from its first entry: {why}"));
  assert!(
    placed(&mut { whole }, &[&at]) == fast,
    "it opens from its last checkpoint and from its first entry to one state"
  );
  let mut later = scripted(&at, false, &SCRIPT).boot::<String>([]).unwrap().0;
  woke(&mut later);
  assert_eq!(
    closed(&mut later, &kept(99)).as_ref().as_int(),
    Some(expected + 1),
    "the killed life goes on"
  );
}

/// The state of a life at its tip, with each of these directories written as <at>, so lives in
/// two directories, a fork and its origin among them, compare.
fn placed(engine: &mut Engine, at: &[&Path]) -> String {
  let home = std::env::var("HOME").unwrap_or_default();
  at.iter().fold(state(engine), |shown, one| {
    let path = one.display().to_string();
    let tilded = path.strip_prefix(&home).map_or_else(|| path.clone(), |rest| format!("~{rest}"));
    shown.replace(&path, "<at>").replace(&tilded, "<at>")
  })
}

/// The names of the questions that the text of a record holds, each a fact whose name is its kind
/// and a number.
fn questions(text: &str) -> Vec<Object> {
  let facts = text.lines().filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok());
  facts
    .filter_map(|one| {
      let (kind, name) = (one[0][0].as_str()?.to_owned(), one[0][1].as_str()?.to_owned());
      let number = name.strip_prefix(&kind)?;
      (!number.is_empty() && number.bytes().all(|one| one.is_ascii_digit()))
        .then(|| Object::string(name))
    })
    .collect()
}

/// The questions of a record that a life which opens it does not hold.
fn missing(engine: &mut Engine, text: &str) -> String {
  let names = Object::list(questions(text));
  engine
    .word("[n for n in __names if not get(n)]", vec![("__names", names)])
    .unwrap()
    .as_ref()
    .py_repr()
}

/// How many entries the store of a directory reads of its record.
fn read_entries(at: &Path) -> usize {
  world::store(at.join("record.jsonl")).unwrap().0.len()
}

#[test]
fn a_record_that_a_kill_tore_in_a_write_or_in_its_checkpoint_opens_as_the_entries_written_whole() {
  let mut points = seeded("torn");
  let (_, text) = thrice("torn-made");
  let starts: Vec<usize> = [0]
    .into_iter()
    .chain(text.match_indices('\n').map(|(at, _)| at + 1))
    .filter(|at| *at < text.len())
    .collect();
  // shortcut: a cut between the gate of the first stand and its done drifts at gate1, since a
  // replay makes that gate before the teach of the extensions that the record holds before it, a
  // defect of the journal of src/furb/engine.py that the Needs of promise-proofs-a74 names with the
  // seed 860273. The test steps over that cut and asserts its drift, so the fix turns it red: then
  // delete `gated`, its filter and its check.
  let gated = text.find("[[\"done\",\"gate1\"").unwrap();
  let known = place("torn-gated");
  fs::write(known.join("record.jsonl"), &text[..gated]).unwrap();
  let drift = inspected(&known).err().map(|why| why.to_string()).unwrap_or_default();
  assert!(
    drift.contains("gate1 drifts"),
    "the known drift at gate1 stands, else delete its step over: {drift:?}"
  );
  let starts: Vec<usize> = starts.into_iter().filter(|at| *at != gated).collect();
  let picked = starts[usize::try_from(draw(&mut points)).unwrap() % starts.len()];
  for (name, start) in [("line", picked), ("checkpoint", text.rfind("[[\"checkpoint\"").unwrap())] {
    let line = text[start..].find('\n').map_or(text.len(), |end| start + end) - start;
    // A cut at the end of the line leaves it whole and not ended, which the store reads as an
    // entry, but at the line of gate1, which then stands with no done.
    let room = if start + line + 1 == gated { line - 1 } else { line };
    let cut = start + 1 + usize::try_from(draw(&mut points)).unwrap() % room;
    let kept_at = if cut == start + line { (start + line + 1).min(text.len()) } else { start };
    let (torn, clean) = (place(&format!("torn-{name}")), place(&format!("clean-{name}")));
    fs::write(torn.join("record.jsonl"), &text[..cut]).unwrap();
    fs::write(clean.join("record.jsonl"), &text[..kept_at]).unwrap();
    assert_eq!(
      read_entries(&torn),
      text[..kept_at].lines().count(),
      "{name}: the store reads each line written whole and no torn one"
    );
    let opened = |at: &Path| {
      inspected(at)
        .unwrap_or_else(|why| panic!("{name}: the record cut at byte {cut} opens: {why}"))
    };
    let whole = placed(&mut opened(&clean), &[&clean]);
    assert!(
      placed(&mut opened(&torn), &[&torn]) == whole,
      "{name}: a record torn at byte {cut} opens as its entries written whole"
    );
    let born = inspected_whole(&torn)
      .unwrap_or_else(|why| panic!("{name}: the torn record opens from its first entry: {why}"));
    assert!(placed(&mut { born }, &[&torn]) == whole, "{name}: and from its first entry");
    let born_at = text[..kept_at].rfind("[[\"checkpoint\"").unwrap_or(0);
    assert_eq!(
      missing(&mut opened(&torn), &text[born_at..kept_at]),
      "[]",
      "{name}: each question written whole stands"
    );
    let mut later = scripted(&torn, false, &SCRIPT).boot::<String>([]).unwrap().0;
    woke(&mut later);
    closed(&mut later, &kept(7));
    drop(later);
    let after = fs::read_to_string(torn.join("record.jsonl")).unwrap();
    assert!(
      after.starts_with(&text[..kept_at])
        && after.lines().all(|one| serde_json::from_str::<serde_json::Value>(one).is_ok()),
      "{name}: the life that goes on cut the torn line and kept each line whole"
    );
  }
}

/// Asserts that the root of the life of a directory goes on in that directory.
fn stands_in(at: &Path) {
  let mut life = inspected(at).unwrap();
  let cwd = life.word("cwd('chain1')", vec![]).unwrap().as_ref().as_str().map(str::to_owned);
  let (home, ours) = (std::env::var("HOME").unwrap_or_default(), at.display().to_string());
  let tilded = ours.strip_prefix(&home).map(|rest| format!("~{rest}"));
  assert!(cwd == Some(ours) || cwd == tilded, "the life goes on in its own directory: {cwd:?}");
}

/// What the root of the life of a directory kept, as python shows it.
fn kept_in(at: &Path) -> String {
  let mut life = inspected(at).unwrap();
  life.word("repr(module('chain1').get('kept'))", vec![]).unwrap().as_ref().py_repr()
}

/// Lives of [`kept`] in the directory of an origin, each second one ended by a checkpoint, and the
/// length of its record and its state at the end of each.
fn lived_for_a_fork(origin: &Path, lives: u64) -> Vec<(usize, String)> {
  // The record of the origin and its open at the end of each life, the state of the origin at a
  // fork there: its own open, since a replay of a life with no checkpoint orders the gate of its
  // first stand otherwise than the live life, the defect that the Needs of promise-proofs-a74
  // names.
  let mut ends = Vec::new();
  for n in 0..lives {
    let mut life = scripted(origin, false, &SCRIPT).boot::<String>([]).unwrap().0;
    woke(&mut life);
    closed(&mut life, &kept(n));
    if n % 2 == 1 {
      life.checkpoint().unwrap();
    } else {
      drop(life);
    }
    let text = fs::read_to_string(origin.join("record.jsonl")).unwrap();
    ends.push((text.len(), placed(&mut inspected(origin).unwrap(), &[origin])));
  }
  ends
}

#[test]
fn a_fork_of_a_life_opens_at_the_state_of_its_origin_at_the_fork_and_goes_on_by_its_own_facts() {
  let mut points = seeded("fork");
  let origin = place("origin");
  let lives = 2 + draw(&mut points) % 2;
  let forked = draw(&mut points) % lives;
  let mut ends = lived_for_a_fork(&origin, lives);
  let text = fs::read_to_string(origin.join("record.jsonl")).unwrap();
  let (point, at_fork) = ends.swap_remove(usize::try_from(forked).unwrap());
  // A fork is the record of its origin up to a fact, in a directory of its own.
  let fork = place("fork");
  fs::write(fork.join("record.jsonl"), &text[..point]).unwrap();
  // The fork opens on the directory of its origin, which the World gives and the record does not
  // hold, so the open differs by no stand of its own; it goes on in its own directory after.
  let on_origin = Opening {
    inspecting: Some(true),
    record: Some(fork.join("record.jsonl").display().to_string()),
    ..scripted(&origin, true, &SCRIPT)
  };
  let opened = placed(&mut on_origin.boot::<String>([]).unwrap().0, &[&origin]);
  assert!(opened == at_fork, "the fork opens at the state of its origin after life {forked}");
  // In its own directory it holds each question and each value of its origin at the fork.
  let born_at = text[..point].rfind("[[\"checkpoint\"").unwrap_or(0);
  assert_eq!(
    missing(&mut inspected(&fork).unwrap(), &text[born_at..point]),
    "[]",
    "the fork holds each question of its origin"
  );
  let upto = |last: u64| (0..last).map(|n| n.to_string()).collect::<Vec<_>>().join(", ");
  assert_eq!(
    kept_in(&fork),
    format!("'[{}]'", upto(forked + 1)),
    "the fork holds the values of its origin at the fork"
  );
  for (at, n) in [(&origin, 50), (&fork, 60)] {
    let mut life = scripted(at, false, &SCRIPT).boot::<String>([]).unwrap().0;
    woke(&mut life);
    closed(&mut life, &kept(n));
  }
  assert_eq!(
    kept_in(&origin),
    format!("'[{}, 50]'", upto(lives)),
    "the origin holds none of the facts of its fork"
  );
  assert_eq!(
    kept_in(&fork),
    format!("'[{}, 60]'", upto(forked + 1)),
    "the fork holds the facts of its origin up to the fork and its own"
  );
  let (ours, theirs) = (
    fs::read_to_string(fork.join("record.jsonl")).unwrap(),
    fs::read_to_string(origin.join("record.jsonl")).unwrap(),
  );
  assert!(
    ours.starts_with(&text[..point]) && theirs.starts_with(&text),
    "each record goes on from where it stood"
  );
  stands_in(&fork);
}
