//! The local transport on a Unix system: a socket in a folder of mode 0700 that the user owns.

use std::{
  ffi::CStr,
  fs::{self, DirBuilder},
  io::{self, Write},
  mem::MaybeUninit,
  os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
  path::{Path, PathBuf},
  ptr,
};

use tokio::net::{UnixListener, UnixStream};

use crate::error::TransportError;

pub(crate) type Accepted = UnixStream;
pub(crate) type Dialed = UnixStream;

pub(crate) struct Listener(UnixListener);

const SOCKET: &str = "host.sock";

pub(crate) async fn listen(folder: &Path) -> Result<Listener, TransportError> {
  guarded(folder)?;
  let socket = folder.join(SOCKET);
  bounded(&socket)?;
  match UnixListener::bind(&socket) {
    Err(no) if no.kind() == io::ErrorKind::AddrInUse => {
      if UnixStream::connect(&socket).await.is_ok() {
        return Err(TransportError::Taken { place: shown(&socket) });
      }
      fs::remove_file(&socket).map_err(|source| listening(&socket, source))?;
      UnixListener::bind(&socket).map(Listener).map_err(|source| listening(&socket, source))
    }
    bound => bound.map(Listener).map_err(|source| listening(&socket, source)),
  }
}

pub(crate) async fn succeed(folder: &Path) -> Result<Listener, TransportError> {
  listen(folder).await
}

impl Listener {
  pub(crate) async fn accept(&self) -> Result<Accepted, TransportError> {
    let (stream, _) = self.0.accept().await.map_err(TransportError::Accept)?;
    Ok(stream)
  }
}

pub(crate) async fn dial(folder: &Path) -> Result<Dialed, TransportError> {
  let socket = folder.join(SOCKET);
  UnixStream::connect(&socket)
    .await
    .map_err(|source| TransportError::Dial { place: shown(&socket), source })
}

/// The login of the peer, who must be the user of the daemon.
pub(crate) fn login(stream: &Accepted) -> Result<String, TransportError> {
  let uid = stream.peer_cred().map_err(TransportError::Accept)?.uid();
  if uid != user() {
    return Err(TransportError::Stranger { uid });
  }
  named(uid)
}

/// The login of the user of the daemon.
pub(crate) fn own() -> Result<String, TransportError> {
  named(user())
}

/// The folder of the socket made, of mode 0700, and refused when another user owns it or may
/// enter it.
pub(crate) fn guarded(folder: &Path) -> Result<(), TransportError> {
  let made = |source| TransportError::Folder { folder: folder.to_owned(), source };
  DirBuilder::new().recursive(true).mode(0o700).create(folder).map_err(made)?;
  let held = fs::metadata(folder).map_err(made)?;
  if !held.is_dir() || held.uid() != user() || held.mode() & 0o077 != 0 {
    return Err(TransportError::Exposed { folder: folder.to_owned() });
  }
  Ok(())
}

/// A new file that only the user reads, of mode 0600, written whole, in the place of any that stood
/// there: a file that stood keeps its own mode through a truncation, so it goes first.
pub(crate) fn private(file: &Path, bytes: &[u8]) -> io::Result<()> {
  match fs::remove_file(file) {
    Err(no) if no.kind() != io::ErrorKind::NotFound => return Err(no),
    _ => {}
  }
  let mut out = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(file)?;
  out.write_all(bytes)?;
  out.sync_all()
}

fn user() -> u32 {
  rustix::process::geteuid().as_raw()
}

/// The login of a user, from the database of the users.
#[expect(
  unsafe_code,
  reason = "the database of the users is read through libc, and no crate of the lock reads it safely"
)]
fn named(uid: u32) -> Result<String, TransportError> {
  let mut entry = MaybeUninit::<libc::passwd>::uninit();
  let mut buffer = vec![0; 4096];
  let mut found: *mut libc::passwd = ptr::null_mut();
  // SAFETY: each pointer is valid for the call, and the call writes at most the length of buffer.
  let code = unsafe {
    libc::getpwuid_r(uid, entry.as_mut_ptr(), buffer.as_mut_ptr(), buffer.len(), &raw mut found)
  };
  if code != 0 {
    return Err(TransportError::Login(io::Error::from_raw_os_error(code)));
  }
  if found.is_null() {
    return Err(TransportError::Login(io::ErrorKind::NotFound.into()));
  }
  // SAFETY: found points at entry, which the call filled.
  let name = unsafe { (*found).pw_name };
  // SAFETY: the name is a C string in buffer, which lives until the copy below.
  let name = unsafe { CStr::from_ptr(name) };
  Ok(name.to_string_lossy().into_owned())
}

/// The most bytes that a socket path holds: the size of `sun_path` less its closing NUL.
const BOUND: usize =
  size_of::<libc::sockaddr_un>() - std::mem::offset_of!(libc::sockaddr_un, sun_path) - 1;

fn bounded(socket: &Path) -> Result<(), TransportError> {
  let length = socket.as_os_str().len();
  if length > BOUND {
    return Err(TransportError::TooLong { place: shown(socket), length, bound: BOUND });
  }
  Ok(())
}

fn listening(socket: &Path, source: io::Error) -> TransportError {
  TransportError::Listen { place: shown(socket), source }
}

fn shown(socket: &Path) -> String {
  PathBuf::from(socket).display().to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn refuses_a_path_over_the_bound_before_the_bind() {
    let folder = PathBuf::from(format!("/tmp/{}", "x".repeat(BOUND)));
    let error = bounded(&folder.join(SOCKET)).unwrap_err();
    let message = error.to_string();
    assert!(matches!(error, TransportError::TooLong { .. }));
    assert!(message.contains(&format!("{}", folder.join(SOCKET).as_os_str().len())), "{message}");
    assert!(message.contains(&format!("{BOUND} bytes")), "{message}");
    assert!(bounded(Path::new("/tmp/furb/host.sock")).is_ok());
  }

  #[test]
  fn names_the_cause_of_a_listen_error() {
    let source = io::Error::from(io::ErrorKind::PermissionDenied);
    let cause = source.to_string();
    let message = listening(Path::new("/x/host.sock"), source).to_string();
    assert!(message.contains("/x/host.sock") && message.contains(&cause), "{message}");
  }
}
