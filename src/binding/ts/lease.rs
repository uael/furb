//! One process at a time owns a record, by a lease that the kernel holds for it.
use napi_derive::napi;
use std::fs::{File, OpenOptions, TryLockError, create_dir_all};
use std::path::Path;

/// The lease of one record: a lock on the file beside the record, `<record>.lock`, which the process holds until
/// it disposes the lease or ends. The kernel ends the lease of a process that ends, whatever the process number
/// says after it. The file stays when the lease ends, since a lease that removed its file by name could remove
/// the file that another process has locked since.
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
    let failed = |error: std::io::Error| napi::Error::from_reason(format!("{error}: {path}.lock"));
    if let Some(parent) = Path::new(&path).parent() {
      create_dir_all(parent).map_err(failed)?;
    }
    let file = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .truncate(false)
      .open(format!("{path}.lock"))
      .map_err(failed)?;
    match file.try_lock() {
      Ok(()) => Ok(Self { file: Some(file), path }),
      Err(TryLockError::WouldBlock) => {
        Err(napi::Error::from_reason(format!("Another process owns {path}.")))
      }
      Err(TryLockError::Error(error)) => Err(failed(error)),
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
