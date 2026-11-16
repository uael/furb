//! `furb host`: the daemon of this machine, which holds each life by its record, at one place in
//! the cache of furb, as `furb-host` serves it. The file `host.json` of the config folder of furb
//! turns on the daemon on the tailnet, gives each operator of the network its role, and gives the
//! bound of a stall. The daemon reads the file again at each change of it, and each peer of the
//! tailnet that is connected follows the list at once: an operator that the file drops is refused
//! and loses its connection, an operator whose role changes holds the new role from its next verb,
//! and an operator that the file adds is let in at its next connection. A file that does not parse
//! keeps the list before.

use std::{
  path::{self, Path, PathBuf},
  time::Duration,
};

use futures::{SinkExt, StreamExt};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};
use serde_json::Value;
use tokio::time;
use tokio_util::codec::{Framed, LinesCodec};

use furb::{life::Opening, skill::Places, view::Phase, world};
use furb_host::{
  DaemonError, ProtocolError, STALL, STARTING, Settings, Tailnet, TransportError,
  access::Config,
  protocol::{Answer, BOUND, Frame, Hello, Kind, PROTOCOL, Request, Verb},
  run,
  transport::{Dialer, Local, LocalApi, Tailscale},
  upgrade,
};

use crate::error::CliError;

/// The daemon of this machine, served until a daemon of a new build takes its place, or, with
/// `upgrading`, served in the place of the daemon that runs, where a life with no holder lives
/// for `grace`. It says where it listens once it listens, so a program that starts it knows when
/// to dial it.
pub(crate) async fn serve(grace: Duration, upgrading: bool) -> Result<(), CliError> {
  let folder = folder()?;
  let door = Some(folder.join(DOOR));
  let path = Places::here().config.join("host.json");
  let config = Config::load(&path)?;
  let stall = config.stall.map_or(STALL, Duration::from_secs);
  let tailnet = tailnet(&path, config)?;
  let _followed = tailnet.as_ref().and_then(|one| {
    let said = |why: &str| eprintln!("furb host: {why}");
    followed(&path, one.identity.clone(), said)
      .inspect_err(|no| eprintln!("furb host: no watch of {}: {no}", path.display()))
      .ok()
  });
  let opening = Opening::default();
  let settings = Settings { opening, grace, first: STARTING, stall, tailnet, door, data: None };
  let local = Local::new(&folder);
  if upgrading {
    return Ok(upgrade(&local, settings).await?);
  }
  let listener = local.listen().await.map_err(DaemonError::from)?;
  println!("furb host listens in {}", folder.display());
  Ok(run(&local, listener, settings).await?)
}

/// The folder of the daemon of this machine in the cache of furb, where it listens.
fn folder() -> Result<PathBuf, CliError> {
  Ok(world::cache().ok_or(CliError::NoCache)?.join("host"))
}

/// What a message to the root of the life of a record came to, said through the daemon of this
/// machine as a window says it, so the life hears it at once, while it loops or waits, and the
/// daemon tells its end. None when no daemon listens, or when it holds no life of the record.
pub(crate) async fn said(
  record: &Path,
  shape: &str,
  message: String,
  to: &str,
) -> Result<Option<Value>, CliError> {
  let Some(mut lines) = dialed("furb prompt").await? else { return Ok(None) };
  let life = named(record);
  let to = (!to.is_empty()).then(|| to.to_owned());
  let prompt = Verb::Prompt { life: life.clone(), shape: shape.to_owned(), message, to, on: None };
  let id = match asked(&mut lines, 1, prompt).await {
    Ok(Answer::Act(id)) => id,
    Err(DaemonError::Refused(ProtocolError::Closed { .. })) => return Ok(None),
    Ok(_) => return Err(DaemonError::Unanswered.into()),
    Err(no) => return Err(no.into()),
  };
  let Answer::Acts(acts) = asked(&mut lines, 2, Verb::Ended { life, act: id.clone() }).await?
  else {
    return Err(DaemonError::Unanswered.into());
  };
  let done = acts.into_iter().find(|one| one.id == id).map(|one| one.phase);
  let Some(Phase::Done(value)) = done else { return Err(DaemonError::Unanswered.into()) };
  Ok(Some(value))
}

/// The life of a record opened in the daemon of this machine and pinned by the name of a task, so
/// it runs on with no window after the command ends, and each window lists it with what holds it.
/// An open of a life that the daemon holds already is no fault: the pin then holds it too.
pub(crate) async fn held(
  opening: &Opening,
  roster: Vec<String>,
  task: &str,
) -> Result<(), CliError> {
  let Some(record) = opening.record.as_deref() else { return Ok(()) };
  let Some(mut lines) = dialed(task).await? else { return Err(CliError::NoDaemon) };
  let life = named(Path::new(record));
  let roster = (!roster.is_empty()).then_some(roster);
  let open = Verb::Open {
    life: life.clone(),
    cwd: Some(opening.directory.clone()),
    actor: opening.actor.clone(),
    roster,
    skills: None,
    config: None,
    claude: None,
    answers: false,
    attach: None,
  };
  asked(&mut lines, 1, open).await?;
  asked(&mut lines, 2, Verb::Pin { life, holder: task.to_owned() }).await?;
  Ok(())
}

