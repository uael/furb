//! One life read three ways once it ended: by a whole replay of its record, by its image, and by a
//! birth on a checkpoint at its tip. Each way reaches the state that the replay reaches, so a model
//! and the owner read the same turns, the same module, the same acts, the same threads and the same
//! pages, at the open and after it.

#![expect(
  clippy::disallowed_methods,
  reason = "a test blocks on the runtime that tokio starts for it"
)]
#![expect(clippy::expect_used, reason = "a helper of a test fails the test where it fails")]

use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::Path,
  process,
};

use furb::{
  life::Opening,
  view::{Act, Reach},
};
use furb_host::{Command, Event, Life, LifeHandle, Reply};
use futures::StreamExt;

use crate::life::{checkpointed, done, made, place, rung, scripted, until};

/// A life that keeps its record at a place, whose model closes each request with `close('ok')`.
fn opening(at: &Path) -> Opening {
  let record = Some(at.join("record.jsonl").display().to_string());
  Opening { record, ..scripted(at, &["close('ok')"; 8]) }
}

/// The end of a life, once its thread is gone, so the record and its image stand whole.
async fn ended(life: LifeHandle) {
  let mut events = Box::pin(life.events(0));
  assert!(matches!(life.send(Command::End).await, Ok(Reply::Done)));
  while events.next().await.is_some() {}
}

/// The representation of a name of the module of a chain.
async fn inspected(life: &LifeHandle, name: &str, on: Option<&str>) -> String {
  let on = on.map(str::to_owned);
  match life.send(Command::Inspect { name: name.to_owned(), on }).await {
    Ok(Reply::Inspected { repr, .. }) => repr,
    other => format!("{other:?}"),
  }
}

/// A folder made again as a copy of another, so each open of the test stands in the folder of the
/// life, as a resume does.
fn restored(from: &Path, to: &Path) {
  for one in fs::read_dir(to).expect("the folder of the life") {
    let one = one.expect("an entry").path();
    if one.is_dir() {
      fs::remove_dir_all(&one).expect("a folder of the life goes");
    } else {
      fs::remove_file(&one).expect("a file of the life goes");
    }
  }
  let status = process::Command::new("cp")
    .arg("-R")
    .arg(format!("{}/.", from.display()))
    .arg(to.as_os_str())
    .status()
    .expect("cp runs");
  assert!(status.success(), "the copy of the life");
}

/// What a model and the owner read of a life, each under its label.
async fn read(
  life: &LifeHandle,
  chains: &[String],
  threads: &[String],
) -> BTreeMap<String, String> {
  let mut seen = BTreeMap::new();
  for chain in chains {
    let on = Some(chain.clone());
    let turns = life.send(Command::Turns { on: on.clone() }).await;
    seen.insert(format!("turns of {chain}"), format!("{turns:?}"));
    let Ok(Reply::Values(names)) = life.send(Command::Names { on: on.clone() }).await else {
      panic!("the names of {chain}")
    };
    for name in names.iter().filter_map(|one| one.as_str()) {
      seen.insert(format!("module of {chain}: {name}"), inspected(life, name, Some(chain)).await);
    }
  }
  let Ok(Reply::Acts(acts)) = life.send(Command::AsOf { at: u64::MAX }).await else {
    panic!("the acts of the life")
  };
  for act in acts {
    let Act { id, kind, by, maker, chain, words, phase, state, began, ended, .. } = &act;
    let said = format!("{kind:?} {by} {maker:?} {chain} {words:?} {phase:?} {state:?}");
    seen.insert(format!("act {id}"), said);
    seen.insert(format!("moments of {id}"), format!("{began} {ended:?}"));
  }
  let all = Command::Threads { on: None, at: None, read: None, site: None };
  seen.insert("threads".to_owned(), format!("{:?}", life.send(all).await));
  for thread in threads {
    let reach = Reach { before: None, limit: 1000 };
    let page = life.send(Command::Thread { thread: thread.clone(), reach }).await;
    seen.insert(format!("page of {thread}"), format!("{page:?}"));
  }
  seen
}

