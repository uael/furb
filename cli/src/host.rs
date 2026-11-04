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
use tokio::runtime;

/// The daemon of this machine, served until a daemon of a new build takes its place, or, with
/// `upgrading`, served in the place of the daemon that runs, where a life with no holder lives
/// for `grace`. It says where it listens once it listens, so a program that starts it knows when
/// to dial it.
#[expect(
  clippy::disallowed_methods,
  reason = "the program blocks on the runtime of the daemon, which is all that it runs"
)]
pub(crate) fn serve(grace: Duration, upgrading: bool) -> Result<(), String> {
  let cache =
    world::cache().ok_or("this machine has no cache of furb, so the daemon has no place")?;
  let settings = Settings { opening: Opening::default(), grace, tailnet: tailnet()? };
  let folder = cache.join("host");
  let local = Local::new(&folder);
  let runtime = runtime::Builder::new_multi_thread().enable_all().build();
  let runtime = runtime.map_err(|no| format!("the daemon has no runtime: {no}"))?;
  let served: Result<(), DaemonError> = runtime.block_on(async {
    if upgrading {
      return upgrade(&local, settings).await;
    }
    let listener = local.listen().await?;
    println!("furb host listens in {}", folder.display());
    run(&local, listener, settings).await
  });
  served.map_err(|no| no.to_string())
}

/// The daemon on the tailnet, when `host.json` turns it on.
fn tailnet() -> Result<Option<Tailnet>, String> {
  let path = Places::here().config.join("host.json");
  let Config { tailnet, roles } = Config::load(&path).map_err(|no| no.to_string())?;
  let Some(ports) = tailnet else { return Ok(None) };
  let api = LocalApi::find().ok_or_else(|| {
    format!("{} turns on the tailnet, and no Tailscale runs on this machine", path.display())
  })?;
  Ok(Some(Tailnet { identity: Tailscale::new(api, roles), ports }))
}
