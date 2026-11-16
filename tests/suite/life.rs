//! The opening of a life: what it enables, what it keeps, and the ears it hears, on the record that
//! the store keeps.

#![expect(clippy::disallowed_methods, reason = "a test blocks on its runtime and fails by a panic")]
#![expect(clippy::unwrap_used, reason = "a helper of a test fails the test where it fails")]
#![cfg_attr(
  unix,
  expect(clippy::expect_used, reason = "a test of the systems of unix fails where its call fails")
)]

use crate::opened::{
  changed, closed, corpus, crossed, crossing, elsewhere, made_of, named, names, opening, ours,
  replayed, scripted, segment, worked, yard,
};

/// The image of a record dropped: it stands beside the record, as `<record>.image`.
fn drop_image(record: &Path) {
  drop(fs::remove_file(format!("{}.image", record.display())));
}

use std::{
  env, fs,
  io::Write,
  path::{Path, PathBuf},
  process,
  sync::{Arc, Mutex},
};

#[cfg(unix)]
use std::time::Instant;

use futures::{channel::mpsc::unbounded, executor, future, future::select};
use tokio::runtime;

use crate::yard::Yard;
use furb::life::{Answer, Opening, Progress, Stage};
use furb::{
  Act, Ear, Engine, Fact, Object, Text,
  checkpoint::CHECKPOINT,
  ear::{ear, hear, say},
  engine::OPERATOR,
  image,
  value::{Fault, entry},
  verbs, world,
};

#[test]
fn a_life_enables_what_the_configs_turn_on_and_a_later_life_runs_what_its_record_enables() {
  let at = yard("configs");
  let (mut first, _) = opening(&at, true).boot::<String>([]).unwrap();
  assert_eq!(names(&mut first), ["catchup", "memory", "skills", "compaction", "naming"]);
  drop(first);
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config").join("config.json"), r#"{"skills": {"skills": false}}"#).unwrap();
  let (mut later, record) = opening(&at, true).boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut later),
    ["catchup", "memory", "skills", "compaction", "naming"],
    "the record wins over the configs"
  );
  drop(later);
  let kept = world::kept(at.join("record.jsonl")).unwrap();
  assert!(!kinds(&kept[record.len()..]).contains(&"enable".to_owned()), "it enabled nothing new");
  let fresh = Some(at.join("fresh.jsonl").display().to_string());
  let (mut fresh, _) = Opening { record: fresh, ..opening(&at, true) }.boot::<String>([]).unwrap();
  assert_eq!(
    names(&mut fresh),
    ["catchup", "memory", "compaction", "naming"],
    "a life on a new record enables what the configs turn on"
  );
}

#[test]
fn a_life_that_enables_its_skills_at_an_open_opens_again_by_its_record_and_plays_each_one_on_a_new_chain()
 {
  let at = yard("enables-at-open");
  drop(Opening { skills: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap());
  let (mut enabling, _) = opening(&at, true).boot::<String>([]).unwrap();
  let copy = at.join("copy.jsonl");
  fs::copy(at.join("record.jsonl"), &copy).unwrap();
  let read = |life: &mut Engine| {
    let state = "[(c, turns(on=c), sorted(module(c))) for c in ('chain1',)]";
    let state = life.word(state, vec![]).unwrap().as_ref().py_repr();
    (state, life.word(PLAYED, vec![]).unwrap().as_ref().py_repr())
  };
  let lived = read(&mut enabling);
  drop(enabling);
  let copied = Some(copy.display().to_string());
  let (mut again, _) = Opening { record: copied, ..opening(&at, true) }.boot::<String>([]).unwrap();
  assert_eq!(read(&mut again), lived, "the replay holds the state of the life and its extensions");
}

#[test]
fn a_life_that_inspects_its_record_keeps_nothing_enables_nothing_new_and_its_ears_that_do_work_are_silent()
 {
  let at = yard("inspects");
  fs::write(at.join("a.txt"), "one\n").unwrap();
  drop(Opening { skills: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap());
  let before = fs::read_to_string(at.join("record.jsonl")).unwrap();
  let (mut engine, _) =
    Opening { inspecting: Some(true), ..opening(&at, true) }.boot::<String>([]).unwrap();
  let read = verbs::Read { on: Some("chain1".to_owned()), ..Default::default() };
  assert_eq!(engine.read("a.txt", read).unwrap_err().message(), "nothing takes read");
  assert_eq!(names(&mut engine), Vec::<String>::new(), "it enables nothing the record does not");
  drop(engine);
  assert_eq!(fs::read_to_string(at.join("record.jsonl")).unwrap(), before);
}

#[test]
fn the_ears_of_the_crate_come_after_the_ears_of_the_host_each_under_the_name_every_host_hears_it_by()
 {
  let at = yard("ears");
  let (_, ears) = opening(&at, true).parts().unwrap();
  let names: Vec<&str> = ears.iter().map(|(name, _)| name.as_str()).collect();
  assert_eq!(
    names,
    ["provider", "extensions", "skills", "kits", "fetch", "files", "bash", "time", "store", "log"]
  );
  drop(ears);
  let (mut engine, _) = opening(&at, true).boot([("mine", mine())]).unwrap();
  let read = verbs::Read { on: Some(engine.root().to_owned()), ..Default::default() };
  assert_eq!(engine.read("mine://a", read).unwrap(), Text::new("mine://a", "mine\n"));
}

#[test]
fn a_life_that_keeps_a_record_writes_a_new_file_and_a_new_file_in_a_new_folder_to_its_scratch_directory()
 {
  let at = yard("scratch");
  let (mut engine, _) =
    Opening { skills: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap();
  let root = engine.root().to_owned();
  let on = || Some(root.clone());
  let write = verbs::Write { on: on() };
  let plan = engine.write(&Text::new("scratch://plan.md", "plan\n"), write).unwrap();
  assert_eq!(plan, Text::new("scratch://plan.md", "plan\n"));
  let write = verbs::Write { on: on() };
  let note = engine.write(&Text::new("scratch://notes/new.md", "note\n"), write).unwrap();
  assert_eq!(note, Text::new("scratch://notes/new.md", "note\n"));
  let read = verbs::Read { on: on(), ..Default::default() };
  assert_eq!(engine.read("scratch://", read).unwrap(), Text::new("scratch://", "notes/\nplan.md"));
  drop(engine);
  // The scratch directory of the record holds the two files that the life wrote: remove it.
  fs::remove_dir_all(world::Scratch::of(&at.join("record.jsonl")).unwrap().unwrap().path())
    .unwrap();
}

#[test]
fn a_life_that_keeps_a_record_writes_the_log_of_every_fact_beside_it_and_a_later_life_writes_it_again_from_its_start()
 {
  let at = yard("log");
  let record = at.join("record.jsonl");
  let open = || Opening { skills: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap().0;
  let mut engine = open();
  let word = Some("x = 1\nclose(x)".to_owned());
  let rung =
    engine.rung(verbs::Rung { word, on: Some("chain1".to_owned()), ..Default::default() }).unwrap();
  assert_eq!(executor::block_on(rung).unwrap().as_ref().as_int(), Some(1));
  drop(engine);
  let shown =
    |facts: Vec<Object>| facts.iter().map(|one| one.as_ref().py_repr()).collect::<Vec<_>>();
  let first = shown(world::logged(&record).unwrap().expect("the log agrees with the record"));
  let rung = first.iter().position(|one| one.contains("'operator', 'chain1', 'x = 1"));
  let named =
    rung.and_then(|at| first[at].split('\'').nth(3).map(str::to_owned)).unwrap_or_default();
  let run = first
    .iter()
    .position(|one| one.starts_with("['run', ") && one.contains(&format!(", '{named}', 'x = 1")));
  assert!(
    rung.is_some_and(|rung| run.is_some_and(|run| rung < run)),
    "the log holds the rung and its run"
  );
  assert!(
    world::kept(&record).unwrap().iter().any(|one| one.as_ref().py_repr().contains("'run1'")),
    "the record keeps the question of the run that the chain made"
  );
  drop(open());
  let again = shown(world::logged(&record).unwrap().expect("a later life writes the log again"));
  // A fact that the journal says again is its own, so a later life says each fact again with its
  // kind and its name.
  let head = |one: &String| one.splitn(3, ", ").take(2).collect::<Vec<_>>().join(", ");
  // The journal says the fact teach again only once the bus is still, so a replay logs it later
  // than the first life did.
  let heads = |facts: &[String]| {
    facts.iter().map(head).filter(|one| !one.starts_with("['teach'")).collect::<Vec<_>>()
  };
  assert!(again.iter().any(|one| one.starts_with("['teach', 'chain1'")), "{again:?}");
  let mine = heads(&first);
  assert_eq!(
    heads(&again)[..mine.len()],
    mine[..],
    "a later life says the log again before its own facts"
  );
  let mut kept = fs::OpenOptions::new().append(true).open(&record).unwrap();
  Write::write_all(&mut kept, b"[[\"stand\",\"stand9\",\"operator\",\"chain1\"]]\n").unwrap();
  assert!(
    world::logged(&record).unwrap().is_none(),
    "a log that its record outgrew does not agree"
  );
}

#[test]
fn a_later_life_writes_in_its_log_each_act_of_its_record_that_an_ear_of_the_outside_made_and_does_not_make_again()
 {
  let at = yard("owed");
  let record = at.join("record.jsonl");
  let open = || {
    let opening = Opening { skills: Some(false), ..opening(&at, true) };
    opening.boot::<String>([]).unwrap().0
  };
  let mut engine = open();
  assert_eq!(
    closed(&mut engine, "await bash('true', show=HIDDEN)\nclose(1)").as_ref().as_int(),
    Some(1)
  );
  drop(engine);
  let asked = |facts: &[Object]| {
    facts.iter().any(|one| one.as_ref().py_repr().contains("['merged', 'merged1', 'bash'"))
  };
  assert!(
    asked(&world::kept(&record).unwrap()),
    "the record keeps the question that the ear of commands asked"
  );
  let mut engine = open();
  assert_eq!(closed(&mut engine, "close(2)").as_ref().as_int(), Some(2));
  drop(engine);
  let facts =
    world::logged(&record).unwrap().expect("the log of the later life agrees with the record");
  assert!(asked(&facts), "the log of the later life holds the question that no ear asked again");
}

#[test]
fn a_life_whose_open_act_a_verb_of_no_words_made_crosses_to_another_build() {
  let at = yard("ends-kept");
  let mut life = Opening { skills: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  // A verb with no words makes an act that makes an open prompt, as name_session makes the
  // prompt of its chain.
  let word = "with site.set(act('named', ROOT, None)):\n  named = prompt(str, 'Name?', to=OPERATOR)\nclose(1)";
  assert_eq!(closed(&mut life, word).as_ref().as_int(), Some(1));
  drop(life);
  crossing(&at);
  drop(Opening { skills: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0);
  let builds = builds(&at.join("record.jsonl"));
  assert_eq!(builds.last(), Some(&world::engine()), "the life carries its open act across");
}

#[test]
fn a_life_whose_module_binds_the_name_of_a_builtin_crosses_with_the_module_of_its_replay() {
  let at = yard("builtin");
  let open = || Opening { skills: Some(false), ..opening(&at, true) };
  let read = |life: &mut Engine| closed(life, "close([list, k, j])").as_ref().py_repr();
  let mut first = open().boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut first, "globals()['list'] = 'ls -la'\nk = 1\nclose(1)").as_ref().as_int(),
    Some(1)
  );
  // An ear tells the chain a note in prose from outside a run, as the ear of the skills tells a
  // change of a config, and the next word binds what the chain heard.
  let note = "(t := site.set('extensions'), say('tell', ROOT, ['The key extensions of a config is now skills.']), site.reset(t))";
  first.word(note, vec![]).unwrap();
  assert_eq!(closed(&mut first, "j = 2\nclose(1)").as_ref().as_int(), Some(1));
  drop(first);
  let mut replayed = open().boot::<String>([]).unwrap().0;
  let by_replay = read(&mut replayed);
  drop(replayed);
  assert_eq!(
    by_replay, "['ls -la', 1, 2]",
    "a replay binds the name of the builtin as the life bound it"
  );
  crossing(&at);
  for _ in 0..2 {
    let mut again = open().boot::<String>([]).unwrap().0;
    assert_eq!(
      read(&mut again),
      by_replay,
      "the open across a crossing reaches the module of the replay"
    );
  }
}

#[test]
fn a_life_whose_record_ends_in_a_fact_made_that_a_stop_tore_opens_at_each_crossing_with_no_drift() {
  let at = yard("torn-made");
  let open = || Opening { skills: Some(false), ..opening(&at, true) };
  let read = |life: &mut Engine| closed(life, "close(k)").as_ref().py_repr();
  let mut first = open().boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut first, "k = 41\nclose(1)").as_ref().as_int(), Some(1));
  drop(first);
  crossing(&at);
  let mut crossed = open().boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut crossed, "k += 1\nclose(1)").as_ref().as_int(), Some(1));
  drop(crossed);
  let record = at.join("record.jsonl");
  let text = fs::read_to_string(&record).unwrap();
  let made = text.lines().rfind(|one| one.starts_with("[[\"made\",")).unwrap();
  fs::write(&record, format!("{text}{}", &made[..made.len() / 2])).unwrap();
  let entries = world::kept(&record).unwrap();
  let places = world::crossings(&record).unwrap();
  assert!(!places.is_empty(), "the record crosses to this build");
  for place in places {
    let fact = entries[place].as_ref().py_repr();
    assert!(fact.starts_with("[['made', "), "the crossing at {place} is a fact made: {fact}");
  }
  let mut again = open().boot::<String>([]).unwrap().0;
  assert_eq!(read(&mut again), "42", "the life opens at the state of its record");
}

