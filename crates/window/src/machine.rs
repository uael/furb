//! A machine: one daemon of furb, which the window reaches on the local transport, and starts when
//! none listens, or on the tailnet by TCP, where the window finds it through the local API of
//! Tailscale.

use std::{
  env,
  fs::{self, DirBuilder, File},
  io,
  net::SocketAddr,
  path::{Path, PathBuf},
  process::{ExitStatus, Stdio},
  time::Duration,
};

use furb::world;
use furb_host::{
  STARTING, TransportError,
  transport::{Dialer, Local, LocalApi, Tcp},
};
use futures::future;
use tokio::{
  io::{AsyncRead, AsyncWrite},
  process::{Child, Command},
  time,
};

use crate::Error;

/// How long the window waits between two dials of a daemon that starts.
const AGAIN: Duration = Duration::from_millis(25);

/// The file of the folder of a daemon that takes what the daemon says, which says why when the
/// daemon ends before it listens.
const LOG: &str = "host.log";

/// How long the window waits for a machine of the tailnet to take a dial before it takes the
/// machine as one where no daemon listens: Tailscale may first find its path to the machine.
const KNOCK: Duration = Duration::from_secs(5);

/// One daemon of furb, where the window reaches it.
#[derive(Debug)]
pub struct Machine {
  /// How the window names the machine.
  pub name: String,
  /// Where the window reaches the daemon.
  pub reach: Reach,
  /// Whether the window only reads on the machine, as `furb --read` asks: it shows each session
  /// live and says nothing to it.
  pub reads: bool,
}

/// Where the window reaches the daemon of a machine.
#[derive(Debug)]
pub enum Reach {
  /// The daemon of this computer on the local transport: the folder where it listens, and the
  /// command that serves it, which the window runs when no daemon answers there.
  Here { folder: PathBuf, served: Box<Command> },
  /// The daemon of another machine on the tailnet, by TCP at its address there and the port of
  /// furb. The window starts nothing there.
  Tailnet(SocketAddr),
}

/// Where the window finds the other machines of the tailnet: the local API of Tailscale, which
/// names them, and the port where it looks for the daemon of each.
#[derive(Debug, Clone)]
pub struct Tailnet {
  pub api: LocalApi,
  pub port: u16,
}

impl Tailnet {
  /// The tailnet of this machine, where Tailscale runs, at the port of furb. A debug build looks at
  /// the port that `FURB_TAILNET_PORT` names in its place, as a capture of the frames asks for its
  /// stand-in of Tailscale, whose machines all stand on loopback.
  pub fn here() -> Option<Self> {
    let api = LocalApi::find()?;
    #[cfg(debug_assertions)]
    let port = env::var("FURB_TAILNET_PORT").ok().and_then(|one| one.parse().ok());
    #[cfg(not(debug_assertions))]
    let port = None;
    Some(Self { api, port: port.unwrap_or(Tcp::PORT) })
  }
}

/// The look of a window at the tailnet: where it looks, the machines that it knows by name, and
/// whether it only reads on each one that it finds.
pub(crate) struct Looking {
  pub(crate) tailnet: Tailnet,
  pub(crate) known: Vec<String>,
  pub(crate) reads: bool,
}

impl Looking {
  /// Each machine of the tailnet that the window did not know, whose daemon takes a dial at the
  /// port, which the window knows from then on. None while Tailscale does not answer.
  pub(crate) async fn found(&mut self) -> Vec<Machine> {
    let Ok(machines) = self.tailnet.api.machines().await else { return Vec::new() };
    let (port, reads) = (self.tailnet.port, self.reads);
    let knocked = machines.into_iter().filter(|(name, _)| !self.known.contains(name)).map(
      |(name, ip)| async move {
        let addr = SocketAddr::new(ip, port);
        let serves = time::timeout(KNOCK, Tcp::new(addr).dial()).await.is_ok_and(|one| one.is_ok());
        serves.then_some(Machine { name, reach: Reach::Tailnet(addr), reads })
      },
    );
    let found: Vec<Machine> = future::join_all(knocked).await.into_iter().flatten().collect();
    self.known.extend(found.iter().map(|one| one.name.clone()));
    found
  }
}

