//! A machine: one daemon of furb, which the window reaches on the local transport, and starts when
//! none listens.

use std::{
  env,
  fs::{self, DirBuilder, File},
  io,
  path::PathBuf,
  process::{ExitStatus, Stdio},
  time::Duration,
};

use furb::world;
use furb_host::{
  TransportError,
  transport::{Dialer, Local},
};
use tokio::{
  process::{Child, Command},
  time,
};

use crate::Error;

/// How long the window waits for a daemon that it started to listen.
const STARTING: Duration = Duration::from_secs(60);

/// How long the window waits between two dials of a daemon that starts.
const AGAIN: Duration = Duration::from_millis(25);

/// The file of the folder of a daemon that takes what the daemon says, which says why when the
/// daemon ends before it listens.
const LOG: &str = "host.log";

/// One daemon of furb on the local transport.
#[derive(Debug)]
pub struct Machine {
  /// How the window names the machine.
  pub name: String,
  /// The folder where the daemon listens.
  pub folder: PathBuf,
  /// The command that serves the daemon of that folder, which the window runs when no daemon
  /// answers there.
  pub served: Command,
}

/// The stream of a daemon that the window dialed.
type Stream = <Local as Dialer>::Stream;

impl Machine {
  /// The daemon of this machine, in the cache of furb, as `furb host` serves it: the program furb
  /// beside the window, which is of its build, else the furb on PATH.
  pub fn here() -> Result<Self, Error> {
    let folder = world::cache().ok_or(Error::NoCache)?.join("host");
    let named = format!("furb{}", env::consts::EXE_SUFFIX);
    let beside = env::current_exe().ok().map(|window| window.with_file_name(named));
    let path = env::var_os("PATH").unwrap_or_default();
    let furb = beside
      .filter(|furb| furb.is_file())
      .or_else(|| world::program("furb", env::split_paths(&path)))
      .ok_or(Error::NoFurb)?;
    let mut served = Command::new(furb);
    served.arg("host");
    Ok(Self { name: name(), folder, served })
  }

  /// The stream of the daemon, which the window serves first when none listens. A daemon that
  /// another window started at the same moment takes the place and this one ends, so the window
  /// dials once more after the end of its own.
  pub async fn reach(mut self) -> Result<Stream, Error> {
    let local = Local::new(&self.folder);
    match local.dial().await {
      Err(TransportError::Dial { .. }) => {}
      dialed => return Ok(dialed?),
    }
    let mut daemon = self.started().map_err(|source| self.start(source))?;
    let listens = async {
      loop {
        if let Ok(stream) = local.dial().await {
          return Ok(stream);
        }
        if let Ok(ended) = time::timeout(AGAIN, daemon.wait()).await {
          let status = ended.map_err(|source| self.start(source))?;
          return local.dial().await.ok().ok_or_else(|| self.ended(status));
        }
      }
    };
    time::timeout(STARTING, listens).await.unwrap_or(Err(Error::Slow { within: STARTING }))
  }

  /// The daemon as a process of its own, which outlives the window: in a group of its own, so no
  /// signal of the terminal of the window reaches it, with what it says in the log of its folder.
  /// The folder is of the user alone, as the daemon asks of the folder where it listens.
  fn started(&mut self) -> io::Result<Child> {
    let mut folder = DirBuilder::new();
    folder.recursive(true);
    #[cfg(unix)]
    {
      use std::os::unix::fs::DirBuilderExt;
      folder.mode(0o700);
      self.served.process_group(0);
    }
    #[cfg(windows)]
    {
      use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
      self.served.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }
    folder.create(&self.folder)?;
    let log = File::options().create(true).append(true).open(self.folder.join(LOG))?;
    self.served.stdin(Stdio::null()).stdout(log.try_clone()?).stderr(log).spawn()
  }

  fn start(&self, source: io::Error) -> Error {
    Error::Start { folder: self.folder.clone(), source }
  }

  /// The end of a daemon before it listened, with the last line that it said.
  fn ended(&self, status: ExitStatus) -> Error {
    let log = fs::read_to_string(self.folder.join(LOG)).unwrap_or_default();
    Error::Ended { status, said: log.trim().lines().last().unwrap_or_default().to_owned() }
  }
}

/// The name of this machine, without its domain.
fn name() -> String {
  #[cfg(unix)]
  let full = {
    use rustix::system;
    system::uname().nodename().to_string_lossy().into_owned()
  };
  #[cfg(not(unix))]
  let full = env::var("COMPUTERNAME").unwrap_or_default();
  full.split('.').next().unwrap_or_default().to_owned()
}
