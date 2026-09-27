//! Time: a reading of the clock, a chance drawn, and a wait, which ends when its time is up.

use std::{
  collections::HashMap,
  sync::mpsc::{self, RecvTimeoutError},
  thread,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

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
/// that a control ended first says nothing more.
pub fn time() -> Box<dyn Ear> {
  ear(|co, voice| async move {
    let mut due: HashMap<String, f64> = HashMap::new();
    let mut running: HashMap<String, mpsc::Sender<()>> = HashMap::new();
    loop {
      let a = hear(&co).await;
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
          let (stop, stopped) = mpsc::channel::<()>();
          let voice = voice.clone();
          let id = about.clone();
          thread::spawn(move || {
            // A wait past what the machine counts waits until a control ends it.
            let left = Duration::try_from_secs_f64((deadline - now()).max(0.0)).ok();
            let ended = match left {
              Some(left) => stopped.recv_timeout(left),
              None => stopped.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            if let Err(RecvTimeoutError::Timeout) = ended {
              voice.say("done", &id, [Object::none()]);
            }
          });
          running.insert(about, stop);
        }
        "due" => {
          if let Some(deadline) = a.word(0).and_then(|one| one.as_float()) {
            due.insert(about, deadline);
          }
        }
        "done" => {
          if running.remove(&about).is_some() {
            voice.hush(&about);
          }
          due.remove(&about);
        }
        _ => {}
      }
    }
  })
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
