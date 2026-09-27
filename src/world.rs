//! The World that the crate writes: ears of the outside that serve the machine, one for each concern.
//!
//! The contract makes the World the ears of the outside that serve the machine, one or many. These are the ones
//! every host shares, written once in rust: [`files`] reads and writes a path, [`bash`] runs a command, [`time`]
//! reads the clock, draws a chance and ends a wait, and [`store`] keeps the record on the disk under a lease. The
//! host gives the rest, its provider of models and its console, which the crate knows nothing of.
//!
//! Each is an ear like any other, so an ear that comes before one of them in the order of boot takes a question in
//! its place, and an ear that takes a question and asks it again wraps it.

mod bash;
mod files;
mod store;
mod time;

use std::path::{Component, Path, PathBuf};

pub use bash::{SHELL, bash};
pub use files::files;
pub use store::{kept, store};
pub use time::time;

use crate::{
  ear::{Co, call},
  value::{Fault, Object},
};

/// The directory a chain stands in, which a path of it resolves against: where the chain went last, resolved against
/// the directory the life stands on.
async fn here(co: &Co, on: &str) -> Result<PathBuf, Fault> {
  let on = vec![("on", Object::string(on))];
  let cwd = call(co, "cwd", vec![], on).await?;
  let standing = call(co, "standing", vec![], vec![]).await?;
  let directory = standing
    .as_ref()
    .items()
    .and_then(|held| held.get(1).and_then(|one| one.as_str().map(str::to_owned)));
  let cwd = cwd.as_ref().as_str().map(str::to_owned).unwrap_or_default();
  Ok(resolved(Path::new(&directory.unwrap_or_default()), &cwd))
}

/// A path resolved against a directory, as a path of the machine: the directory with the path after it, or the path
/// itself when it is absolute, with every `.` and `..` read.
fn resolved(directory: &Path, path: &str) -> PathBuf {
  let mut out = PathBuf::new();
  for part in directory.join(path).components() {
    match part {
      Component::CurDir => {}
      Component::ParentDir => {
        out.pop();
      }
      other => out.push(other),
    }
  }
  out
}

#[cfg(test)]
#[path = "world.test.rs"]
mod test;
