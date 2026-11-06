//! `furb host`: the daemon of this machine, which holds each life by its record, at one place in
//! the cache of furb, as `furb-host` serves it. The file `host.json` of the config folder of furb
//! turns on the daemon on the tailnet, gives each login of the network its role, and gives the
//! bound of a stall. The daemon reads the file again at each change of it, so a login that the
//! file adds or drops holds its role at once, and a file that does not parse keeps the list before.

use std::{path::Path, time::Duration};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};

use furb::{extension::Places, life::Opening, world};
use furb_host::{
  DaemonError, STALL, Settings, Tailnet,
  access::Config,
  run,
  transport::{Local, LocalApi, Tailscale},
  upgrade,
};

use crate::error::CliError;

/// The daemon of this machine, served until a daemon of a new build takes its place, or, with
/// `upgrading`, served in the place of the daemon that runs, where a life with no holder lives
/// for `grace`. It says where it listens once it listens, so a program that starts it knows when
/// to dial it.
pub(crate) async fn serve(grace: Duration, upgrading: bool) -> Result<(), CliError> {
  let cache = world::cache().ok_or(CliError::NoCache)?;
  let folder = cache.join("host");
  let door = Some(folder.join(DOOR));
  let host = Some(folder.clone());
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
  let settings = Settings { opening: Opening::default(), grace, stall, tailnet, door, host };
  let local = Local::new(&folder);
  if upgrading {
    return Ok(upgrade(&local, settings).await?);
  }
  let listener = local.listen().await.map_err(DaemonError::from)?;
  println!("furb host listens in {}", folder.display());
  Ok(run(&local, listener, settings).await?)
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

#[cfg(test)]
#[path = "host.test.rs"]
mod test;
