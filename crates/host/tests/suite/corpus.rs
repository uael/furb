//! Each record of `tests/corpus` opens in a life of the host, on the ears of the host, as a window
//! opens it through the daemon: no fault and no drift ends it, and a record from before the stable
//! cut opens in no life, which says why.

#![expect(
  clippy::disallowed_methods,
  reason = "the tests block on the runtime that tokio starts for them"
)]

use std::{
  fs,
  path::{Path, PathBuf},
  time::Duration,
};

use furb::{life::Opening, world::unopened};
use furb_host::{Command, Life, LifeError, Reply};
use futures::StreamExt;
use tokio::time;

use crate::life::{place, scripted};

/// Each record of each folder of the corpus, by its folder and its name.
fn records() -> Vec<(String, PathBuf)> {
  let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus");
  let mut found = Vec::new();
  let read = |at: &Path| fs::read_dir(at).into_iter().flatten();
  for folder in read(&corpus).flatten().filter(|one| one.path().is_dir()) {
    for one in read(&folder.path()).flatten() {
      if one.path().extension().is_some_and(|ext| ext == "jsonl") {
        found.push((folder.file_name().to_string_lossy().into_owned(), one.path()));
      }
    }
  }
  found.sort();
  found
}

#[tokio::test]
async fn each_record_of_the_corpus_opens_on_the_ears_of_the_host_with_no_fault() {
  let found = records();
  assert!(found.iter().any(|one| one.0 == "next"), "tests/corpus/next holds no record");
  for (folder, one) in found {
    let name = format!("corpus-{folder}-{}", one.file_stem().unwrap().to_string_lossy());
    let at = place(&name);
    let record = at.join("record.jsonl");
    fs::copy(&one, &record).unwrap();
    let opening =
      Opening { record: Some(record.display().to_string()), skills: None, ..scripted(&at, &[]) };
    let unopened = unopened(&record);
    let life = Life::spawn(opening).unwrap();
    let mut events = Box::pin(life.events(0));
    // A command waits until the life opens, so the standing that the life gives is its open, which
    // a record that never crossed tells by no event of its own.
    let stands = time::timeout(Duration::from_secs(50), life.send(Command::Standing)).await;
    let Ok(stands) = stands else { panic!("{name} does not open in time") };
    if let Some(why) = unopened {
      // A record from before the stable cut opens in no life, which says why.
      assert!(
        matches!(&stands, Err(LifeError::Refused(no)) if *no == why.to_string()),
        "{name}: {stands:?}"
      );
    } else {
      assert!(matches!(stands, Ok(Reply::Value(_))), "{name} opens: {stands:?}");
      let ended = life.send(Command::End).await;
      assert!(matches!(ended, Ok(Reply::Done)), "{name} ends: {ended:?}");
    }
    while events.next().await.is_some() {}
  }
}
