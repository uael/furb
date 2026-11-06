//! Time: a reading of the clock, a chance drawn, and a wait, which ends when its time is up.

use std::{
  collections::HashMap,
  future,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::{Stream, stream};

use crate::{
  ear::{Co, Ear, Next, hearing, say},
  fact::Fact,
  value::{Fault, Given, Object},
};
use tokio::time;

/// The ear of time: it answers a clock with the seconds since the epoch and a chance with a number drawn, at least
/// zero and under one, and it takes a wait and says it done when its time is up.
///
/// It says when a wait it takes is due, as a fact of its own, which the record keeps since it comes from outside
/// and says again in a later life, so a wait that a wake starts again ends when it would have ended then. A wait
/// that a control ended first ends at once, and says nothing more.
pub fn time() -> Box<dyn Ear> {
  hearing(&["clock", "chance", "wait", "due"], |mut co: Co<String>| async move {
    let mut due: HashMap<String, f64> = HashMap::new();
    loop {
      let a = match co.next().await {
        Next::Heard(a) | Next::Resumed(a) => a,
        Next::Worked(id) => {
          say(&mut co, Fact::says("done", &id, [Object::none()]));
          continue;
        }
      };
      let about = a.about().to_owned();
      match a.kind() {
        "clock" if a.question() => {
          say(&mut co, Fact::says("done", &about, [Object::float(now())]));
        }
        "chance" if a.question() => {
          let drawn = drawn().map_or_else(|fault| fault.object(), Object::float);
          say(&mut co, Fact::says("done", &about, [drawn]));
        }
        "wait" if a.question() => {
          say(&mut co, Fact::says("started", &about, []));
          let seconds = a.word(1).and_then(|one| one.seconds());
          let deadline = if let Some(deadline) = due.get(&about) {
            *deadline
          } else {
            let deadline = now() + seconds.unwrap_or_default();
            say(&mut co, Fact::says("due", &about, [Object::float(deadline)]));
            deadline
          };
          co.take(&about, waited(about.clone(), deadline));
        }
        "due" => {
          if let Some(deadline) = a.word(0).and_then(|one| one.as_float()) {
            due.insert(about, deadline);
          }
        }
        "done" => {
          due.remove(&about);
        }
        _ => {}
      }
    }
  })
}

/// A wait: its id once its deadline passed. A wait past what the machine counts waits until the
/// done of its act ends it.
fn waited(id: String, deadline: f64) -> impl Stream<Item = String> {
  let left = Duration::try_from_secs_f64((deadline - now()).max(0.0)).ok();
  stream::once(async move {
    match left {
      Some(left) => time::sleep(left).await,
      None => future::pending().await,
    }
    id
  })
}

/// The seconds since the epoch, on the clock of the machine.
fn now() -> f64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |since| since.as_secs_f64())
}

/// A number drawn by the system, at least zero and under one.
#[expect(
  clippy::cast_precision_loss,
  reason = "the 53 bits kept fit the mantissa of a float exactly"
)]
fn drawn() -> Result<f64, Fault> {
  let bits = getrandom::u64().map_err(|no| Fault::refused(no.to_string()))?;
  Ok((bits >> 11) as f64 / (1u64 << 53) as f64)
}
