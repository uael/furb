//! uv, which runs a module of Python of a kit whose header names what it needs, as PEP 723 says:
//! it makes that environment once, and runs the module there, so a kit needs no package of the
//! Python of the machine.
//!
//! The ear takes the uv that `FURB_UV` names, else the one beside the program of this process,
//! else the one on PATH. With none, it fetches the release of uv that furb pins for the system into
//! the cache of furb, once, and checks its sha256 against the one that this file holds, never one
//! that the server of the release gives. A fetch or a check that fails refuses, and says how to
//! install uv by hand.

use std::{
  env, fs, io,
  path::{Path, PathBuf},
  process,
  time::Duration,
};

use reqwest::{Client, StatusCode};
use sha2::{Digest, Sha256};
use tokio::{process::Command, task::JoinError};

use super::beside;
use crate::{ear::spawned, world::cache};

/// The variable of the environment that names the uv that runs a module of a kit.
const NAMED: &str = "FURB_UV";

/// The program of uv on this system.
const NAME: &str = if cfg!(windows) { "uv.exe" } else { "uv" };

/// The release of uv that furb fetches.
const VERSION: &str = "0.13.1";

/// What a person who reads a failed fetch can do.
const BY_HAND: &str = "Install uv as https://docs.astral.sh/uv/getting-started/installation/ \
                       says, or name its program in FURB_UV.";

/// The archive of the release of uv for one system, by the architecture and the system that
/// `std::env::consts` names, and its sha256.
struct Pin {
  arch: &'static str,
  os: &'static str,
  archive: &'static str,
  sha256: &'static str,
}

const PINS: [Pin; 6] = [
  Pin {
    arch: "aarch64",
    os: "macos",
    archive: "uv-aarch64-apple-darwin.tar.gz",
    sha256: "cfe454573412ff7380d15fc7dae5997ab53ffce72fce0a78e91ddbc84001acd2",
  },
  Pin {
    arch: "x86_64",
    os: "macos",
    archive: "uv-x86_64-apple-darwin.tar.gz",
    sha256: "bb7b805bdcf3d0bc8c74ceb94569e6b3e8aa35bd492acdbb9d46bf46d9d4ff6b",
  },
  Pin {
    arch: "aarch64",
    os: "linux",
    archive: "uv-aarch64-unknown-linux-gnu.tar.gz",
    sha256: "9041c73c99fdc5892cf128140beb354eca9231d502068cc36e6cb53a8e59ad14",
  },
  Pin {
    arch: "x86_64",
    os: "linux",
    archive: "uv-x86_64-unknown-linux-gnu.tar.gz",
    sha256: "eadd9d27c134ab5ac1ffd4473b97a741407f3ea238288f03121fc4deddf90220",
  },
  Pin {
    arch: "aarch64",
    os: "windows",
    archive: "uv-aarch64-pc-windows-msvc.zip",
    sha256: "ec7846a5939b4654afee24f2d82bf6fde71bd7a6952d7748f1135ab301dd0a49",
  },
  Pin {
    arch: "x86_64",
    os: "windows",
    archive: "uv-x86_64-pc-windows-msvc.zip",
    sha256: "3b2661a463a5baea7f5076ea2b25ab71d77f36a6c0d809e5350651792f80a672",
  },
];

/// Why furb has no uv.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
  #[error("furb pins no release of uv for {arch} {os}. {}", BY_HAND)]
  Unpinned { arch: &'static str, os: &'static str },
  #[error("furb finds no folder of its cache for uv. {}", BY_HAND)]
  Uncached,
  #[error("furb could not fetch uv from {url}: {source}. {}", BY_HAND)]
  Request { url: String, source: reqwest::Error },
  #[error("furb could not fetch uv: {url} answered {status}. {}", BY_HAND)]
  Status { url: String, status: StatusCode },
  #[error("the archive of {url} has the sha256 {found}, not the one that furb pins. {}", BY_HAND)]
  Checked { url: String, found: String },
  #[error("the fetch of uv ended before it was done: {source}. {}", BY_HAND)]
  Lost { source: JoinError },
  #[error("furb could not unpack uv in {}: {why}. {}", folder.display(), BY_HAND)]
  Unpacked { folder: PathBuf, why: String },
}

