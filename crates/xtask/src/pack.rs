//! The task `pack`, which fails on each file that the build of a published crate reads and that
//! its pack leaves out.
//!
//! A pack holds only the files that the `include` of its manifest names, so a file that the build
//! reads and the include leaves out builds in the workspace and fails in the crate that a user
//! adds. The build reads a file that a source of the pack names: by `include_str!`,
//! `include_bytes!` or `#[path]`, beside that source, and by a `rerun-if-changed` of its build
//! script, from the folder of the crate. The task reads each source of the pack, as `cargo package
//! --list` gives it, and builds nothing, so it ends in a second and runs before each commit.
#![expect(clippy::indexing_slicing, reason = "debt: an index or a slice that can panic")]

use std::{
  collections::BTreeSet,
  env, fs,
  path::{Component, Path, PathBuf},
  process::{Command as Process, ExitCode},
};

use fancy_regex::Regex;
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
  // A crate whose `publish` is an empty list is never packed.
  let published =
    packages.iter().filter(|one| one["publish"].as_array().is_none_or(|to| !to.is_empty()));
  let mut left = Vec::new();
  for package in published {
    let manifest = Path::new(package["manifest_path"].as_str().unwrap_or_default());
    let (name, home) =
      (package["name"].as_str().unwrap_or_default(), manifest.parent().unwrap_or(Path::new("")));
    let listed = cargo(&["package", "--list", "--allow-dirty", "--quiet", "-p", name])?;
    // cargo lists each path with the separator of the system, which Windows writes `\\`.
    let listed: BTreeSet<String> = listed.lines().map(|one| one.replace('\\', "/")).collect();
    for file in read(home, &listed)? {
      let packed = file.strip_prefix(home).ok().map(|one| {
        one
          .components()
          .map(|part| part.as_os_str().to_string_lossy())
          .collect::<Vec<_>>()
          .join("/")
      });
      if !packed.is_some_and(|one| listed.contains(&one)) {
        let shown =
          file.strip_prefix(home).unwrap_or(&file).display().to_string().replace('\\', "/");
        left.push(format!("{name}: {shown}"));
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

/// Each file that the build of the library and the programs of a crate reads, by the sources of its
/// pack. A source of a test reads nothing that a user builds: a source of the folders `tests`,
/// `benches` and `examples`, and a module that a `#[cfg(test)]` or a `#[cfg(all(test, ..))]`
/// declares, with each module under it. A name in a comment or a string that names no file builds
/// nothing.
fn read(home: &Path, listed: &BTreeSet<String>) -> Result<Vec<PathBuf>, Fault> {
  let declared = Regex::new(r"((?:#\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;")?;
  // `#[cfg(test)]`, or `#[cfg(all(test, ..))]`, which holds a module to the tests alone.
  let only = Regex::new(r"#\[cfg\((?:all\(\s*)?test\b")?;
  let named = Regex::new(r#"(?:include_(?:str|bytes)!\(\s*|#\[path\s*=\s*)"([^"]+)""#)?;
  let watched = Regex::new(r#"rerun-if-changed=([^"{}\s]+)"#)?;
  let mut sources = Vec::new();
  for source in
    listed.iter().filter(|one| Path::new(one).extension().is_some_and(|kind| kind == "rs"))
  {
    let path = home.join(source);
    let text =
      fs::read_to_string(&path).map_err(|source| Fault::Read { path: path.clone(), source })?;
    sources.push((plain(&path), text));
  }
  let mut tests: BTreeSet<PathBuf> = sources
    .iter()
    .filter(|(path, _)| {
      let inside = path.strip_prefix(home).ok().and_then(|one| one.components().next());
      inside.is_some_and(|one| {
        ["tests", "benches", "examples"].contains(&&*one.as_os_str().to_string_lossy())
      })
    })
    .map(|(path, _)| path.clone())
    .collect();
  // A module of a test declares its own modules, so the tests grow until no source joins them.
  loop {
    let before = tests.len();
    for (path, text) in &sources {
      for hit in declared.captures_iter(text.as_str()) {
        let hit = hit?;
        if tests.contains(path) || only.is_match(&hit[1])? {
          tests.extend(module(path, &hit[1], &hit[2]));
        }
      }
    }
    if tests.len() == before {
      break;
    }
  }
  let mut read = Vec::new();
  for (path, text) in sources.iter().filter(|(path, _)| !tests.contains(path)) {
    let folder = path.parent().unwrap_or(home);
    for hit in named.captures_iter(text.as_str()) {
      read.push(plain(&folder.join(&hit?[1])));
    }
    for hit in watched.captures_iter(text.as_str()) {
      read.push(plain(&home.join(&hit?[1])));
    }
  }
  read.retain(|one| one.is_file());
  Ok(read)
}

/// The files that a module that a source declares may stand in: the one that its `#[path]` names
/// beside the source, else `<name>.rs` and `<name>/mod.rs` in the folder of the modules of that
/// source, which is its own folder for a `lib.rs`, a `main.rs` or a `mod.rs`.
fn module(source: &Path, attributes: &str, name: &str) -> Vec<PathBuf> {
  let folder = source.parent().unwrap_or(Path::new(""));
  if let Some(path) = attributes.split("path").nth(1).and_then(|rest| rest.split('"').nth(1)) {
    return vec![plain(&folder.join(path))];
  }
  let stem = source.file_stem().map(|one| one.to_string_lossy()).unwrap_or_default();
  let modules = if ["lib", "main", "mod"].contains(&&*stem) {
    folder.to_path_buf()
  } else {
    folder.join(&*stem)
  };
  vec![modules.join(format!("{name}.rs")), modules.join(name).join("mod.rs")]
}

/// A path with each `.` and `..` read, as the build reads it, with no access to the disk.
fn plain(path: &Path) -> PathBuf {
  let mut plain = PathBuf::new();
  for part in path.components() {
    match part {
      Component::CurDir => {}
      Component::ParentDir => {
        plain.pop();
      }
      other => plain.push(other),
    }
  }
  plain
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