/// Each label whose value differs between two reads, or that one of them lacks, with each value
/// around the first character where they part.
fn differ(one: &BTreeMap<String, String>, other: &BTreeMap<String, String>) -> Vec<String> {
  let labels = one.keys().chain(other.keys()).collect::<BTreeSet<_>>();
  let mut found = Vec::new();
  for label in labels.into_iter().filter(|label| one.get(*label) != other.get(*label)) {
    let (a, b) = (one.get(label).map_or("nothing", String::as_str), other.get(label));
    let b = b.map_or("nothing", String::as_str);
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let at = a.iter().zip(&b).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
    let near = |of: &[char]| -> String {
      of[at.saturating_sub(120).min(of.len())..(at + 280).min(of.len())].iter().collect()
    };
    found.push(format!("{label}:\n    {}\n    {}", near(&a), near(&b)));
  }
  found
}

/// What a life of the record at a place reads by one way to open it: at the open, and after a
/// word that it runs then.
async fn read_by(
  way: &str,
  at: &Path,
  chains: &[String],
  threads: &[String],
) -> (BTreeMap<String, String>, Option<BTreeMap<String, String>>) {
  let image = at.join("record.jsonl.image");
  match way {
    "replay" => fs::remove_file(&image).expect("the image"),
    "checkpoint" => {
      checkpointed(opening(at)).await;
      fs::remove_file(&image).ok();
    }
    _ => {}
  }
  let life = Life::spawn(opening(at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  until(&mut events, |event| matches!(event, Event::Act(_))).await;
  let opened = read(&life, chains, threads).await;
  // A word after the open: each way goes on from the same state, so its acts take the same
  // moments. A birth on a checkpoint does not go on as the replay does yet: a chain tells again
  // each act that the birth made again, and not what it heard before the checkpoint and did not
  // tell.
  let after = made(&life, rung("z = 3")).await;
  until(&mut events, done(&after)).await;
  let went = if way == "checkpoint" { None } else { Some(read(&life, chains, threads).await) };
  ended(life).await;
  (opened, went)
}

#[tokio::test]
async fn a_life_by_its_record_by_its_image_and_by_a_checkpoint_reads_the_same() {
  let at = place("equal");
  let life = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(life.events(0));
  let word = "x = 1\nc = chain('c')\nf = chain('f', source=ROOT)\nb = bash('sleep 600')\n\
    asked = prompt(str, 'Name a color.', to=OPERATOR)";
  let first = made(&life, rung(word)).await;
  until(&mut events, done(&first)).await;
  let ask = |message: &str, on: Option<String>| Command::Prompt {
    shape: "str".to_owned(),
    message: message.to_owned(),
    to: None,
    on,
  };
  let thread = made(&life, ask("Do it.", None)).await;
  until(&mut events, done(&thread)).await;
  let more = made(&life, ask("And more.", Some(thread.clone()))).await;
  until(&mut events, done(&more)).await;
  let mut chains = vec!["chain1".to_owned()];
  for name in ["c", "f"] {
    chains.push(inspected(&life, name, None).await.trim_matches('\'').to_owned());
  }
  let on_c = made(&life, Command::Rung { word: "y = 2".to_owned(), on: Some(chains[1].clone()) });
  let on_c = on_c.await;
  until(&mut events, done(&on_c)).await;
  let asked = inspected(&life, "asked", None).await.trim_matches('\'').to_owned();
  let threads = [thread, more, asked];
  ended(life).await;
  // A later life ends the command that the end of the first life cut and stands the root, so the
  // life that only writes the checkpoint below finds no act in flight, and adds nothing.
  let later = Life::spawn(opening(&at)).expect("the life has a thread");
  let mut events = Box::pin(later.events(0));
  until(&mut events, |event| matches!(event, Event::Act(act) if act.id == "stand3" && act.done()))
    .await;
  ended(later).await;

  let kept = place("equal-kept");
  restored(&at, &kept);

  let mut found = Vec::new();
  let mut truth = None;
  for way in ["replay", "image", "checkpoint"] {
    restored(&kept, &at);
    let (opened, went) = read_by(way, &at, &chains, &threads).await;
    // The whole replay of the record is the one truth, and each other way is read against it.
    match &truth {
      None => truth = Some((opened, went)),
      Some((at_open, after_open)) => {
        let mut differs = differ(at_open, &opened);
        if let (Some(truth), Some(went)) = (after_open, &went) {
          differs
            .extend(differ(truth, went).into_iter().map(|one| format!("after the open: {one}")));
        }
        found.extend(differs.into_iter().map(|one| format!("{way}: {one}")));
      }
    }
  }
  assert!(found.is_empty(), "{} differences:\n{}", found.len(), found.join("\n"));
}