/// The uv that runs a module of a kit with a header: the one that [`NAMED`] names, else the one
/// beside the program of this process, else the one on PATH, else the one that the cache of furb
/// holds, which it fetches when it holds none.
pub(super) async fn uv() -> Result<PathBuf, Error> {
  let named = env::var_os(NAMED).filter(|one| !one.is_empty()).map(PathBuf::from);
  match named.or_else(|| beside(NAME)).or_else(on_path) {
    Some(found) => Ok(found),
    None => spawned(cached()).await.unwrap_or_else(|no| Err(Error::Lost { source: no })),
  }
}

/// The program of uv in a folder of PATH.
fn on_path() -> Option<PathBuf> {
  env::split_paths(&env::var_os("PATH")?).map(|folder| folder.join(NAME)).find(|one| one.is_file())
}

/// The uv of the cache of furb, fetched and checked when the cache holds none.
async fn cached() -> Result<PathBuf, Error> {
  let (arch, os) = (env::consts::ARCH, env::consts::OS);
  let pin = PINS.iter().find(|one| one.arch == arch && one.os == os);
  let pin = pin.ok_or(Error::Unpinned { arch, os })?;
  let folder = cache().ok_or(Error::Uncached)?.join("uv").join(VERSION);
  let program = folder.join(NAME);
  if program.is_file() {
    return Ok(program);
  }
  let url = format!("https://github.com/astral-sh/uv/releases/download/{VERSION}/{}", pin.archive);
  let archive = fetched(&url).await?;
  checked(pin, url, &archive)?;
  // Each process unpacks in a folder of its own, and moves the program into place at once, so two
  // lives that fetch at the same time never run a program that the other writes.
  let unpacked = folder.join(format!("{}.part", process::id()));
  let moved = unpack(pin, &archive, &unpacked).await.and_then(|inner| fs::rename(inner, &program));
  let removed = fs::remove_dir_all(&unpacked);
  match moved.and(removed) {
    Ok(()) => Ok(program),
    Err(_) if program.is_file() => Ok(program),
    Err(no) => Err(Error::Unpacked { folder: unpacked, why: no.to_string() }),
  }
}

/// The bytes of the archive at an address.
async fn fetched(url: &str) -> Result<Vec<u8>, Error> {
  let request = |source| Error::Request { url: url.to_owned(), source };
  let client = Client::builder().timeout(Duration::from_secs(300)).build().map_err(request)?;
  let response = client.get(url).send().await.map_err(request)?;
  let status = response.status();
  if !status.is_success() {
    return Err(Error::Status { url: url.to_owned(), status });
  }
  Ok(response.bytes().await.map_err(request)?.to_vec())
}

/// The archive of a pin, or why it is not the one that furb pins.
fn checked(pin: &Pin, url: String, archive: &[u8]) -> Result<(), Error> {
  let found = format!("{:x}", Sha256::digest(archive));
  if found == pin.sha256 { Ok(()) } else { Err(Error::Checked { url, found }) }
}

/// The program of uv, unpacked from its archive into a folder, by the tar of the system, which
/// reads a zip too on Windows: the program stands in a folder of the name of a tar.gz, and at the
/// root of a zip.
async fn unpack(pin: &Pin, archive: &[u8], folder: &Path) -> io::Result<PathBuf> {
  fs::create_dir_all(folder)?;
  let file = folder.join(pin.archive);
  fs::write(&file, archive)?;
  let status = Command::new(tar()).arg("-xf").arg(&file).arg("-C").arg(folder).status().await?;
  if !status.success() {
    return Err(io::Error::other(format!("tar ended with {status}")));
  }
  let inner = pin.archive.strip_suffix(".tar.gz").map_or(PathBuf::new(), PathBuf::from);
  Ok(folder.join(inner).join(NAME))
}

/// The tar of the system: on Windows the one of the system folder, since the tar of Git that PATH
/// can name first reads no zip.
fn tar() -> PathBuf {
  let system = env::var_os("SystemRoot").filter(|_| cfg!(windows));
  system.map_or_else(|| PathBuf::from("tar"), |root| Path::new(&root).join(r"System32\tar.exe"))
}

#[cfg(test)]
#[path = "uv.test.rs"]
mod test;