#[test]
fn a_log_after_a_crossing_goes_on_from_the_log_before_it_when_that_log_agrees_and_else_counts_the_record()
 {
  for case in ["agrees", "none", "short"] {
    let at = yard(&format!("log-{case}"));
    let open = || Opening { skills: Some(false), ..opening(&at, true) };
    let mut first = open().boot::<String>([]).unwrap().0;
    assert_eq!(closed(&mut first, "k = 1\nclose(1)").as_ref().as_int(), Some(1));
    drop(first);
    crossing(&at);
    let mut crossed = open().boot::<String>([]).unwrap().0;
    assert_eq!(closed(&mut crossed, "k += 1\nclose(1)").as_ref().as_int(), Some(1));
    drop(crossed);
    let (record, log) = (at.join("record.jsonl"), at.join("record.jsonl.log"));
    let tip = *world::crossings(&record).unwrap().last().expect("the record crossed");
    let before = fs::read_to_string(&log).unwrap();
    let held = world::kept(&record).unwrap().len();
    match case {
      "none" => fs::remove_file(&log).unwrap(),
      "short" => fs::write(&log, "{\"entries\":0}\n").unwrap(),
      _ => {}
    }
    // The image would open the life with no read of its record, and so with no new log.
    drop(fs::remove_file(format!("{}.image", record.display())));
    drop(open().boot::<String>([]).unwrap().0);
    let after = fs::read_to_string(&log).unwrap();
    if case == "agrees" {
      // The log before keeps its lines up to the entries before the crossing, and the life says
      // the segment after it again.
      let lines: Vec<&str> = before.lines().collect();
      let crossed = format!("[\"keep\",\"\",{tip}]");
      let kept = lines.iter().position(|one| *one == crossed).expect("the keep of the crossing");
      let kept: String = lines[..=kept].iter().flat_map(|one| [*one, "\n"]).collect();
      assert!(after.starts_with(&kept) && after.len() > kept.len(), "{case}: {after}");
    } else {
      // A log that does not agree is written again: its head counts each entry of the record,
      // which the life says no more but for each act that it owes.
      assert!(after.starts_with(&format!("{{\"entries\":{held}}}\n")), "{case}: {after}");
    }
  }
}

#[test]
fn a_record_names_the_engine_that_made_it_once_in_a_file_of_lines_and_in_a_database() {
  for name in ["record.jsonl", "record.db"] {
    let at = yard(&format!("made-{name}"));
    let record = at.join(name);
    let open = || {
      let path = Some(record.display().to_string());
      Opening { record: path, skills: Some(false), ..opening(&at, true) }.boot::<String>([])
    };
    drop(open().unwrap());
    let made = vec![world::engine()];
    assert_eq!(builds(&record), made);
    let kept = world::kept(&record).unwrap();
    assert_eq!(
      world::release(&kept).as_deref(),
      Some(env!("CARGO_PKG_VERSION")),
      "it names the release"
    );
    let schema = kept
      .iter()
      .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
      .find(|one| one.kind() == world::MADE)
      .and_then(|one| one.word(2).and_then(|word| word.as_str().map(str::to_owned)));
    assert_eq!(schema, Some(world::RECORD_SCHEMA.to_string()), "it names the schema of the record");
    drop(open().unwrap());
    assert_eq!(builds(&record), made, "a later life names it no more");
  }
}

/// A segment after a crossing replays on the checkpointer that seeded it, which the fact made of
/// the segment holds, and never on the checkpointer of this build: here a checkpointer whose seed
/// binds one more value in the root, which a life that keeps nothing reads, since it does not
/// cross.
#[test]
fn the_segment_after_a_crossing_replays_on_the_checkpointer_that_seeded_it() {
  let at = yard("seeded");
  let open = |keeps| Opening { skills: Some(false), ..opening(&at, keeps) }.boot::<String>([]);
  let mut first = open(true).unwrap().0;
  closed(&mut first, "x = 1\nclose(x)");
  drop(first);
  crossing(&at);
  closed(&mut open(true).unwrap().0, "x += 1\nclose(x)");
  let seeds = "__seed = seed\ndef seed(names, data):\n  data['chains'][0]['values']['seeded'] = 'True'\n  \
               return __seed(names, data)\n";
  let checkpointer = format!("{}\n{seeds}", furb::CHECKPOINTER);
  let record = at.join("record.jsonl");
  let text = fs::read_to_string(&record).unwrap();
  let made = named(&[furb::ENGINE, furb::KERNEL, &checkpointer]).0;
  fs::write(&record, text.replace(&ours(), &made)).unwrap();
  let read = "close([x, globals().get('seeded')])";
  assert_eq!(
    closed(&mut open(false).unwrap().0, read).as_ref().py_repr(),
    "[2, True]",
    "the tail replays on the seed that bore it"
  );
}

/// A record whose checkpointer cannot gather the life of its engine, as an older one whose
/// `literal` gave a bool for a lost value, crosses all the same: the crossing replays the record
/// on its engine and gathers it with the checkpointer of this build, which reads the next life.
#[test]
fn a_record_whose_checkpointer_cannot_gather_crosses_with_the_checkpointer_of_this_build() {
  let at = yard("gathers");
  let raising = "def gathered(names, define, floor):\n  \
                 raise TypeError('sequence item 3: expected str instance, bool found')\n";
  let checkpointer = format!("{}\n{raising}", furb::CHECKPOINTER);
  let (made, named) = named(&[furb::ENGINE, furb::KERNEL, &checkpointer]);
  let (record, _) = made_elsewhere(&at, &made);
  let open = || Opening { skills: Some(false), ..opening(&at, true) };
  let mut later = open().boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut later, "close(x + 1)").as_ref().as_int(), Some(2));
  drop(later);
  assert_eq!(builds(&record), [named, world::engine()], "the record crossed to this build");
}

/// A life that keeps its record and opens on a record of another engine crosses to this build at
/// its open: it carries the life at the tip of the record on that engine to this build, writes the
/// fact made of this build and nothing else, and goes on with this build, and a later open
/// crosses no more.
#[test]
fn a_record_of_another_engine_crosses_to_this_build_at_its_open() {
  let at = yard("texts");
  let (made, named) = elsewhere("crossed = True");
  let (record, text) = made_elsewhere(&at, &made);
  let open = || Opening { skills: Some(false), ..opening(&at, true) };
  let read = |life: &mut Engine, n: i64| {
    closed(life, &format!("close([x + {n}, globals().get('crossed')])")).as_ref().py_repr()
  };
  let mut later = open().boot::<String>([]).unwrap().0;
  assert_eq!(read(&mut later, 1), "[2, None]", "the crossing moves the life to this build");
  drop(later);
  let kept = world::kept(&record).unwrap();
  let kinds: Vec<_> =
    kept.iter().filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of)).collect();
  assert!(kinds.iter().all(|one| one.kind() != CHECKPOINT), "the record keeps no carry");
  assert_eq!(builds(&record), [named.clone(), world::engine()]);
  assert_eq!(world::crossings(&record).unwrap().len(), 1, "the crossing is the one fact made");
  let mut again = open().boot::<String>([]).unwrap().0;
  assert_eq!(read(&mut again, 2), "[3, None]", "the life goes on with this build");
  drop(again);
  assert_eq!(builds(&record), [named.clone(), world::engine()], "a later open crosses no more");
  fs::write(&record, text.replace(&named, &"0".repeat(64))).unwrap();
  let refused = open().boot::<String>([]).err().map(|one| format!("{one:?}")).unwrap_or_default();
  assert!(
    refused.contains("another engine"),
    "texts of another engine refuse the record: {refused}"
  );
}

/// Only a life that keeps its record writes an image: each other life says facts that its record
/// does not hold, as the stand of its boot, so no image of it is a cache of its record.
#[test]
fn only_a_life_that_keeps_its_record_writes_an_image() {
  let at = yard("image-unkept");
  let open = |inspecting, keeps| {
    let opening =
      Opening { inspecting: Some(inspecting), skills: Some(false), ..opening(&at, keeps) };
    opening.boot::<String>([]).unwrap().0
  };
  closed(&mut open(false, true), "x = 1\nclose(x)");
  let record = at.join("record.jsonl");
  assert!(
    !image::keep(&mut open(false, false), &record, &()).unwrap(),
    "a life that works writes none"
  );
  assert!(
    !image::keep(&mut open(true, false), &record, &()).unwrap(),
    "a life that inspects writes none"
  );
  assert!(
    image::keep(&mut open(false, true), &record, &()).unwrap(),
    "a life that keeps writes one"
  );
}

