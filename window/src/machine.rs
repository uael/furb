//! A machine: one daemon of furb, which the window reaches on the local transport, and starts when
//! none listens, or on the tailnet by TCP at the address that `machines.json` of the config folder
//! of furb names.

use std::{
  env,
  fs::{self, DirBuilder, File},
  io,
  path::{Path, PathBuf},
  process::{ExitStatus, Stdio},
  time::Duration,
};

use furb::{skill::Places, world};
use furb_host::{
  STARTING, TransportError,
  transport::{Dialer, Local, Tcp},
};
use serde::Deserialize;
use tokio::{
  io::{AsyncRead, AsyncWrite},
  net,
  process::{Child, Command},
  time,
};

use crate::Error;

/// How long the window waits between two dials of a daemon that starts.
const AGAIN: Duration = Duration::from_millis(25);

/// The file of the folder of a daemon that takes what the daemon says, which says why when the
/// daemon ends before it listens.
const LOG: &str = "host.log";

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
  /// The daemon of another machine on the tailnet, by TCP at an address as `host:port`, which
  /// Tailscale names. The window starts nothing there.
  Tailnet(String),
}

/// A stream of a daemon that the window dialed, by any transport.
pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> Stream for S {}

/// A machine of `machines.json`: its name and its address on the tailnet.
#[derive(Deserialize)]
struct Listed {
  name: String,
  address: String,
}

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

  /// The machines of the tailnet that `machines.json` of the config folder of furb names, in its
  /// order, each with its name and its address, as `[{"name": "studio", "address":
  /// "studio:7860"}]`. None where no file stands.
  pub fn listed() -> Result<Vec<Self>, Error> {
    let file = Places::here().config.join("machines.json");
    let text = match fs::read_to_string(&file) {
      Err(no) if no.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
      read => {
        read.map_err(|source| Error::Machines { file: file.clone(), why: source.to_string() })?
      }
    };
    let listed: Vec<Listed> = serde_json::from_str(&text)
      .map_err(|source| Error::Machines { file, why: source.to_string() })?;
    let machine =
      |one: Listed| Self { name: one.name, reach: Reach::Tailnet(one.address), reads: false };
    Ok(listed.into_iter().map(machine).collect())
  }

  /// The stream of the daemon. The window serves the daemon of this computer first when none
  /// listens. A daemon that another window started at the same moment takes the place and this one
  /// ends, so the window dials once more after the end of its own.
  pub async fn reach(&mut self) -> Result<Box<dyn Stream>, Error> {
    let (folder, served) = match &mut self.reach {
      Reach::Here { folder, served } => (folder, served),
      Reach::Tailnet(address) => {
        let dialed = |source| TransportError::Dial { place: address.clone(), source };
        let mut found = net::lookup_host(address.as_str()).await.map_err(dialed)?;
        let unknown = || io::Error::new(io::ErrorKind::NotFound, "the name has no address");
        let addr = found.next().ok_or_else(|| dialed(unknown()))?;
        return Ok(Box::new(Tcp::new(addr).dial().await?));
      }
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
