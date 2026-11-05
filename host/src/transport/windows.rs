//! The local transport on Windows: a named pipe, which only the user of the daemon reaches, and
//! which takes no peer of another machine, beside a folder for the files of the daemon under the
//! access list of that user alone.

use std::{
  future,
  io::{self, Write},
  os::windows::{ffi::OsStrExt, io::FromRawHandle},
  path::Path,
  ptr,
  time::Duration,
};

use tokio::net::windows::named_pipe::{
  ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};
use windows_sys::Win32::{
  Foundation::{
    ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_PIPE_BUSY, GENERIC_WRITE,
    INVALID_HANDLE_VALUE, LocalFree,
  },
  Security::{
    Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
    PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
  },
  Storage::FileSystem::{CREATE_NEW, CreateDirectoryW, CreateFileW, FILE_ATTRIBUTE_NORMAL},
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

pub(crate) fn listen(
  folder: &Path,
) -> impl Future<Output = Result<Listener, TransportError>> + use<> {
  let place = piped(folder);
  let next = guarded(folder).and_then(|()| instance(&place, true));
  future::ready(next.map(|next| Listener { place, next }))
}

/// The pipe after a handover: the daemon that gave its lives holds a peer of the pipe, so the name
/// stands, and the first instance of it cannot be asked for.
pub(crate) fn succeed(
  folder: &Path,
) -> impl Future<Output = Result<Listener, TransportError>> + use<> {
  let place = piped(folder);
  future::ready(instance(&place, false).map(|next| Listener { place, next }))
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
  own()
}

/// The login of the user of the daemon.
pub(crate) fn own() -> Result<String, TransportError> {
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

/// A security descriptor that gives all rights to the owner alone, the user of the daemon, and
/// none to another, handed to `made` and freed once `made` returns.
fn owner_alone<T>(made: impl FnOnce(*mut SECURITY_ATTRIBUTES) -> T) -> io::Result<T> {
  let sddl: Vec<u16> = OWNER_ALONE.encode_utf16().chain([0]).collect();
  let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
  // SAFETY: sddl ends with a NUL, and descriptor takes memory that LocalFree frees below.
  let converted = unsafe {
    ConvertStringSecurityDescriptorToSecurityDescriptorW(
      sddl.as_ptr(),
      SDDL_REVISION_1,
      &raw mut descriptor,
      ptr::null_mut(),
    )
  };
  if converted == 0 {
    return Err(io::Error::last_os_error());
  }
  let mut attributes = SECURITY_ATTRIBUTES {
    nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(u32::MAX),
    lpSecurityDescriptor: descriptor,
    bInheritHandle: 0,
  };
  let got = made(&raw mut attributes);
  // SAFETY: the conversion above gave descriptor, and nothing reads it once made returned.
  unsafe { LocalFree(descriptor.cast()) };
  Ok(got)
}

/// One instance of the pipe, under the access list of the owner alone. The first instance is
/// refused when another daemon holds the pipe.
fn instance(place: &str, first: bool) -> Result<NamedPipeServer, TransportError> {
  let listening = |source| TransportError::Listen { place: place.to_owned(), source };
  let mut options = ServerOptions::new();
  options.first_pipe_instance(first).reject_remote_clients(true);
  // SAFETY: the attributes, and the descriptor that they point at, live until the call returns.
  let server = owner_alone(|attributes| unsafe {
    options.create_with_security_attributes_raw(place, attributes.cast())
  })
  .map_err(listening)?;
  server.map_err(|source| match source.raw_os_error() {
    Some(code) if first && code == ERROR_ACCESS_DENIED.cast_signed() => {
      TransportError::Taken { place: place.to_owned() }
    }
    _ => listening(source),
  })
}

/// The folder of the files of the daemon made, under the access list of its owner alone when it
/// is new, and refused when it is not a folder, as a link that points elsewhere.
pub(crate) fn guarded(folder: &Path) -> Result<(), TransportError> {
  let made = |source| TransportError::Folder { folder: folder.to_owned(), source };
  if let Some(parent) = folder.parent().filter(|parent| !parent.as_os_str().is_empty()) {
    std::fs::create_dir_all(parent).map_err(made)?;
  }
  let wide = widened(folder);
  // SAFETY: wide ends with a NUL, and the attributes live until the call returns.
  let created = owner_alone(|attributes| unsafe { CreateDirectoryW(wide.as_ptr(), attributes) })
    .map_err(made)?;
  if created == 0 {
    let no = io::Error::last_os_error();
    if no.raw_os_error() != Some(ERROR_ALREADY_EXISTS.cast_signed()) {
      return Err(made(no));
    }
  }
  let held = std::fs::symlink_metadata(folder).map_err(made)?;
  if !held.is_dir() || held.file_type().is_symlink() {
    return Err(TransportError::Exposed { folder: folder.to_owned() });
  }
  Ok(())
}

/// A new file that only its owner reaches, written whole, in the place of any that stood there:
/// its access list is given at its creation, so no other user opens it, even for a moment.
pub(crate) fn private(file: &Path, bytes: &[u8]) -> io::Result<()> {
  match std::fs::remove_file(file) {
    Err(no) if no.kind() != io::ErrorKind::NotFound => return Err(no),
    _ => {}
  }
  let wide = widened(file);
  // SAFETY: wide ends with a NUL, and the attributes live until the call returns.
  let handle = owner_alone(|attributes| unsafe {
    CreateFileW(
      wide.as_ptr(),
      GENERIC_WRITE,
      0,
      attributes,
      CREATE_NEW,
      FILE_ATTRIBUTE_NORMAL,
      ptr::null_mut(),
    )
  })?;
  if handle == INVALID_HANDLE_VALUE {
    return Err(io::Error::last_os_error());
  }
  // SAFETY: CreateFileW gave this handle, which nothing else owns.
  let mut out = unsafe { std::fs::File::from_raw_handle(handle) };
  out.write_all(bytes)?;
  out.sync_all()
}

/// A path as the NUL-ended UTF-16 that the calls of Windows read.
fn widened(path: &Path) -> Vec<u16> {
  path.as_os_str().encode_wide().chain([0]).collect()
}

#[cfg(test)]
mod tests {
  use std::{path::Path, ptr};

  use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
      Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, GetNamedSecurityInfoW,
        SDDL_REVISION_1, SE_FILE_OBJECT,
      },
      DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    },
  };

  use super::{guarded, private, widened};

  /// The access list of a file or a folder, as SDDL.
  fn listed(path: &Path) -> String {
    let wide = widened(path);
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    // SAFETY: wide ends with a NUL, and descriptor takes memory that LocalFree frees below.
    let code = unsafe {
      GetNamedSecurityInfoW(
        wide.as_ptr(),
        SE_FILE_OBJECT,
        DACL_SECURITY_INFORMATION,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        &raw mut descriptor,
      )
    };
    assert_eq!(code, 0, "{}", path.display());
    let mut text: *mut u16 = ptr::null_mut();
    let mut size = 0u32;
    // SAFETY: descriptor is valid, and text takes memory that LocalFree frees below.
    let made = unsafe {
      ConvertSecurityDescriptorToStringSecurityDescriptorW(
        descriptor,
        SDDL_REVISION_1,
        DACL_SECURITY_INFORMATION,
        &raw mut text,
        &raw mut size,
      )
    };
    assert_ne!(made, 0);
    // SAFETY: the call wrote size units at text.
    let sddl = unsafe { std::slice::from_raw_parts(text, usize::try_from(size).unwrap()) };
    let sddl = String::from_utf16_lossy(sddl).trim_end_matches('\0').to_owned();
    // SAFETY: text came from the call above, and nothing reads it after.
    unsafe { LocalFree(text.cast()) };
    // SAFETY: descriptor came from the call above, and nothing reads it after.
    unsafe { LocalFree(descriptor.cast()) };
    sddl
  }

  /// Whether an access list is protected from what its parent gives, and lets in the owner alone.
  fn owner_alone(sddl: &str) -> bool {
    sddl.starts_with("D:P") && sddl.matches('(').count() == 1 && sddl.ends_with(";;;OW)")
  }

  #[test]
  fn the_folder_and_the_file_of_the_door_let_in_their_owner_alone() {
    let yard = std::env::temp_dir().join(format!("furb-acl-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&yard));
    let folder = yard.join("cold").join("host");
    guarded(&folder).unwrap();
    assert!(owner_alone(&listed(&folder)), "{}", listed(&folder));
    let file = folder.join("window.draft");
    std::fs::write(&file, "open to all").unwrap();
    private(&file, b"{}").unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"{}");
    assert!(owner_alone(&listed(&file)), "{}", listed(&file));
    guarded(&folder).unwrap();
    std::fs::remove_dir_all(&yard).unwrap();
  }
}
