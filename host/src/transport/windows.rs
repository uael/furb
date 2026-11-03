//! The local transport on Windows: a named pipe, which only the user of the daemon reaches, and
//! which takes no peer of another machine.

use std::{io, path::Path, ptr, time::Duration};

use tokio::net::windows::named_pipe::{
  ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};
use windows_sys::Win32::{
  Foundation::{ERROR_ACCESS_DENIED, ERROR_PIPE_BUSY, LocalFree},
  Security::{
    Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
    PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
  },
  System::WindowsProgramming::GetUserNameW,
};

use crate::error::TransportError;

pub(crate) type Accepted = NamedPipeServer;
pub(crate) type Dialed = NamedPipeClient;

/// The place of the pipe, and the instance of it that waits for the next peer.
pub(crate) struct Listener {
  place: String,
  next: NamedPipeServer,
}

/// The access list of the pipe: all rights to its owner, the user of the daemon, and none to
/// another.
const OWNER_ALONE: &str = "D:P(A;;GA;;;OW)";

pub(crate) async fn listen(folder: &Path) -> Result<Listener, TransportError> {
  let place = piped(folder);
  let next = instance(&place, true)?;
  Ok(Listener { place, next })
}

impl Listener {
  pub(crate) async fn accept(&mut self) -> Result<Accepted, TransportError> {
    self.next.connect().await.map_err(TransportError::Accept)?;
    let next = instance(&self.place, false)?;
    Ok(std::mem::replace(&mut self.next, next))
  }
}

/// A client of the pipe. Each instance of the pipe may be busy for a moment between two peers.
pub(crate) async fn dial(folder: &Path) -> Result<Dialed, TransportError> {
  let place = piped(folder);
  loop {
    match ClientOptions::new().open(&place) {
      Err(no) if no.raw_os_error() == Some(ERROR_PIPE_BUSY.cast_signed()) => {
        tokio::time::sleep(Duration::from_millis(20)).await;
      }
      opened => return opened.map_err(|source| TransportError::Dial { place, source }),
    }
  }
}

/// The login of the user of the daemon, the one user whom the access list of the pipe lets in.
pub(crate) fn login(_: &Accepted) -> Result<String, TransportError> {
  let mut name = [0u16; 257];
  let mut size: u32 = 257;
  // SAFETY: name holds size units, and the call writes at most that many.
  if unsafe { GetUserNameW(name.as_mut_ptr(), &raw mut size) } == 0 {
    return Err(TransportError::Login(io::Error::last_os_error()));
  }
  let size = usize::try_from(size).unwrap_or(name.len()).min(name.len());
  Ok(String::from_utf16_lossy(&name[..size.saturating_sub(1)]))
}

/// The name of the pipe of a folder.
fn piped(folder: &Path) -> String {
  let named = folder.to_string_lossy().replace(['\\', '/', ':'], "-");
  format!(r"\\.\pipe\furb-{named}")
}

/// One instance of the pipe, under the access list of the owner alone. The first instance is
/// refused when another daemon holds the pipe.
fn instance(place: &str, first: bool) -> Result<NamedPipeServer, TransportError> {
  let listening = |source| TransportError::Listen { place: place.to_owned(), source };
  let sddl: Vec<u16> = OWNER_ALONE.encode_utf16().chain([0]).collect();
  let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
  // SAFETY: sddl ends with a NUL, and descriptor takes memory that LocalFree frees below.
  let made = unsafe {
    ConvertStringSecurityDescriptorToSecurityDescriptorW(
      sddl.as_ptr(),
      SDDL_REVISION_1,
      &raw mut descriptor,
      ptr::null_mut(),
    )
  };
  if made == 0 {
    return Err(listening(io::Error::last_os_error()));
  }
  let mut attributes = SECURITY_ATTRIBUTES {
    nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(u32::MAX),
    lpSecurityDescriptor: descriptor,
    bInheritHandle: 0,
  };
  let mut options = ServerOptions::new();
  options.first_pipe_instance(first).reject_remote_clients(true);
  // SAFETY: attributes, and the descriptor that it points at, live until the call returns.
  let server =
    unsafe { options.create_with_security_attributes_raw(place, (&raw mut attributes).cast()) };
  // SAFETY: the conversion above gave descriptor, and nothing reads it after the pipe is made.
  unsafe { LocalFree(descriptor.cast()) };
  server.map_err(|source| match source.raw_os_error() {
    Some(code) if first && code == ERROR_ACCESS_DENIED.cast_signed() => {
      TransportError::Taken { place: place.to_owned() }
    }
    _ => listening(source),
  })
}
