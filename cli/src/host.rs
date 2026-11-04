//! `furb host`: the daemon of this machine, which holds each life by its record, at one place in
//! the cache of furb, as `furb-host` serves it.

use furb::{life::Opening, world};
use furb_host::{DaemonError, GRACE, Settings, run, transport::Local, upgrade};
use tokio::runtime;

/// The daemon of this machine, served until a daemon of a new build takes its place, or, with
/// `upgrading`, served in the place of the daemon that runs. It says where it listens once it
/// listens, so a program that starts it knows when to dial it.
#[expect(
  clippy::disallowed_methods,
  reason = "the program blocks on the runtime of the daemon, which is all that it runs"
)]
pub(crate) fn serve(upgrading: bool) -> Result<(), String> {
  let cache =
    world::cache().ok_or("this machine has no cache of furb, so the daemon has no place")?;
  let (folder, settings) =
    (cache.join("host"), Settings { opening: Opening::default(), grace: GRACE });
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
