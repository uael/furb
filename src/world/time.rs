//! Time: a reading of the clock, a chance drawn, and a wait, which ends when its time is up.

use std::{
  collections::HashMap,
  future,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::{FutureExt, StreamExt, future::LocalBoxFuture, stream::FuturesUnordered};
use unsync::oneshot;

use crate::{
  ear::{Ear, ear, hear, say},
  fact::Fact,
  value::{Fault, Object},
};

/// The ear of time: it answers a clock with the seconds since the epoch and a chance with a number drawn, at least
/// zero and under one, and it takes a wait and says it done when its time is up.
///
/// It says when a wait it takes is due, as a fact of its own, which the record keeps since it comes from outside
/// and says again in a later life, so a wait that a wake starts again ends when it would have ended then. A wait
/// that a control ended first ends at once, and says nothing more.
pub fn time() -> Box<dyn Ear> {
  ear(|co| async move {
    let mut due: HashMap<String, f64> = HashMap::new();
    let mut stops: HashMap<String, oneshot::Sender<()>> = HashMap::new();
    let mut waits: FuturesUnordered<LocalBoxFuture<'static, Option<String>>> =
      FuturesUnordered::new();
    loop {
      let a = tokio::select! {
        biased;
        a = hear(&co) => a,
        Some(Some(id)) = waits.next() => {
          stops.remove(&id);
          say(&co, Fact::says("done", &id, [Object::none()])).await;
          continue;
        }
      };
      let about = a.about().to_owned();
      match a.kind() {
        "clock" if a.question() => {
          say(&co, Fact::says("done", &about, [Object::float(now())])).await;
        }
        "chance" if a.question() => {
          let drawn = drawn().map_or_else(|fault| fault.object(), Object::float);
          say(&co, Fact::says("done", &about, [drawn])).await;
        }
        "wait" if a.question() => {
          say(&co, Fact::says("started", &about, [])).await;
          let seconds =
            a.word(1).and_then(|one| one.as_float().or_else(|| one.as_int().map(|n| n as f64)));
          let deadline = match due.get(&about) {
            Some(deadline) => *deadline,
            None => {
              let deadline = now() + seconds.unwrap_or_default();
              say(&co, Fact::says("due", &about, [Object::float(deadline)])).await;
              deadline
            }
          };
          let (stop, stopped) = oneshot::channel();
          waits.push(waited(about.clone(), deadline, stopped).boxed_local());
          stops.insert(about, stop);
        }
        "due" => {
          if let Some(deadline) = a.word(0).and_then(|one| one.as_float()) {
            due.insert(about, deadline);
          }
        }
        "done" => {
          stops.remove(&about);
          due.remove(&about);
        }
        _ => {}
      }
    }
  })
}

/// A wait: its id once its deadline passed, or nothing when it was stopped first. A wait past what the machine
/// counts waits until it is stopped.
async fn waited(id: String, deadline: f64, stopped: oneshot::Receiver<()>) -> Option<String> {
  let left = Duration::try_from_secs_f64((deadline - now()).max(0.0)).ok();
  let up = async {
    match left {
      Some(left) => tokio::time::sleep(left).await,
      None => future::pending().await,
    }
  };
  tokio::select! {
    () = up => Some(id),
    _ = stopped => None,
  }
}

/// The seconds since the epoch, on the clock of the machine.
fn now() -> f64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |since| since.as_secs_f64())
}

/// A number drawn by the system, at least zero and under one.
fn drawn() -> Result<f64, Fault> {
  let bits = getrandom::u64().map_err(|no| Fault::refused(no.to_string()))?;
  Ok((bits >> 11) as f64 / (1u64 << 53) as f64)
}