/// An image names the texts of the engine of the build that wrote it, so a build of other texts
/// reads none, and the life opens by its record.
#[test]
fn an_image_of_a_build_of_other_texts_does_not_load() {
  let at = yard("image-texts");
  let open = || Opening { skills: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap();
  let (mut first, _) = open();
  closed(&mut first, "x = 1\nclose(x)");
  assert!(
    image::keep(&mut first, &at.join("record.jsonl"), &()).unwrap(),
    "the life writes its image"
  );
  drop(first);
  let path = at.join("record.jsonl.image");
  let held = fs::read(&path).unwrap();
  let other = String::from_utf8_lossy(&held).replacen(&world::engine(), &"0".repeat(64), 1);
  fs::write(&path, other.as_bytes()).unwrap();
  let (mut later, replayed) = open();
  assert!(!replayed.is_empty(), "the life opens by its record");
  assert_eq!(closed(&mut later, "close(x)").as_ref().py_repr(), "1");
}

/// A chain that crossed reads in its turns the lines of the minified engine that differ, as one
/// paragraph headed crossed, and a chain with the crossed chain as its source holds the same turns
/// first.
#[test]
fn a_chain_that_crossed_reads_the_lines_of_the_engine_that_differ() {
  let at = yard("crossed-turns");
  made_elsewhere(&at, &elsewhere("crossed = True").0);
  let mut later =
    Opening { skills: Some(false), ..opening(&at, true) }.boot::<String>([]).unwrap().0;
  let turns = |life: &mut Engine, on: &str| {
    let word = format!("close([t[1] for t in turns(on={on:?})])");
    closed(life, &word)
      .as_ref()
      .items()
      .unwrap()
      .iter()
      .map(|one| one.as_str().unwrap().to_owned())
      .collect::<Vec<_>>()
  };
  let root = turns(&mut later, "chain1");
  let told = root.last().unwrap();
  let paragraph = told.split("\n\n").find(|one| one.starts_with("#chain1 crossed")).unwrap();
  assert_eq!(
    paragraph.lines().take(4).collect::<Vec<_>>(),
    ["#chain1 crossed", "<s:chain1_diff>", "--- engine", "+++ engine"]
  );
  assert!(paragraph.contains("\n-crossed=True\n"), "{paragraph}");
  assert!(paragraph.ends_with("</s:chain1_diff>"), "{paragraph}");
  closed(&mut later, "close(chain('fork', 'chain1'))");
  let fork = turns(&mut later, "chain2");
  assert!(fork[0].starts_with(root[0].as_str()), "the fork holds the turns of its source first");
}

/// After a crossing, a chain that crossed and a chain with it as its source read the engine that
/// they read first, which their turns tell the change of, and a new chain with no source reads the
/// engine of this build whole.
#[cfg(unix)]
#[test]
fn after_a_crossing_each_chain_reads_the_system_prompt_that_it_read_first() {
  let at = yard("crossed-system");
  made_elsewhere(&at, &elsewhere("elsewhere = True").0);
  let mut later = Opening { keeps: Some(true), ..faked(&at) }.boot::<String>([]).unwrap().0;
  let ask = |life: &mut Engine, on: &str| {
    closed(life, &format!("close(await prompt(str, 'Say.', to=actor, on={on:?}))"));
  };
  ask(&mut later, "chain1");
  closed(&mut later, "close([chain('fork', 'chain1'), chain('new')])");
  ask(&mut later, "chain2");
  ask(&mut later, "chain3");
  let read = systems_read(&at);
  let old = |one: &String| one.contains("elsewhere");
  assert_eq!(
    read.iter().map(old).collect::<Vec<_>>(),
    [true, true, false],
    "the old engine, twice"
  );
  assert_eq!(read[2], furb::SYSTEM, "a new chain reads this build whole");
}

/// The model of a life that opens by a record of another engine reads that engine.
#[cfg(unix)]
#[test]
fn the_model_of_a_life_that_opens_by_its_record_reads_the_engine_that_the_record_holds() {
  let at = yard("record-system");
  made_elsewhere(&at, &elsewhere("elsewhere = True").0);
  let mut later = faked(&at).boot::<String>([]).unwrap().0;
  let system = system(&at, &mut later);
  assert!(system.contains("elsewhere"), "the model reads the engine of the life: {system:.200}");
}

#[cfg(unix)]
#[test]
fn the_boot_of_a_life_asks_the_claude_command_line_its_models_once_and_a_later_stand_again() {
  use std::os::unix::fs::PermissionsExt;
  let at = yard("discovery");
  let bin = at.join("claude");
  let fake = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/world/provider/claude/fake-claude.sh");
  // A program written now has a stamp of its own, so no discovery of an earlier run in the cache of
  // furb answers it.
  fs::write(&bin, fs::read(fake).unwrap()).unwrap();
  fs::set_permissions(&bin, PermissionsExt::from_mode(0o755)).unwrap();
  fs::write(at.join("models"), "nova 100 low\n").unwrap();
  let asked = || fs::read_to_string(at.join("discoveries")).unwrap_or_default().lines().count();
  let (mut engine, _) = Opening {
    claude: Some(bin.display().to_string()),
    roster: Some(vec!["claude-cli:nova".to_owned()].into()),
    skills: Some(false),
    ..opening(&at, true)
  }
  .boot::<String>([])
  .unwrap();
  // This test failed once in 20 full runs, with no log that said why. Each check says what the
  // fake saw, so the next failure names its cause.
  let saw = || {
    let heard = |name: &str| fs::read_to_string(at.join(name)).unwrap_or_else(|no| no.to_string());
    format!("discoveries: {:?}, model: {:?}", heard("discoveries"), heard("model"))
  };
  assert_eq!(
    asked(),
    1,
    "the opening asked the models, and the stand of the boot holds what it found; {}",
    saw()
  );
  let started = Instant::now();
  let stand = stood(&mut engine);
  let took = started.elapsed();
  assert!(stand.is_ok(), "the stand took {took:?} and failed: {stand:?}; {}", saw());
  assert_eq!(asked(), 2, "a later stand asks the command line again, in {took:?}; {}", saw());
}

#[test]
fn a_life_opens_when_its_actor_and_a_roster_name_left_the_catalog_and_tells_each_chain_once() {
  let at = yard("catalog-left");
  let (mut engine, _) = Opening {
    skills: Some(false),
    roster: Some(vec!["claude-cli:opus".to_owned(), "claude-cli:gone".to_owned()].into()),
    actor: Some("claude-cli:vanished/high".to_owned()),
    ..scripted(&at, &[])
  }
  .boot::<String>([])
  .unwrap();
  let roster = "close([a[0] for a in standing()[0]])";
  assert_eq!(closed(&mut engine, roster).py_repr(), "['claude-cli:opus', 'operator']");
  let actor = "close(standing()[2])";
  assert_eq!(closed(&mut engine, actor).py_repr(), "'claude-cli:opus/high'");
  let left = r#"close(len([1 for x in transcript('chain1') if x[0] == 'tell' and x[2] == 'provider' and "_left = ['claude-cli:gone', 'claude-cli:vanished/high']" in x[3][0]]))"#;
  assert_eq!(closed(&mut engine, left).as_ref().as_int(), Some(1));
  stood(&mut engine).unwrap();
  assert_eq!(closed(&mut engine, left).as_ref().as_int(), Some(1), "a later stand tells nothing");
  let refused = "try:\n  await prompt(int, 'which', to='claude-cli:gone')\nexcept Refused as no:\n  close(str(no))";
  assert_eq!(closed(&mut engine, refused).py_repr(), "'claude-cli:gone no actor'");
}

#[test]
fn a_life_that_names_no_actor_and_no_roster_goes_on_with_the_ones_its_record_stood_on_last() {
  let at = yard("stood-again");
  let both = "close((standing()[2], sorted(a[0] for a in standing()[0])))";
  let given = Opening {
    skills: Some(false),
    roster: Some(vec!["claude-cli:opus".to_owned(), "claude-cli:sonnet".to_owned()].into()),
    actor: Some("claude-cli:sonnet/low".to_owned()),
    ..scripted(&at, &[])
  };
  let (mut first, _) = given.boot::<String>([]).unwrap();
  let had = closed(&mut first, both).py_repr();
  assert_eq!(
    had,
    "('claude-cli:sonnet/low', ['claude-cli:opus', 'claude-cli:sonnet', 'operator'])"
  );
  drop(first);
  let unsaid = Opening { skills: Some(false), roster: None, actor: None, ..scripted(&at, &[]) };
  let (mut later, _) = unsaid.boot::<String>([]).unwrap();
  assert_eq!(
    closed(&mut later, both).py_repr(),
    had,
    "the open named neither, so the record rules"
  );
}

#[test]
fn a_stand_reloads_provider_settings_and_a_refused_reload_keeps_the_life() {
  let at = yard("provider-settings");
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(
    at.join("config/config.json"),
    r#"{"providers":{"bench":{"url":"http://127.0.0.1:9/v1","models":{"one":{"window":65536}}}}}"#,
  )
  .unwrap();
  let path = at.join(".furb/config.json");
  fs::create_dir_all(at.join(".furb")).unwrap();
  fs::write(&path, r#"{"providers":{"claude-cli":false,"local":false}}"#).unwrap();
  let (mut engine, _) = Opening {
    skills: Some(false),
    roster: None,
    actor: Some("bench:one".to_owned()),
    ..scripted(&at, &["close(23)", "close(29)"])
  }
  .boot::<String>([])
  .unwrap();
  let names = "close([a[0] for a in standing()[0] if str(a[0]).startswith('bench:')])";
  assert_eq!(closed(&mut engine, names).py_repr(), "['bench:one']");
  assert_eq!(
    closed(&mut engine, "marker = 17\nclose(await prompt(int, to='bench:one'))").as_ref().as_int(),
    Some(23)
  );
  fs::write(&path, r#"{"providers":{"bench":{"models":{"two":{"window":131072,"images":true}}}}}"#)
    .unwrap();
  stood(&mut engine).unwrap();
  assert_eq!(closed(&mut engine, names).py_repr(), "['bench:two']");
  assert_eq!(
    closed(&mut engine, "close(marker + await prompt(int, to='bench:two'))").as_ref().as_int(),
    Some(46)
  );
  fs::write(&path, r#"{"providers":{"bench":{"models":{"bad":{"window":0}}}}}"#).unwrap();
  assert!(stood(&mut engine).is_err());
  assert_eq!(closed(&mut engine, names).py_repr(), "['bench:two']");
  assert_eq!(closed(&mut engine, "close(marker)").as_ref().as_int(), Some(17));
  fs::write(&path, r#"{"providers":{"bench":false}}"#).unwrap();
  stood(&mut engine).unwrap();
  assert_eq!(closed(&mut engine, names).py_repr(), "[]");
  drop(engine);
  fs::remove_dir_all(at).unwrap();
}

#[test]
fn a_life_whose_replay_drifts_keeps_nothing_beside_its_record_but_the_lease() {
  let at = yard("drifts");
  let record = at.join("record.jsonl");
  // The place that the first cd of the record asks for, changed, makes the replay drift at that
  // cd: the word of the root asks for the place where its stand stood. The new place holds a
  // backslash on each system, as a place of Windows does, which python writes as two.
  let mut lines: Vec<String> =
    fs::read_to_string(worked("drifts-work")).unwrap().lines().map(str::to_owned).collect();
  let moved = lines
    .iter()
    .position(|line| line.starts_with(r#"[["cd","cd1","#))
    .expect("the record holds the first cd");
  let mut entry: serde_json::Value = serde_json::from_str(&lines[moved]).unwrap();
  let place = entry[0][4].as_str().expect("the cd names its place").to_owned();
  let elsewhere = format!("{place}\\elsewhere");
  entry[0][4] = serde_json::Value::from(elsewhere.as_str());
  lines[moved] = entry.to_string();
  // The last line is whole and not ended, which a store mends only at its first keep.
  fs::write(&record, lines.join("\n")).unwrap();
  fs::write(at.join("record.jsonl.log"), "{\"entries\": 0}\n").unwrap();
  let held = |name: &str| fs::read(at.join(name)).unwrap();
  let (text, log) = (held("record.jsonl"), held("record.jsonl.log"));
  // The config directory of the user stands apart, since a life writes there the copy of each word.
  let apart = Yard::unmade(format!("furb-life-drifts-config-{}", process::id()));
  let config = Some(apart.display().to_string());
  let refused =
    Opening { config, ..opening(&at, true) }.boot::<String>([]).map(|_| ()).unwrap_err();
  assert!(refused.to_string().starts_with("Drift: cd1 drifts: "), "{refused}");
  // The drift shows the place as python writes it.
  let shown = Object::string(elsewhere).py_repr();
  assert!(
    refused.to_string().contains(&format!(", {shown})")),
    "the drift is at the cd that the record holds: {refused}"
  );
  let mut beside: Vec<String> = fs::read_dir(&at)
    .unwrap()
    .map(|one| one.unwrap().file_name().to_string_lossy().into_owned())
    .collect();
  beside.sort();
  assert_eq!(beside, ["record.jsonl", "record.jsonl.lock", "record.jsonl.log"]);
  assert_eq!(held("record.jsonl"), text, "the record stays as it was");
  assert_eq!(held("record.jsonl.log"), log, "the log stays as it was");
}

/// A record from before the stable cut of the schema of the record opens in no life, not even one
/// that inspects it, which says why, and nothing writes on it.
#[test]
fn a_record_from_before_the_stable_cut_opens_in_no_life() {
  let at = yard("before-cut");
  let record = at.join("record.jsonl");
  let before = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/replay/speaker.jsonl");
  fs::copy(before, &record).unwrap();
  let text = fs::read_to_string(&record).unwrap();
  let why = world::SchemaError::BeforeCut { of: "record" }.to_string();
  for (inspecting, keeps) in [(true, false), (false, false), (false, true)] {
    let open = Opening { inspecting: Some(inspecting), ..opening(&at, keeps) };
    let refused = open.boot::<String>([]).map(|_| ()).unwrap_err().to_string();
    assert!(refused.ends_with(&why), "inspecting {inspecting}, keeps {keeps}: {refused}");
  }
  assert_eq!(fs::read_to_string(&record).unwrap(), text, "nothing writes on it");
}

/// The operator is no model of the catalog, so a later life whose standing names the operator as
/// its actor, as the standing of a life whose models left the catalog does, tells no chain that the
/// operator left it, by its image or by its record.
#[test]
fn a_life_never_tells_that_the_operator_left_the_catalog() {
  for imaged in [true, false] {
    let at = yard("left-once");
    let record = at.join("record.jsonl");
    let operated = Opening { actor: Some(OPERATOR.to_owned()), ..opening(&at, true) };
    let mut first = operated.boot::<String>([]).unwrap().0;
    let stood = first.word("standing()[2]", vec![]).unwrap();
    assert_eq!(stood.as_ref().as_str(), Some(OPERATOR), "the standing names the operator");
    assert!(image::keep(&mut first, &record, &()).unwrap(), "the life writes its image");
    drop(first);
    if !imaged {
      drop_image(&record);
    }
    let mut later = opening(&at, true).boot::<String>([]).unwrap().0;
    let word = "len([x for x in transcript('chain1', 'tell') if 'left the catalog' in x[3][0]])";
    let told = later.word(word, vec![]).unwrap();
    assert_eq!(told.as_ref().as_int(), Some(1), "the first life told it, imaged {imaged}");
  }
}

/// Each record of each folder of the corpus opens from the image that a life that keeps it wrote at
/// its tip to the state that its record gives, in a life that inspects, in one that works and keeps
/// nothing, and in one that keeps the record. An image is a cache of its record, so a record that a
/// build cannot open by its image is a defect of that build.
#[test]
fn each_record_of_the_corpus_opens_by_its_image_to_the_state_of_its_record() {
  let word = "[[c, turns(on=c), sorted(k for k in module(c) if not k.startswith('__'))] \
              for c in [f'chain{n}' for n in range(1, 99) if get(f'chain{n}')]]";
  for one in corpus() {
    let name = one.file_stem().unwrap().to_string_lossy().into_owned();
    for (how, inspecting, keeps) in
      [("inspects", true, false), ("works", false, false), ("keeps", false, true)]
    {
      // The state of the life of the mode, opened by the image that a life that keeps the record
      // wrote, or by the record once that image is gone.
      let read = |imaged: bool| {
        let at = yard(&format!("imaged-{name}-{how}"));
        let record = at.join("record.jsonl");
        fs::copy(&one, &record).unwrap();
        let open = |inspecting, keeps| {
          Opening { inspecting: Some(inspecting), ..opening(&at, keeps) }.boot::<String>([])
        };
        let mut first = open(false, true)
          .unwrap_or_else(|no| panic!("{name}: a life that keeps it opens: {no}"))
          .0;
        assert!(
          image::keep(&mut first, &record, &()).unwrap(),
          "{name}: the life writes its image"
        );
        drop(first);
        if !imaged {
          drop_image(&record);
        }
        let mut life = open(inspecting, keeps)
          .unwrap_or_else(|no| panic!("{name} {how}: the life opens: {no}"))
          .0;
        let shown = life.word(word, vec![]).unwrap().as_ref().py_repr();
        shown
          .replace(at.file_name().and_then(|one| one.to_str()).expect("a yard with a name"), "<at>")
      };
      assert_eq!(read(true), read(false), "{name} {how}: the image gives the state of the record");
    }
  }
}

/// An image that an earlier build wrote opens on each later build by the image, to the state of its
/// record, as monty loads a dump that an earlier monty wrote: `tests/image` holds a record and its
/// image, which no build makes again but the one that raises the version of an image. That build
/// writes both again with `FURB_IMAGE_AGAIN=1`, from `language.jsonl` of the corpus. The stand of
/// the life says the home, the folder and the doctrine, and `cargo xtask private` fails on a trace
/// of a person in either file, so the test writes them in a run of its own program whose `HOME` and
/// `TMPDIR` are folders that name no person, whatever the environment of the command that asks.
#[test]
fn an_image_that_an_earlier_build_wrote_opens_by_the_image_to_the_state_of_its_record() {
  let frozen = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("image");
  let at = yard("image-of-before");
  let record = at.join("record.jsonl");
  match env::var("FURB_IMAGE_AGAIN").ok().as_deref() {
    Some(NAMELESS) => image_again(&at, &frozen),
    Some(_) => image_again_nameless(),
    None => {}
  }
  for name in ["record.jsonl", "record.jsonl.image"] {
    fs::copy(frozen.join(name), at.join(name)).unwrap();
  }
  let word = "[[c, turns(on=c), sorted(k for k in module(c) if not k.startswith('__'))] \
              for c in [f'chain{n}' for n in range(1, 99) if get(f'chain{n}')]]";
  let read = || {
    let told = Arc::new(Mutex::new(Vec::new()));
    let heard = Arc::clone(&told);
    let progress = Progress(Arc::new(move |stage, _, _| heard.lock().unwrap().push(stage)));
    let reactor = runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let opening = Opening { inspecting: Some(true), ..opening(&at, false) };
    let mut life =
      reactor.block_on(opening.open::<String>([], progress, &mut |_| true, None)).unwrap().0;
    let state = life.word(word, vec![]).unwrap().as_ref().py_repr();
    let booted = told.lock().unwrap().contains(&Stage::Boot);
    (state, booted)
  };
  let (imaged, booted) = read();
  assert!(
    !booted,
    "the image of tests/image opens by the image: a change that reads it to another meaning raises \
     the version of an image and writes it again with FURB_IMAGE_AGAIN=1"
  );
  drop_image(&record);
  let (recorded, booted) = read();
  assert!(booted, "with no image the life boots on its record");
  assert_eq!(imaged, recorded, "the image of an earlier build gives the state of its record");
}

/// The record of the life of `WORK`, and that record once it crossed to another build, which a
/// later life carries across, so the rung by which a birth plays each life word again shows in a
/// turn too. A record of the corpus replays on the checkpointer that made it, which
/// may hide it.
#[test]
fn each_run_of_a_chain_of_the_replays_has_a_turn_that_shows_its_word() {
  let unshown = replayed("unshown").word(UNSHOWN, vec![]).unwrap();
  assert_eq!(unshown.as_ref().items().unwrap().len(), 0, "{}", unshown.as_ref().py_repr());
  let at = yard("shown-born");
  fs::copy(worked("shown-born-made"), at.join("record.jsonl")).unwrap();
  crossing(&at);
  drop(opening(&at, true).boot::<String>([]).unwrap().0);
  let mut born = Opening { inspecting: Some(true), ..opening(&at, false) }.boot::<String>([]);
  let born = &mut born.as_mut().unwrap().0;
  let rungs = born.word("[x[4] for x in transcript('chain1', 'rung') if x[2] == OPERATOR]", vec![]);
  assert!(
    rungs.unwrap().as_ref().py_repr().contains("for __part in"),
    "the birth plays its life words"
  );
  let unshown = born.word(UNSHOWN, vec![]).unwrap();
  assert_eq!(unshown.as_ref().items().unwrap().len(), 0, "{}", unshown.as_ref().py_repr());
}

#[test]
fn a_replay_of_work_shows_each_file_of_a_skill_as_the_rung_of_its_read_then_its_code_alone() {
  let mut engine = replayed("seeded");
  let enables = engine.word("len(transcript('chain1', 'enable'))", vec![]).unwrap();
  assert_eq!(
    enables.as_ref().as_int(),
    Some(5),
    "each official extension that plays by default runs on the life of work"
  );
  let unseeded = engine.word(UNSEEDED, vec![]).unwrap();
  assert_eq!(unseeded.as_ref().items().unwrap().len(), 0, "{}", unseeded.as_ref().py_repr());
}

#[test]
fn an_open_that_inspects_a_record_that_crossed_replays_the_life_words_that_ask_the_clock_with_no_drift()
 {
  let at = yard("tip-clock");
  let mut first = scripted(&at, &["close('one')"]).boot::<String>([]).unwrap().0;
  let word = "close(await prompt(str, 'Reply with the word one.'))";
  assert_eq!(closed(&mut first, word).as_ref().as_str(), Some("one"));
  // The turns up to the tip, which the pending turn of the operator ends.
  let upto = |life: &mut Engine, n: i64| {
    let word = format!("repr(turns(on='chain1')[:{n}])");
    life.word(&word, vec![]).unwrap().as_ref().py_repr()
  };
  let n = first.word("len(turns(on='chain1')) - 1", vec![]).unwrap().as_ref().as_int().unwrap();
  let tip = upto(&mut first, n);
  drop(first);
  crossing(&at);
  drop(scripted(&at, &[]).boot::<String>([]).unwrap().0);
  // As furb turns opens it: each segment replays on its engine, and the last on the carry.
  let inspects = Opening { inspecting: Some(true), ..scripted(&at, &[]) };
  let mut inspected =
    inspects.boot::<String>([]).expect("the open replays to its tip with no drift").0;
  let clocks = "[peek(x[1], None) for x in transcript('chain1', 'clock')]";
  let asked = inspected.word(clocks, vec![]).unwrap().as_ref().py_repr();
  assert!(
    !asked.contains("Refused") && !asked.contains("None"),
    "the life that crossed kept the clock that its birth asked, which the open that inspects reads: {asked}"
  );
  // Each turn up to the tip is as in the life that wrote it and as in a live reopen.
  let seen = upto(&mut inspected, n);
  drop(inspected);
  assert_eq!(seen, tip, "the open that inspects stands at the tip");
  let mut live = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(upto(&mut live, n), seen, "the open that inspects stands where a live reopen stands");
  let again = live.word(clocks, vec![]).unwrap().as_ref().py_repr();
  assert!(
    again.starts_with(&asked[..asked.len() - 1]),
    "a live reopen reads the same clocks: {again}"
  );
}

/// A record whose last segment another engine, Kernel or checkpointer made crosses to this build
/// in a life that keeps it, whose birth starts the parts that live, as the kit catchup asks the
/// clock. Each later open, which inspects the record or keeps it, replays that life from its record
/// to the same turns with no drift, since the record keeps what that birth asked of the World.
#[test]
fn a_record_that_crossed_onto_other_texts_opens_again_with_no_drift() {
  for text in 0..3 {
    let at = yard(&format!("crossed-texts-{text}"));
    let mut first = opening(&at, true).boot::<String>([]).unwrap().0;
    assert_eq!(closed(&mut first, "n = 1\nclose(n)").as_ref().as_int(), Some(1));
    drop(first);
    let mut held =
      [furb::ENGINE.to_owned(), furb::KERNEL.to_owned(), furb::CHECKPOINTER.to_owned()];
    held[text].push_str("\n# another build\n");
    let other = named(&[&held[0], &held[1], &held[2]]).0;
    changed(&at, &ours(), |line| line.replace(&ours(), &other));
    let turns =
      "repr([turns(on=c) for c in [f'chain{n}' for n in range(1, 9) if get(f'chain{n}')]])";
    let clocks = "repr([peek(x[1], None) for x in transcript('chain1', 'clock')])";
    let mut crossing = opening(&at, true).boot::<String>([]).unwrap().0;
    let read = |life: &mut Engine, word: &str| {
      let text = life.word(word, vec![]).unwrap().as_ref().as_str().unwrap().to_owned();
      text[..text.len() - 1].to_owned()
    };
    let (tip, asked) = (read(&mut crossing, turns), read(&mut crossing, clocks));
    drop(crossing);
    for keeps in [false, true] {
      let open = Opening { inspecting: Some(!keeps), ..opening(&at, keeps) };
      let mut later = open.boot::<String>([]).unwrap_or_else(|no| panic!("{text} {keeps}: {no}")).0;
      // A later life that keeps the record says more after the tip, as the clock that its stand
      // asks.
      let seen = read(&mut later, turns);
      assert!(seen.starts_with(&tip), "{text} {keeps}: the open stands at the tip");
      let seen = read(&mut later, clocks);
      assert!(seen.starts_with(&asked), "{text} {keeps}: the clocks {seen} begin with {asked}");
    }
  }
}

#[test]
fn a_life_goes_on_across_a_crossing_with_the_names_the_module_and_the_open_prompt_of_each_chain() {
  let at = yard("carry");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nran = await bash('echo one')\nqueue = [ran]\nSIZE = 6\ndef f():\n  return SIZE\n\
              waiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  crossing(&at);
  let mut later =
    scripted(&at, &["rounds += 1\nclose(f'round {rounds}')"]).boot::<String>([]).unwrap().0;
  let word = format!(
    "wake('chain2')\nclose([str(ran), queue, f(), get('{waiting}')[:2], await waiting, str(chain('more')), str(bash('echo two'))])"
  );
  let exit = "Exit(code=0, stdout=Text(path='bash1/stdout', content='one\\n'), stderr=Text(path='bash1/stderr', content=''))";
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[{exit:?}, [{exit}], 6, ('prompt', '{waiting}'), 'round 2', 'chain3', 'bash2']"),
    "each name, value and answer of the past stands, the model starts the cut word again in a turn on the module \
     that the carry holds and closes the open prompt, and a new act takes the next name"
  );
  assert_eq!(
    closed(&mut later, "close([x[1] for x in transcript('chain1', 'merged')])").as_ref().py_repr(),
    "['merged2']",
    "a name that an ear of the outside took in the past stays taken"
  );
  drop(later);
  let builds = builds(&at.join("record.jsonl"));
  assert_eq!(
    builds.iter().filter(|one| **one == world::engine()).count(),
    1,
    "the build that goes on names itself once"
  );
  let mut again = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut again, "close([f(), str(chain('last'))])").as_ref().py_repr(),
    "[6, 'chain4']",
    "a later life replays the segment after the crossing"
  );
}

#[test]
fn a_life_born_on_a_carry_binds_the_function_whose_default_reads_a_tuple_of_constants_each_value_and_each_life_word()
 {
  let at = yard("tuple-default");
  let mut first = opening(&at, true).boot::<String>([]).unwrap().0;
  let word = "EVERY, FLOOR = 300.0, 900.0\ndef desk(every: float = EVERY, floor: float = FLOOR):\n  return every + floor\n\
              limit = 7\nclose(desk())";
  assert_eq!(closed(&mut first, word).as_ref().as_float(), Some(1200.0));
  drop(first);
  crossing(&at);
  let mut later = opening(&at, true).boot::<String>([]).unwrap().0;
  let kinds = "sorted({x[0] for x in transcript('chain1') if x[0] in ('catchup', 'remember', 'compacted', 'fitted')})";
  assert_eq!(
    closed(&mut later, &format!("close([desk(), desk(1.0), EVERY, FLOOR, limit, {kinds}])"))
      .as_ref()
      .py_repr(),
    "[1200.0, 901.0, 300.0, 900.0, 7, ['catchup', 'compacted', 'fitted', 'remember']]",
    "the born life holds the function, each value and each life word of the chain, whatever the function read when it was made"
  );
}

#[test]
fn a_crossing_of_a_later_life_of_a_record_opens_with_no_drift_and_seeds_the_root_once() {
  let at = yard("crossing-relife");
  let open = || Opening { skills: Some(true), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  let mut first = open();
  assert_eq!(closed(&mut first, "n = 1\nclose(n)").as_ref().as_int(), Some(1));
  assert_eq!(seeds(&at), 1, "the first life of a record seeds the root once");
  drop(first);
  crossing(&at);
  for n in 2..=4 {
    let mut later = open();
    let word = "n = n + 1\nclose([n, len([x for x in transcript('chain1', 'setup')])])";
    assert_eq!(
      closed(&mut later, word).as_ref().py_repr(),
      format!("[{n}, 1]"),
      "each later life goes on across the crossing after the life before it, and holds the one setup of the root"
    );
    assert_eq!(seeds(&at), 0, "a life that goes on across a crossing runs no seed of the root");
    drop(later);
    crossing(&at);
  }
}

#[test]
fn a_large_value_and_an_exit_come_back_from_the_carry_at_each_later_open_and_no_rung_of_the_seed_raises()
 {
  let at = yard("carry-large");
  let open = || scripted(&at, &[]).boot::<String>([]).unwrap().0;
  let mut first = open();
  let word = "big = 'x' * LARGEST\nran = Exit(0, Text('o', 'y' * LARGEST), Text('e', ''))\nsmall = 1\nclose(small)";
  assert_eq!(closed(&mut first, word).as_ref().as_int(), Some(1));
  drop(first);
  crossing(&at);
  for _ in 0..2 {
    let mut later = open();
    let read = "close([len(big), len(ran.stdout.content), small, repr(raised)])";
    assert_eq!(
      closed(&mut later, read).as_ref().py_repr(),
      "[524288, 524288, 1, 'None']",
      "the definitions make no value again, so each comes back from the carry"
    );
    drop(later);
    crossing(&at);
  }
}

#[test]
fn a_record_whose_tail_holds_a_kind_of_fact_that_this_build_does_not_know_opens_with_each_chain_and_its_prompts()
 {
  let at = yard("record-kind");
  let waiting = crossed(&at);
  reopened(
    &at,
    &waiting,
    "the first life after the crossing goes on across it and asks the open prompt again",
  );
  let later = format!(
    "[[\"made\",\"chain1\",\"store\",\"{}\",null,\"{}.9\",\"9.9.9\"]]",
    "0".repeat(64),
    world::RECORD_SCHEMA.major
  );
  let unknown = [
    later.as_str(),
    "[[\"glimpse\",\"chain1\",\"store\",{\"seen\":1}]]",
    "[[\"glimpse\",\"glimpse1\",\"store\",\"chain1\",\"later\"]]",
    "[[\"done\",\"glimpse1\",\"store\",\"seen\"]]",
  ];
  let path = at.join("record.jsonl");
  let text = fs::read_to_string(&path).unwrap();
  fs::write(&path, format!("{text}{}\n", unknown.join("\n"))).unwrap();
  reopened(
    &at,
    &waiting,
    "a later build of the major made the tail, and this build skips the kind that it does not know",
  );
}

#[test]
fn a_record_of_another_major_of_its_schema_is_refused_with_both_schemas_and_keeps_nothing() {
  let at = yard("record-major");
  crossed(&at);
  let ours = format!(",\"{}\",", world::RECORD_SCHEMA);
  let theirs = format!(",\"{}.0\",", world::RECORD_SCHEMA.major + 1);
  changed(&at, "[[\"made\",", |line| line.replace(&ours, &theirs));
  let text = fs::read_to_string(at.join("record.jsonl")).unwrap();
  let refused = scripted(&at, &[]).boot::<String>([]).unwrap_err();
  assert_eq!(
    refused.message(),
    format!(
      "the record holds the schema {}.0, of another major than the schema {} that this build writes: open it with a \
       build that writes the major {}, or start a new life",
      world::RECORD_SCHEMA.major + 1,
      world::RECORD_SCHEMA,
      world::RECORD_SCHEMA.major + 1
    )
  );
  assert_eq!(fs::read_to_string(at.join("record.jsonl")).unwrap(), text, "it keeps nothing");
}

#[test]
fn a_later_life_on_a_record_that_crossed_boots_on_the_carry_and_the_last_segment_alone() {
  let at = yard("carry-live");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  closed(&mut first, "await bash('echo kept')\nclose(None)");
  let word = "work = chain('work')\nSIZE = 6\ndef f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\n\
              await wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  let turns = "close(len(turns('chain2')))";
  let told = closed(&mut first, turns).as_ref().as_int();
  drop(first);
  crossing(&at);
  let mut second = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(closed(&mut second, "after = f() + 1\nclose(after)").as_ref().as_int(), Some(7));
  drop(second);
  let tail = segment(&at);
  let carried = 1;
  let again = "rounds += 1\nclose(f'round {rounds}')";
  let (mut later, booted) = scripted(&at, &[again]).boot::<String>([]).unwrap();
  assert_eq!(
    booted.len(),
    carried + tail.len(),
    "a later life boots on the carry and the last segment alone"
  );
  assert_eq!(booted[carried..], tail);
  assert!(closed(&mut later, turns).as_ref().as_int() >= told, "each chain keeps its turns");
  let word = format!(
    "wake('chain2')\nclose([f(), after, get('{waiting}')[:2], peek('bash1').stdout.content, await Act('{waiting}')])"
  );
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[6, 7, ('prompt', '{waiting}'), 'kept\\n', 'round 2']"),
    "the module of each chain and its open prompt go on, with what the tail added, and a command that no value names \
     keeps its outcome"
  );
}

#[test]
fn a_command_that_ran_at_a_crossing_ends_in_the_life_that_goes_on_across_it_as_interrupted() {
  let at = yard("crossing-running");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  let with = verbs::Bash { on: Some("chain1".to_owned()), ..Default::default() };
  drop(first.bash("sleep 60", with).unwrap());
  drop(first);
  crossing(&at);
  let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert!(later.pending().unwrap().is_empty(), "the carry left nothing to wake");
  assert_eq!(
    closed(&mut later, "close(str(peek('bash1')))").as_ref().as_str(),
    Some("bash1 was interrupted: the life that ran it ended before it did"),
    "the command ends as the ear of commands ends one that a life left"
  );
}

#[test]
fn a_word_that_awaits_a_command_at_a_crossing_ends_as_cut_and_its_model_starts_it_again_in_a_turn()
{
  let at = yard("crossing-cut-word");
  let work = "await bash('sleep 60')\nclose('late')";
  let mut first = scripted(&at, &[work]).boot::<String>([]).unwrap().0;
  let asked = "waiting = prompt(str, 'Wait.', on=chain('work'))\nawait wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, asked).as_ref().as_str().unwrap().to_owned();
  let open = "close([x[1] for x in transcript('chain2', 'run') if not over(x[1])])";
  let runs = closed(&mut first, open).as_ref().py_repr();
  let run = runs.trim_start_matches("['").trim_end_matches("']").to_owned();
  assert!(
    run.starts_with("run") && !run.contains(','),
    "the word of chain2 awaits its command: {runs}"
  );
  drop(first);
  crossing(&at);
  let mut later = scripted(&at, &["close('again')"]).boot::<String>([]).unwrap().0;
  let again = format!(
    "wake('chain2')\nclose([await Act('{waiting}'), str(peek('bash1')), \
     any(\"it cut ['{run}', 'bash1']\" in t[1] for t in turns('chain2'))])"
  );
  assert_eq!(
    closed(&mut later, &again).as_ref().py_repr(),
    "['again', 'bash1 was interrupted: the life that ran it ended before it did', True]",
    "the command ends as interrupted, the chain is told that the carry cut the word, and the model of the \
     chain starts the word again in a turn that it reads"
  );
  drop(later);
  let ran = segment(&at)
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .filter(|a| {
      (a.kind() == "started" && a.about() == "bash1")
        || (a.kind() == "rung" && a.word(1).and_then(|one| one.as_str()) == Some(work))
    })
    .count();
  assert_eq!(ran, 0, "no word that ran at the crossing runs again, nor its command");
}

#[test]
fn a_life_that_keeps_no_record_does_not_cross() {
  let at = yard("crossing-unkept");
  let life = Opening { keeps: Some(false), ..scripted(&at, &[]) }.boot::<String>([]).unwrap().0;
  assert_eq!(life.cross().unwrap_err().message(), "a life that keeps no record does not cross");
}

#[test]
fn the_log_of_a_life_after_a_crossing_holds_the_facts_of_the_lives_before_it_and_agrees_with_its_record()
 {
  let at = yard("crossing-log");
  let record = at.join("record.jsonl");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  let before = "before = 'said before'\nclose(before)";
  closed(&mut first, before);
  drop(first);
  crossing(&at);
  for word in ["close('said after')", "close('said last')"] {
    let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
    closed(&mut later, word);
    drop(later);
    let facts = world::logged(&record).unwrap().expect("the log agrees with its record");
    let words: Vec<String> = facts
      .iter()
      .filter_map(|one| {
        let items = one.as_ref().items()?;
        (items.first()?.as_str()? == "rung")
          .then(|| items.get(4)?.as_str().map(str::to_owned))
          .flatten()
      })
      .collect();
    for said in [before, word] {
      assert_eq!(words.iter().filter(|one| *one == said).count(), 1, "{said}");
    }
  }
}

#[test]
fn a_chain_that_only_a_value_names_goes_on_across_a_crossing_with_its_own_module() {
  let at = yard("crossing-named");
  let mut first = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  closed(&mut first, "kids = [chain('kid')]\nawait rung('n = 5', on=kids[0])\nclose(None)");
  drop(first);
  crossing(&at);
  let mut later = scripted(&at, &[]).boot::<String>([]).unwrap().0;
  assert_eq!(
    closed(&mut later, "close([kids, module(kids[0]).get('n')])").as_ref().py_repr(),
    "[['chain2'], 5]",
    "the carry holds a chain that a value of a module names, with the values of its own module"
  );
}

#[test]
fn a_word_that_ran_at_a_crossing_is_cut_with_the_prompt_that_it_awaited() {
  let at = yard("crossing-asked");
  let work = "got = await prompt(str, 'Ask.', on=chain('asked'))\nclose(got)";
  let mut first = scripted(&at, &[work, "await wait(3600, __name__)\nclose('late')"])
    .boot::<String>([])
    .unwrap()
    .0;
  let again = "waiting = prompt(str, 'Wait.', on=chain('work'))\nawait wait(0.5, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, again).as_ref().as_str().unwrap().to_owned();
  drop(first);
  crossing(&at);
  let mut later = scripted(&at, &["close('fresh')"]).boot::<String>([]).unwrap().0;
  let again = format!(
    "wake('chain2')\ngot = await Act('{waiting}')\nclose([got, [x[1] for c in ['chain1', 'chain2', 'chain3', 'chain4'] for x in transcript(c, 'prompt') if not over(x[1])], str(chain('more'))])"
  );
  assert_eq!(
    closed(&mut later, &again).as_ref().py_repr(),
    "['fresh', [], 'chain4']",
    "the model starts the cut word again in a new turn, no word runs again to make a chain, and the prompt that the \
     cut word awaited at the crossing is not carried"
  );
}

#[test]
fn a_record_that_another_engine_went_on_with_after_a_crossing_replays_its_tail_to_the_exact_state_at_its_tip()
 {
  let at = yard("crossed");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose(f'round {rounds}')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nqueue = [1]\nSIZE = 6\ndef f():\n  return SIZE\n\
              waiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3, __name__)\nclose(str(waiting))";
  closed(&mut first, word);
  drop(first);
  crossing(&at);
  let mut tail =
    scripted(&at, &["await wait(3600, __name__)\nclose('late')"]).boot::<String>([]).unwrap().0;
  let word = "after = f() + 1\nqueue.append(after)\nmore = chain('more')\nasked = prompt(str, 'Ask.', on=more)\n\
              shell = bash('echo tail')\nawait shell\nawait wait(0.3, __name__)\nclose(str(asked))";
  let asked = closed(&mut tail, word).as_ref().as_str().unwrap().to_owned();
  drop(tail);
  // The facts after the crossing name another engine, as a build of another engine names them.
  let path = at.join("record.jsonl");
  let text = fs::read_to_string(&path).unwrap();
  let cut = text.rfind(&ours()).unwrap();
  let theirs = text[cut..].replacen(&ours(), &made_of(&"0".repeat(64)), 1);
  fs::write(&path, format!("{}{theirs}", &text[..cut])).unwrap();
  let (mut later, booted) = scripted(&at, &["close('three')"]).boot::<String>([]).unwrap();
  assert!(booted.len() > 1, "the life replays the tail, word by word");
  let word = format!(
    "close([f(), after, queue, more, shell, peek(shell).stdout.content, get('{asked}')[:2], str(chain('last'))])"
  );
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[6, 7, [1, 7], 'chain3', 'bash1', 'tail\\n', ('prompt', '{asked}'), 'chain4']"),
    "each name that the tail bound holds its value, each act that the tail made goes on, and a new act takes the next \
     name"
  );
  drop(later);
  let kept = world::kept(&path).unwrap();
  let carries = kept.iter().filter(|one| {
    entry(&one.as_ref(), 0).and_then(Fact::of).is_some_and(|a| a.kind() == CHECKPOINT)
  });
  assert_eq!(carries.count(), 0, "the record keeps no carry");
  fs::remove_file(format!("{}.log", path.display())).unwrap();
  let told = world::retold(&path).unwrap();
  assert!(!told.logged, "the reader reads the record alone");
  let facts: Vec<String> = told.facts().unwrap().iter().map(|one| one.as_ref().py_repr()).collect();
  let heads = |kind: &str, of: &str| -> Vec<&String> {
    facts.iter().filter(|one| one.starts_with(&format!("['{kind}', '{of}', "))).collect()
  };
  assert!(
    heads("chain", "chain3").iter().any(|one| one.contains("'more'")),
    "the reader shows the chain that a word of the tail made, with its label: {facts:?}"
  );
  assert!(
    heads("prompt", &asked).len() == 1 && heads("done", &asked).is_empty(),
    "the reader shows the prompt that the word made on it, open: {facts:?}"
  );
}

#[test]
fn a_life_reads_the_classes_of_the_config_directory_that_it_opens_on_and_none_of_the_machine() {
  let at = yard("classes");
  fs::create_dir_all(at.join("config")).unwrap();
  fs::write(at.join("config/config.json"), r#"{"classes": {"claude-cli:opus": "cheap"}}"#).unwrap();
  let opening = Opening {
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    skills: Some(false),
    ..opening(&at, false)
  };
  let (mut engine, _) = opening.boot::<String>([]).unwrap();
  let standing = engine.standing().unwrap().py_repr();
  // The catalog gives opus the class dear, so only the config of this directory makes it cheap.
  assert!(standing.starts_with("[[['claude-cli:opus', ["), "{standing}");
  assert!(standing.contains("1000000, 'cheap']"), "{standing}");
}

#[test]
fn an_open_tells_each_stage_in_order_and_ends_with_each_entry_of_its_boot_counted() {
  let at = yard("stages");
  let mut first = opening(&at, true).boot::<String>([]).unwrap().0;
  closed(&mut first, "x = 1\nclose(x)");
  drop(first);
  crossing(&at);
  let mut tail = opening(&at, true).boot::<String>([]).unwrap().0;
  closed(&mut tail, "y = x + 1\nclose(y)");
  drop(tail);
  let bytes = fs::metadata(at.join("record.jsonl")).unwrap().len();
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = Arc::clone(&told);
  let progress =
    Progress(Arc::new(move |stage, done, of| heard.lock().unwrap().push((stage, done, of))));
  let reactor = runtime::Builder::new_current_thread().enable_all().build().unwrap();
  let later =
    reactor.block_on(opening(&at, true).open::<String>([], progress, &mut |_| true, None)).unwrap();
  let told = told.lock().unwrap().clone();
  assert_eq!(told.first(), Some(&(Stage::Find, 0, bytes)), "the open tells its first stage first");
  let mut stages: Vec<Stage> = told.iter().map(|one| one.0).collect();
  stages.dedup();
  // The boot takes the entries that the stage find read, so no stage reads the record again.
  assert_eq!(stages, [Stage::Find, Stage::Seed, Stage::Boot]);
  let back = |pair: &[(Stage, u64, u64)]| {
    pair[0].0 == pair[1].0 && (pair[1].1 < pair[0].1 || pair[1].2 < pair[0].2)
  };
  assert!(!told.windows(2).any(back), "no number of a stage goes back: {told:?}");
  let last = |stage| told.iter().rev().find(|one| one.0 == stage).map(|one| (one.1, one.2));
  assert_eq!(last(Stage::Find), Some((bytes, bytes)), "the scan passed each byte of the record");
  assert_eq!(last(Stage::Seed), Some((0, 0)), "the seed counts nothing");
  let (done, of) = last(Stage::Boot).unwrap();
  assert!(of > 0 && done == of, "the boot ends with each entry counted: {done} of {of}");
  let within = |one: &&(Stage, u64, u64)| one.0 == Stage::Boot && 0 < one.1 && one.1 < of;
  assert!(told.iter().any(|one| within(&one)), "the replay counts entries as it passes them");
  drop(later);
}

/// A life whose image stands at the tip of its record keeps that image as it is: the end of a life
/// that said nothing after its last image writes none again.
#[test]
fn a_life_whose_image_stands_at_its_tip_writes_none_again() {
  let at = yard("image-again");
  let record = at.join("record.jsonl");
  let mut life = opening(&at, true).boot::<String>([]).unwrap().0;
  closed(&mut life, "x = 1\nclose(x)");
  assert!(image::keep(&mut life, &record, &()).unwrap(), "the life writes its image");
  // A tab in the spaces after the key marks this file and changes no value of it: a new write of
  // the image pads its key with spaces.
  let path = at.join("record.jsonl.image");
  let mut bytes = fs::read(&path).unwrap();
  let end = bytes.iter().position(|one| *one == b'\n').unwrap();
  assert_eq!(bytes[end - 1], b' ', "the key stands in spaces");
  bytes[end - 1] = b'\t';
  fs::write(&path, &bytes).unwrap();
  assert!(image::agrees(&record), "the marked image still agrees");
  assert!(image::keep(&mut life, &record, &()).unwrap(), "the record has its image at its tip");
  assert!(fs::read(&path).unwrap() == bytes, "the image at the tip is not written again");
  closed(&mut life, "close(x + 1)");
  assert!(image::keep(&mut life, &record, &()).unwrap(), "the life writes its new image");
  assert!(fs::read(&path).unwrap() != bytes, "an image of a later tip is written");
}

/// An open whose image does not load, as one whose part of the host the host does not take, opens
/// by its record on what it waited for: it tells no stage again and waits for its catalog once.
#[test]
fn an_open_whose_image_does_not_load_tells_no_stage_again() {
  let at = yard("stages-image");
  let mut first = opening(&at, true).boot::<String>([]).unwrap().0;
  closed(&mut first, "x = 1\nclose(x)");
  assert!(image::keep(&mut first, &at.join("record.jsonl"), &()).unwrap(), "the life has an image");
  drop(first);
  let told = Arc::new(Mutex::new(Vec::new()));
  let heard = Arc::clone(&told);
  let progress =
    Progress(Arc::new(move |stage, done, of| heard.lock().unwrap().push((stage, done, of))));
  let reactor = runtime::Builder::new_current_thread().enable_all().build().unwrap();
  let (mut later, replayed) = reactor
    .block_on(opening(&at, true).open::<String>([], progress, &mut |_| false, None))
    .unwrap();
  assert!(!replayed.is_empty(), "the life opens by its record");
  assert_eq!(closed(&mut later, "close(x)").as_ref().py_repr(), "1");
  let told = told.lock().unwrap().clone();
  let mut stages: Vec<Stage> = told.iter().map(|one| one.0).collect();
  stages.dedup();
  assert!(stages.is_sorted_by(|a, b| a < b), "no stage comes again: {told:?}");
  let starts = told.iter().filter(|one| **one == (Stage::Find, 0, told[0].2)).count();
  assert_eq!(starts, 1, "the open waits for its catalog once: {told:?}");
}

/// A life on the image of an earlier build, whose engine alone differs, opens on the texts that
/// the image holds, with no word, then moves to this build as a step of its own: the fact made of
/// this build, across which the next open carries it and tells each chain the lines of the engine
/// that differ, after the turns that it had, after which its next word runs on this build.
#[test]
fn a_life_opens_on_the_image_of_another_engine_and_then_moves_to_this_build() {
  let at = yard("image-moves");
  let (made, named) = elsewhere("crossed = True");
  let (record, _) = made_elsewhere(&at, &made);
  let reactor = runtime::Builder::new_current_thread().enable_all().build().unwrap();
  let open = || {
    let opening = Opening { skills: Some(false), ..opening(&at, true) };
    reactor
      .block_on(opening.open::<String>([], Progress::default(), &mut |_| true, None))
      .unwrap()
      .0
  };
  let read =
    |life: &mut Engine| closed(life, "close([x, globals().get('crossed')])").as_ref().py_repr();
  let mut first = open();
  assert!(first.crosses(), "a life opens on the texts of its record");
  assert_eq!(read(&mut first), "[1, True]");
  assert!(image::keep(&mut first, &record, &()).unwrap(), "the life writes its image");
  drop(first);
  let mut shown = open();
  assert!(shown.crosses() && image::agrees(&record), "the life opens on its image");
  let before = shown.word("turns(on='chain1')", vec![]).unwrap();
  assert!(world::crossings(&record).unwrap().is_empty(), "the open writes nothing");
  shown.cross().unwrap();
  assert_eq!(world::crossings(&record).unwrap().len(), 1, "the move writes one fact made");
  assert_eq!(builds(&record), [named, world::engine()], "the move ends with this build");
  let mut moved = open();
  assert!(!moved.crosses(), "the life runs this build");
  // Each turn stays, and the last one goes on with the lines of the engine that differ.
  let kept = "(t := turns(on='chain1'))[:-1] == __b[:-1] and t[-1][1].startswith(__b[-1][1]) \
              and t[-1][1][len(__b[-1][1]):].startswith('\\n\\n#chain1 crossed\\n')";
  let kept = moved.word(kept, vec![("__b", before)]).unwrap();
  assert_eq!(kept.as_ref().py_repr(), "True", "the move appends the engine that differs");
  assert_eq!(read(&mut moved), "[1, None]", "the next word runs on this build");
}

/// A wait that a life left open takes its time again in the life that resumes its record: the ear
/// of the kits hears it first, answers no such act and leaves it to the ear of the time.
#[test]
fn a_wait_that_a_life_left_open_goes_on_in_the_life_that_resumes_its_record() {
  let at = yard("wait-left-open");
  let (mut first, _) = opening(&at, true).boot::<String>([]).unwrap();
  let on = Some("chain1".to_owned());
  let wait =
    first.wait(&["chain1"], verbs::Wait { seconds: Some(0.05), on }).unwrap().id().to_owned();
  drop(first);
  let (mut later, _) = opening(&at, true).boot::<String>([]).unwrap();
  later.resume().unwrap();
  let came = executor::block_on(Act::<Object>::of(&mut later, &wait));
  assert_eq!(came.map(|one| one.py_repr()).map_err(|no| no.to_string()), Ok("None".to_owned()));
}

/// The kind of each fact of a record.
fn kinds(record: &[Object]) -> Vec<String> {
  let fact = |one: &Object| entry(&one.as_ref(), 0).and_then(Fact::of).unwrap().kind().to_owned();
  record.iter().map(fact).collect()
}

/// The words that the ear of the extensions plays on a new chain.
const PLAYED: &str =
  "[x[4].split(chr(10))[0] for x in transcript(chain('two'), 'rung') if x[2] == 'extensions']";

/// An ear of the host that takes a read of a path of its own, which the files refuse.
fn mine() -> Box<dyn Ear> {
  ear(|mut co| async move {
    loop {
      let a = hear(&mut co).await;
      let path = a.word(1).and_then(|one| one.as_str().map(str::to_owned)).unwrap_or_default();
      if a.question() && a.kind() == "read" && path.starts_with("mine://") {
        say(&mut co, Fact::says("done", a.about(), [Text::new(path, "mine\n").object()]));
      }
    }
  })
}

/// The standing that a stand on the root gives, once the World answered it, or why it refused it.
fn stood(engine: &mut Engine) -> Result<Object, Fault> {
  let stand = engine.stand(verbs::Stand { on: Some("chain1".to_owned()) })?;
  executor::block_on(stand)
}

/// The engine that each fact made of a record names, in its order.
fn builds(record: &Path) -> Vec<String> {
  let kept = world::kept(record).unwrap();
  let fact = |one: &Object| entry(&one.as_ref(), 0).and_then(Fact::of).unwrap();
  let named = |one: Fact| one.word(0).and_then(|word| word.as_str().map(str::to_owned));
  kept.iter().map(fact).filter(|one| one.kind() == world::MADE).filter_map(named).collect()
}

/// A record of one rung that a life of a test kept, and the text of the record with its fact made
/// changed.
fn made_elsewhere(at: &Path, made: &str) -> (PathBuf, String) {
  let record = at.join("record.jsonl");
  let mut first =
    Opening { skills: Some(false), ..opening(at, true) }.boot::<String>([]).unwrap().0;
  closed(&mut first, "x = 1\nclose(x)");
  drop(first);
  let ours = ours();
  let text = fs::read_to_string(&record).unwrap();
  assert!(text.contains(&ours), "the record names this build");
  let text = text.replace(&ours, made);
  fs::write(&record, &text).unwrap();
  (record, text)
}

/// The system prompt of each process of the claude command line of a test, in the order that they
/// started.
#[cfg(unix)]
fn systems_read(at: &Path) -> Vec<String> {
  let mut args: Vec<(usize, String)> = fs::read_dir(at)
    .unwrap()
    .filter_map(|one| {
      let path = one.ok()?.path();
      let n = path.file_name()?.to_str()?.strip_prefix("args.")?.parse().ok()?;
      Some((n, fs::read_to_string(path).ok()?))
    })
    .collect();
  args.sort();
  let file = |args: &str| {
    let args: Vec<&str> = args.split('\0').collect();
    let at = args.iter().position(|one| *one == "--system-prompt-file").expect("a system prompt");
    fs::read_to_string(args[at + 1]).unwrap()
  };
  args.iter().map(|(_, one)| file(one)).collect()
}

/// The opening of a life whose model is the claude command line of a test, which writes the words
/// that it was given beside the folder.
#[cfg(unix)]
fn faked(at: &Path) -> Opening {
  use std::os::unix::fs::PermissionsExt;
  let bin = at.join("claude");
  let fake = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/world/provider/claude/fake-claude.sh");
  fs::write(&bin, fs::read(fake).unwrap()).unwrap();
  fs::set_permissions(&bin, PermissionsExt::from_mode(0o755)).unwrap();
  fs::write(at.join("models"), "nova 100 low\n").unwrap();
  let roster = Some(vec!["claude-cli:nova".to_owned()].into());
  let claude = Some(bin.display().to_string());
  Opening { claude, roster, skills: Some(false), ..opening(at, false) }
}

/// The system prompt that the command line of the model of a life read, once a prompt asked it.
#[cfg(unix)]
fn system(at: &Path, life: &mut Engine) -> String {
  assert_eq!(closed(life, "close(await prompt(str, 'Say.'))").py_repr(), "'reply 1'");
  let args = fs::read_dir(at)
    .unwrap()
    .filter_map(|one| one.ok().map(|one| one.path()))
    .find(|one| one.file_name().is_some_and(|name| name.to_string_lossy().starts_with("args.")))
    .map(|one| fs::read_to_string(one).unwrap())
    .expect("the command line ran");
  let args: Vec<&str> = args.split('\0').collect();
  let file = args.iter().position(|one| *one == "--system-prompt-file").map(|at| args[at + 1]);
  fs::read_to_string(file.expect("a file of the system prompt")).unwrap()
}

/// The value of `FURB_IMAGE_AGAIN` in the run whose `HOME` and `TMPDIR` name no person.
const NAMELESS: &str = "nameless";

/// The run of this test that writes `tests/image` again, with a new empty `HOME` and a `TMPDIR`
/// that name no person.
fn image_again_nameless() {
  let (home, tmp) = (Path::new("/tmp/furb-home"), Path::new("/tmp/furb-tmp"));
  drop(fs::remove_dir_all(home));
  for one in [home, tmp] {
    fs::create_dir_all(one).unwrap();
  }
  let test = module_path!().split_once("::").unwrap().1;
  let name = format!(
    "{test}::an_image_that_an_earlier_build_wrote_opens_by_the_image_to_the_state_of_its_record"
  );
  let ran = process::Command::new(env::current_exe().unwrap())
    .args([name.as_str(), "--exact", "--nocapture"])
    .env("FURB_IMAGE_AGAIN", NAMELESS)
    .env("HOME", home)
    .env("TMPDIR", tmp)
    .status()
    .unwrap();
  assert!(ran.success(), "the run that writes tests/image again failed");
}

/// `tests/image` written again from `language.jsonl` of the corpus, by a life in `at`.
fn image_again(at: &Path, frozen: &Path) {
  let record = at.join("record.jsonl");
  let corpus = corpus().into_iter().find(|one| one.ends_with("next/language.jsonl")).unwrap();
  fs::copy(corpus, &record).unwrap();
  // The doctrine of the project wins over the checkout of dev mode, whose path names its owner.
  let doctrine = at.join(".furb").join("DOCTRINE.md");
  fs::create_dir_all(doctrine.parent().unwrap()).unwrap();
  fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("DOCTRINE.md"), doctrine).unwrap();
  let mut life = opening(at, true).boot::<String>([]).unwrap().0;
  assert!(image::keep(&mut life, &record, &()).unwrap(), "the life writes its image");
  drop(life);
  fs::create_dir_all(frozen).unwrap();
  for name in ["record.jsonl", "record.jsonl.image"] {
    fs::copy(at.join(name), frozen.join(name)).unwrap();
  }
}

/// Each run of each chain whose word no turn of that chain shows, as a python expression in the
/// names of the engine.
const UNSHOWN: &str = "[(c, x[1], x[4]) for c in [f'chain{n}' for n in range(1, 99) if get(f'chain{n}')] \
                       for x in transcript(c, 'run') if x[3] == c \
                       and not any(x[5] in unquoted(t[1]) for t in turns(on=c))]";

/// Each extension whose file the root does not show as the turn `rung(read('<file>', HIDDEN))` and
/// then the turn of its code alone, or whose definitions the module of the root lacks.
const UNSEEDED: &str = "[e[3] for e in transcript('chain1', 'enable') if e[4].strip() and (not any(t[:2] == \
                        ('assistant', f'rung(read({e[7]!r}, HIDDEN).content)') and turns(on='chain1')[n + 1][:2] == \
                        ('user', e[4]) for n, t in enumerate(turns(on='chain1'))) or not all(m in \
                        module('chain1') for m in re.findall(r'^(?:def|class) (\\w+)', e[4], re.M)))]";

/// The runs of the word of the seed of the root that the lives after the last crossing of a record
/// made.
fn seeds(at: &Path) -> usize {
  let seed = |a: &Fact| {
    a.kind() == "run"
      && a.word(2).and_then(|one| one.as_str()) == Some("debug(t\"{standing()[4]=}\")")
  };
  segment(at)
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .filter(seed)
    .count()
}

/// A later life on the record of [`crossed`] holds each chain, the turns of the root, the
/// module and the open prompt on its chain.
fn reopened(at: &Path, waiting: &str, why: &str) {
  let mut later = scripted(at, &[]).boot::<String>([]).unwrap().0;
  let word = format!(
    "close([f(), get('chain2')[4], len(turns(on='chain1')) > 1, get('{waiting}')[3:5], \
     isinstance(peek('{waiting}'), str) or not over('{waiting}')])"
  );
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    "[6, 'work', True, ('chain2', 'str'), True]",
    "{why}"
  );
}

#[test]
fn a_life_that_opens_on_a_record_that_a_stop_cut_asks_the_reply_again_and_ends_the_command_with_no_wake()
 {
  let at = yard("cut");
  let (asked, heard) = unbounded::<()>();
  let asked = Mutex::new(asked);
  // The first model writes a part of its turn, and the life stops while it writes the rest.
  let streams = Answer(Arc::new(move |_, told| {
    told("x = ", "");
    drop(asked.lock().unwrap().unbounded_send(()));
    Box::pin(future::pending())
  }));
  let first = Opening {
    roster: Some(vec!["claude-cli:opus".to_owned()].into()),
    answer: Some(streams),
    skills: Some(false),
    ..opening(&at, true)
  };
  let mut first = first.boot::<String>([]).unwrap().0;
  let on = Some("chain1".to_owned());
  drop(first.bash("sleep 30", verbs::Bash { on: on.clone(), ..Default::default() }).unwrap());
  let message = Some("go".to_owned());
  let prompt = verbs::Prompt { message, on, ..Default::default() };
  let prompt = first.prompt(Object::string("str"), prompt).unwrap().id().to_owned();
  {
    let mut heard = heard;
    let streamed = futures::StreamExt::next(&mut heard);
    let act = Act::<Object>::of(&mut first, &prompt);
    executor::block_on(select(act, streamed));
  }
  drop(first);
  let mut later = Opening { skills: Some(false), ..scripted(&at, &["close('again')"]) }
    .boot::<String>([])
    .unwrap()
    .0;
  assert!(
    later.pending().unwrap().is_empty(),
    "the open put the work that the stop cut to the outside"
  );
  let got = executor::block_on(Act::<Object>::of(&mut later, &prompt)).unwrap();
  assert_eq!(got.as_ref().as_str(), Some("again"), "the reply that the stop cut is asked again");
  let command = later.word("repr(peek('bash1'))", vec![]).unwrap();
  let command = command.as_ref().as_str().unwrap_or_default().to_owned();
  assert!(
    command.contains("interrupted"),
    "the command that the stop cut ends as interrupted: {command}"
  );
  let running =
    "[x[1] for x in transcript(ROOT) if x[0] == 'started' and x[1] != ROOT and not over(x[1])]";
  let running = later.word(running, vec![]).unwrap().as_ref().py_repr();
  assert_eq!(running, "[]", "no act stays running with no process");
}

/// A life born on a carry holds the teach and the enables of its root in the turns that the carry
/// holds, which no journal says again, so the ear of the extensions teaches nothing again at its
/// first stand, and enables nothing again.
#[test]
fn a_life_born_on_a_carry_teaches_and_enables_nothing_again() {
  let at = yard("taught");
  let open = || opening(&at, true).boot::<String>([]).unwrap().0;
  let mut first = open();
  let said = "close([len(transcript('chain1', 'teach')), len(transcript('chain1', 'enable'))])";
  let was = closed(&mut first, said).as_ref().py_repr();
  drop(first);
  crossing(&at);
  assert_eq!(closed(&mut open(), said).as_ref().py_repr(), was);
}

/// A birth makes each chain again and keeps its turns, so the turns of a chain hold its start once
/// at most, however many births come.
#[test]
fn each_chain_holds_its_start_once_at_most_through_births_in_a_row() {
  let at = yard("reborn-twice");
  let given = || Opening { skills: Some(false), ..opening(&at, true) };
  let mut first = given().boot::<String>([]).unwrap().0;
  closed(&mut first, "x = 1\nwork = chain('work')\nclose(x)");
  drop(first);
  for _ in 0..3 {
    crossing(&at);
    drop(given().boot::<String>([]).unwrap().0);
  }
  assert_eq!(world::crossings(at.join("record.jsonl")).unwrap().len(), 3);
  // The fourth crossing: the life that opens on it is born on the carry of each crossing before it.
  crossing(&at);
  let mut last = given().boot::<String>([]).unwrap().0;
  let starts = "close([len([x for x in transcript(c) if x[0] == 'started' and x[1] == c]) \
                for c in ['chain1', 'chain2']])";
  assert_eq!(closed(&mut last, starts).py_repr(), "[1, 1]");
}

#[test]
fn a_life_on_a_record_that_crossed_goes_on_with_the_standing_of_its_last_stand() {
  let at = yard("stood-crossing");
  let both = "close((standing()[2], sorted(a[0] for a in standing()[0])))";
  let given = Opening {
    skills: Some(false),
    roster: Some(vec!["claude-cli:opus".to_owned(), "claude-cli:sonnet".to_owned()].into()),
    actor: Some("claude-cli:sonnet/low".to_owned()),
    ..scripted(&at, &[])
  };
  let (mut first, _) = given.boot::<String>([]).unwrap();
  let had = closed(&mut first, both).py_repr();
  drop(first);
  // No stand follows the crossing, and the last stand before it gave the standing.
  crossing(&at);
  let unsaid = Opening { skills: Some(false), roster: None, actor: None, ..scripted(&at, &[]) };
  let (mut later, _) = unsaid.boot::<String>([]).unwrap();
  assert_eq!(closed(&mut later, both).py_repr(), had);
}

#[test]
fn a_carry_holds_data_alone_and_a_later_engine_seeds_a_life_from_it_with_no_act_run_again() {
  let at = yard("carry-data");
  let ran = "rounds = globals().get('rounds', 0) + 1\nawait wait(3600 if rounds == 1 else 0, __name__)\nclose('late')";
  let mut first = scripted(&at, &[ran]).boot::<String>([]).unwrap().0;
  let word = "work = chain('work')\nshell = bash('echo one')\nawait shell\nqueue = [shell]\nSIZE = 6\n\
              def f():\n  return SIZE\nwaiting = prompt(str, 'Wait.', on=work)\nawait wait(0.3, __name__)\nclose(str(waiting))";
  let waiting = closed(&mut first, word).as_ref().as_str().unwrap().to_owned();
  drop(first);
  crossing(&at);
  let mut later = scripted(&at, &["close('late')"]).boot::<String>([]).unwrap().0;
  let word = format!(
    "wake('chain2')\nclose([[c for c in ['chain1', 'chain2', 'chain3'] if get(c)], await queue[0], f(), \
     get('{waiting}')[:2], await Act('{waiting}'), str(bash('echo two'))])"
  );
  let exit = "Exit(code=0, stdout=Text(path='bash1/stdout', content='one\\n'), stderr=Text(path='bash1/stderr', content=''))";
  assert_eq!(
    closed(&mut later, &word).as_ref().py_repr(),
    format!("[['chain1', 'chain2'], {exit}, 6, ('prompt', '{waiting}'), 'late', 'bash2']"),
    "each chain, value, definition, open prompt and act that a value names stands, and a new act takes the \
     next name"
  );
  drop(later);
  let segment = segment(&at);
  let again = segment
    .iter()
    .filter_map(|one| entry(&one.as_ref(), 0).and_then(Fact::of))
    .filter(|a| a.kind() == "started" && a.about() == "bash1");
  assert_eq!(again.count(), 0, "no carried act runs again");
}
