//! `furb host`: the daemon of this machine, which holds each life by its record, at one place in
//! the cache of furb, as `furb-host` serves it. The file `host.json` of the config folder of furb
//! turns on the daemon on the tailnet, and gives each login of the network its role.

use std::time::Duration;

use furb::{extension::Places, life::Opening, world};
use furb_host::{
  DaemonError, Settings, Tailnet,
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
  let settings = Settings { opening: Opening::default(), grace, tailnet: tailnet()?, door, host };
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

/// The daemon on the tailnet, when `host.json` turns it on.
fn tailnet() -> Result<Option<Tailnet>, CliError> {
  let path = Places::here().config.join("host.json");
  let Config { tailnet, roles } = Config::load(&path)?;
  let Some(ports) = tailnet else { return Ok(None) };
  let Some(api) = LocalApi::find() else { return Err(CliError::NoTailscale(path)) };
  Ok(Some(Tailnet { identity: Tailscale::new(api, roles), ports }))
}