/// The end of the life of a record that the daemon of this machine holds, as a window ends it:
/// with `cancel`, each open work of the life is cancelled first. The daemon takes the end at once
/// and the life writes its image after, so the command returns once the daemon holds the life no
/// more, and a command after it finds the record free.
pub(crate) async fn ended(record: &Path, cancel: bool) -> Result<(), CliError> {
  let Some(mut lines) = dialed("furb end").await? else { return Err(CliError::NoDaemon) };
  let life = named(record);
  asked(&mut lines, 1, Verb::End { life: life.clone(), cancel }).await?;
  // The daemon tells the end of a life only to a window that holds it, so the command asks the
  // list again until the life leaves it, within the bound of a command.
  let gone = async {
    for id in 2.. {
      let Answer::Lives(listed) = asked(&mut lines, id, Verb::List { config: None }).await? else {
        return Err(DaemonError::Unanswered);
      };
      if !listed.iter().any(|one| one.life == life && one.held) {
        return Ok(());
      }
      time::sleep(Duration::from_millis(50)).await;
    }
    Ok(())
  };
  time::timeout(Duration::from_secs(60), gone).await.map_err(|_late| DaemonError::Unanswered)??;
  Ok(())
}

/// Each life that the daemon of this machine holds, one on each line: its record, then a window
/// when a window watches it, and the holder of each pin, so a life that only a pin holds, as a
/// life that an agent holds for a task, says what holds it. No daemon holds no life.
pub(crate) async fn lives() -> Result<Vec<String>, CliError> {
  let Some(mut lines) = dialed("furb lives").await? else { return Ok(Vec::new()) };
  let Answer::Lives(listed) = asked(&mut lines, 1, Verb::List { config: None }).await? else {
    return Err(DaemonError::Unanswered.into());
  };
  let held = listed.into_iter().filter(|one| one.held);
  Ok(
    held
      .map(|one| {
        let watched = one.watched.then(|| "a window".to_owned());
        let pins = one.pins.into_iter().map(|pin| format!("the pin {pin:?}"));
        let holders = watched.into_iter().chain(pins).collect::<Vec<_>>().join(", ");
        format!(
          "{}\t{}",
          one.life,
          if holders.is_empty() { "nothing, until its grace ends" } else { &holders }
        )
      })
      .collect(),
  )
}

/// A connection to the daemon of this machine, past its hello, or none when no daemon listens.
async fn dialed(
  name: &str,
) -> Result<Option<Framed<<Local as Dialer>::Stream, LinesCodec>>, CliError> {
  let stream = match Local::new(folder()?).dial().await {
    Ok(stream) => stream,
    Err(TransportError::Dial { .. }) => return Ok(None),
    Err(no) => return Err(DaemonError::from(no).into()),
  };
  let mut lines = Framed::new(stream, LinesCodec::new_with_max_length(BOUND));
  let name = Some(name.to_owned());
  let hello = Verb::Hello(Hello { protocol: PROTOCOL, kind: Kind::Window, name, role: None });
  asked(&mut lines, 0, hello).await?;
  Ok(Some(lines))
}

/// The name of the life of a record in the daemon: its absolute path.
fn named(record: &Path) -> String {
  path::absolute(record).unwrap_or_else(|_| record.to_owned()).display().to_string()
}

/// The answer of the daemon to one request of a peer, past each frame that it pushes meanwhile.
async fn asked(
  lines: &mut Framed<<Local as Dialer>::Stream, LinesCodec>,
  id: u64,
  verb: Verb,
) -> Result<Answer, DaemonError> {
  let request = serde_json::to_string(&Request { id, verb }).map_err(TransportError::from)?;
  lines.send(request).await.map_err(TransportError::from)?;
  loop {
    let heard = lines.next().await.ok_or(DaemonError::Unanswered)?;
    match serde_json::from_str::<Frame>(&heard.map_err(TransportError::from)?) {
      Ok(Frame::Reply { id: got, answer }) if got == id => return Ok(answer),
      Ok(Frame::Refused { id: Some(got), error, .. }) if got == id => return Err(error.into()),
      Ok(_) => {}
      Err(no) => return Err(DaemonError::Unread(no)),
    }
  }
}

/// The file of the folder of the daemon that holds the address of its door of the windows, with
/// its token.
const DOOR: &str = "window.json";

/// The daemon on the tailnet, when the settings of `host.json` at a path turn it on.
fn tailnet(path: &Path, config: Config) -> Result<Option<Tailnet>, CliError> {
  let Config { tailnet, roles, .. } = config;
  let Some(ports) = tailnet else { return Ok(None) };
  let Some(api) = LocalApi::find() else { return Err(CliError::NoTailscale(path.to_path_buf())) };
  Ok(Some(Tailnet { identity: Tailscale::new(api, roles), ports }))
}

/// The watch of `host.json` at a path, which gives the access list of each change of the file to
/// the identity of the tailnet, and says why it keeps the list before when the file does not
/// parse. It watches the folder, since an editor writes the file anew in its place. The ports of
/// the tailnet and the bound of a stall take a change only at the next start of the daemon.
fn followed(
  path: &Path,
  identity: Tailscale,
  said: impl Fn(&str) + Send + 'static,
) -> notify::Result<RecommendedWatcher> {
  let file = path.to_path_buf();
  let mut watcher = recommended_watcher(move |event: notify::Result<Event>| {
    let Ok(event) = event else { return };
    let ours = event.paths.iter().any(|one| one.file_name() == file.file_name());
    if !ours || event.kind.is_access() {
      return;
    }
    match Config::load(&file) {
      Ok(config) => identity.replace(config.roles),
      Err(no) => said(&format!("keeps the access list before the change of the file: {no}")),
    }
  })?;
  watcher.watch(path.parent().unwrap_or(path), RecursiveMode::NonRecursive)?;
  Ok(watcher)
}