/// A stream of a daemon that the window dialed, by any transport.
pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> Stream for S {}

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
    let served = Box::new(served);
    Ok(Self { name: furb_host::machine(), reach: Reach::Here { folder, served }, reads: false })
  }

  /// The stream of the daemon. The window serves the daemon of this computer first when none
  /// listens. A daemon that another window started at the same moment takes the place and this one
  /// ends, so the window dials once more after the end of its own.
  pub async fn reach(&mut self) -> Result<Box<dyn Stream>, Error> {
    let (folder, served) = match &mut self.reach {
      Reach::Here { folder, served } => (folder, served),
      Reach::Tailnet(addr) => return Ok(Box::new(Tcp::new(*addr).dial().await?)),
    };
    let local = Local::new(&*folder);
    match local.dial().await {
      Err(TransportError::Dial { .. }) => {}
      dialed => return Ok(Box::new(dialed?)),
    }
    let mut daemon = started(folder, served).map_err(|source| start(folder, source))?;
    let listens = async {
      loop {
        if let Ok(stream) = local.dial().await {
          return Ok(stream);
        }
        if let Ok(ended) = time::timeout(AGAIN, daemon.wait()).await {
          let status = ended.map_err(|source| start(folder, source))?;
          return local.dial().await.ok().ok_or_else(|| ended_before(folder, status));
        }
      }
    };
    let stream =
      time::timeout(STARTING, listens).await.unwrap_or(Err(Error::Slow { within: STARTING }))?;
    Ok(Box::new(stream))
  }
}

impl Machine {
  /// The build of the program that serves the daemon of this computer, as [`furb_host::built`]
  /// gives it: none on the tailnet, where the window starts nothing, and none while the program
  /// cannot be read, as while a build replaces it.
  pub(crate) fn program(&self) -> Option<String> {
    let Reach::Here { served, .. } = &self.reach else { return None };
    furb_host::built(Path::new(served.as_std().get_program()))
  }

  /// The daemon of this computer handed to the program that serves it, as `furb host upgrade` does:
  /// in a group of its own, as [`started`] starts a daemon, so it outlives the window and no end of
  /// a command reaches it. The daemon that runs ends each life with its image and gives it, and
  /// each window dials the new one.
  pub(crate) fn upgrade(&self) -> io::Result<()> {
    let Reach::Here { folder, served } = &self.reach else { return Ok(()) };
    let mut upgrade = Command::new(served.as_std().get_program());
    upgrade.args(["host", "upgrade"]);
    for (key, value) in served.as_std().get_envs() {
      match value {
        Some(value) => upgrade.env(key, value),
        None => upgrade.env_remove(key),
      };
    }
    let mut child = started(folder, &mut upgrade)?;
    // The new daemon outlives the window; the task only reaps it if it ends first.
    drop(tokio::spawn(async move { child.wait().await }));
    Ok(())
  }
}

/// The daemon as a process of its own, which outlives the window: in a group of its own, so no
/// signal of the terminal of the window reaches it, with what it says in the log of its folder.
/// The folder is of the user alone, as the daemon asks of the folder where it listens.
fn started(folder: &Path, served: &mut Command) -> io::Result<Child> {
  let mut made = DirBuilder::new();
  made.recursive(true);
  #[cfg(unix)]
  {
    use std::os::unix::fs::DirBuilderExt;
    made.mode(0o700);
    served.process_group(0);
  }
  #[cfg(windows)]
  {
    use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
    served.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
  }
  made.create(folder)?;
  let log = File::options().create(true).append(true).open(folder.join(LOG))?;
  served.stdin(Stdio::null()).stdout(log.try_clone()?).stderr(log).spawn()
}

fn start(folder: &Path, source: io::Error) -> Error {
  Error::Start { folder: folder.to_path_buf(), source }
}

/// The end of a daemon before it listened, with the last line that it said.
fn ended_before(folder: &Path, status: ExitStatus) -> Error {
  let log = fs::read_to_string(folder.join(LOG)).unwrap_or_default();
  Error::Ended { status, said: log.trim().lines().last().unwrap_or_default().to_owned() }
}
