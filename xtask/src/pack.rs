//! The task `pack`, which fails on each file that the build of a published crate reads and that
//! its pack leaves out.
//!
//! A pack holds only the files that the `include` of its manifest names, so a file that the build
//! reads and the include leaves out builds in the workspace and fails in the crate that a user
//! adds. cargo writes beside each library and program that it builds the files that its build
//! read: each source, each file of `include_str!`, and each file that its build script watches.
//! The task builds the published crates, which a warm target does at once, and reads each of those
//! files against the list of the pack, as `cargo package --list` gives it.

use std::{
  collections::BTreeSet,
  env, fs,
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode},
};

use serde_json::Value;

use crate::Fault;

/// Print each file that the build of a published crate of the workspace reads and that its pack
/// leaves out, as `<crate>: <path>`, and fail when one stands. The workspace is the one of the
/// folder where the task runs, as for a command of cargo.
pub(crate) fn check() -> Result<ExitCode, Fault> {
  let metadata: Value =
    serde_json::from_str(&cargo(&["metadata", "--no-deps", "--format-version", "1"])?)
      .map_err(|no| Fault::Cargo { args: "metadata".to_owned(), stderr: no.to_string() })?;
  let packages = metadata["packages"].as_array().cloned().unwrap_or_default();
  let target = PathBuf::from(metadata["target_directory"].as_str().unwrap_or("target"));
  let root = PathBuf::from(metadata["workspace_root"].as_str().unwrap_or("."));
  let folders: Vec<PathBuf> = packages.iter().map(folder).collect();
  // A crate whose `publish` is an empty list is never packed.
  let published: Vec<&Value> = packages
    .iter()
    .filter(|one| one["publish"].as_array().is_none_or(|to| !to.is_empty()))
    .collect();
  let mut build = vec!["build", "--quiet"];
  for one in &published {
    build.extend(["-p", one["name"].as_str().unwrap_or_default()]);
  }
  cargo(&build)?;
  let mut left = Vec::new();
  for package in published {
    let (name, home) = (package["name"].as_str().unwrap_or_default(), folder(package));
    let listed = cargo(&["package", "--list", "--allow-dirty", "--quiet", "-p", name])?;
    let listed: BTreeSet<&str> = listed.lines().collect();
    for read in
      package["targets"].as_array().into_iter().flatten().filter_map(|one| built(&target, one))
    {
      for file in read_by(&read, &root)? {
        // A file of another crate of the workspace is the pack of that crate.
        let owner = folders
          .iter()
          .filter(|one| file.starts_with(one))
          .max_by_key(|one| one.as_os_str().len());
        if owner != Some(&home) || file.starts_with(&target) || !file.is_file() {
          continue;
        }
        let path = file.strip_prefix(&home).unwrap_or(&file);
        let path = path
          .components()
          .map(|one| one.as_os_str().to_string_lossy())
          .collect::<Vec<_>>()
          .join("/");
        if !listed.contains(path.as_str()) {
          left.push(format!("{name}: {path}"));
        }
      }
    }
  }
  left.sort();
  left.dedup();
  for one in &left {
    println!("{one} is read by the build of the crate and left out of its pack");
  }
  Ok(if left.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

/// The folder of a crate, where its manifest stands.
fn folder(package: &Value) -> PathBuf {
  let manifest = Path::new(package["manifest_path"].as_str().unwrap_or_default());
  manifest.parent().map(Path::to_path_buf).unwrap_or_default()
}

/// The file of what cargo read to build a library or a program, which it writes beside it.
fn built(target: &Path, one: &Value) -> Option<PathBuf> {
  let kinds = one["kind"].as_array()?;
  let name = one["name"].as_str()?;
  let file = if kinds.iter().any(|kind| kind == "lib") {
    format!("lib{}.d", name.replace('-', "_"))
  } else if kinds.iter().any(|kind| kind == "bin") {
    format!("{name}.d")
  } else {
    return None;
  };
  Some(target.join("debug").join(file))
}

/// Each file that a file of cargo names as read, as a path of the machine. A space in a path is
/// written `\ `, and a path that is not whole stands under the root of the workspace.
fn read_by(file: &Path, root: &Path) -> Result<Vec<PathBuf>, Fault> {
  let text =
    fs::read_to_string(file).map_err(|source| Fault::Read { path: file.to_owned(), source })?;
  let first = text.lines().next().unwrap_or_default();
  let read = first.split_once(": ").map_or("", |(_, read)| read);
  let paths = read.replace("\\ ", "\0");
  Ok(
    paths
      .split(' ')
      .filter(|one| !one.is_empty())
      .map(|one| root.join(one.replace('\0', " ")))
      .collect(),
  )
}

/// What a command of cargo says, or why it failed.
fn cargo(args: &[&str]) -> Result<String, Fault> {
  let program = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
  let done = Process::new(&program)
    .args(args)
    .output()
    .map_err(|source| Fault::Spawn { program: "cargo".to_owned(), source })?;
  if !done.status.success() {
    let why = String::from_utf8_lossy(&done.stderr);
    return Err(Fault::Cargo { args: args.join(" "), stderr: why.trim().to_owned() });
  }
  Ok(String::from_utf8_lossy(&done.stdout).into_owned())
}
