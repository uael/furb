//! One process at a time owns a record, by a lease that the kernel holds for it.
use napi_derive::napi;
use std::fs::{File, OpenOptions, TryLockError, create_dir_all, metadata};
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// The lease of one record: a lock on the file beside the record, `<record>.lock`, which the process holds until
/// it disposes the lease or ends. The kernel ends the lease of a process that ends, whatever the process number
/// says after it. The holder may move or remove the file, as a delete of the record does.
#[napi]
pub struct RecordLock {
  path: String,
  file: Option<File>,
}

#[napi]
impl RecordLock {
  /// The lease of the record at this path, or the refusal when another holds it.
  #[napi(constructor)]
  pub fn new(path: String) -> napi::Result<Self> {
    let lock = format!("{path}.lock");
    let failed = |error: std::io::Error| napi::Error::from_reason(format!("{error}: {lock}"));
    if let Some(parent) = Path::new(&path).parent() {
      create_dir_all(parent).map_err(failed)?;
    }
    loop {
      let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)
        .map_err(failed)?;
      match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
          return Err(napi::Error::from_reason(format!("Another process owns {path}.")));
        }
        Err(TryLockError::Error(error)) => return Err(failed(error)),
      }
      // The lock is the lease only while its file is the one at the path. A holder that moved or removed the file
      // after this process opened it leaves a lock that no later process meets, so this process opens the path
      // again.
      let held = file.metadata().map_err(failed)?;
      match metadata(&lock) {
        Ok(now) if (now.dev(), now.ino()) == (held.dev(), held.ino()) => {
          return Ok(Self { file: Some(file), path });
        }
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(failed(error)),
      }
    }
  }

  /// The record the lease is of.
  #[napi(getter)]
  pub fn path(&self) -> String {
    self.path.clone()
  }

  /// The lease ended: the lock is released, and another process may take it.
  #[napi]
  pub fn dispose(&mut self) {
    self.file = None;
  }
}
