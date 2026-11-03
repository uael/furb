//! The host of a life, which the core starts as a process of its own: `gui/host/main.ts` of the checkout that the app
//! was built from, or the program that `FURB_GUI_HOST` names, on the bun that `FURB_BUN` names, else on the bun of
//! PATH, else on the bun of the places where its installers put it, since an app that the Finder opens has a short
//! PATH. The host prints the address of its socket as its first line, and ends when its stdin ends.

#![expect(unreachable_pub, reason = "debt: pub where the crate alone uses it")]
#![expect(
  clippy::let_underscore_must_use,
  reason = "debt: a let _ drops a value that must be used"
)]
#![expect(clippy::let_underscore_untyped, reason = "debt: a let _ hides the type of what it drops")]
#![expect(clippy::manual_let_else, reason = "debt: a match where let else serves")]
#![expect(clippy::map_err_ignore, reason = "debt: a map_err that drops the cause")]
#![expect(clippy::map_unwrap_or, reason = "debt: map then unwrap_or, where map_or serves")]
#![expect(
  clippy::single_match_else,
  reason = "debt: a match of one arm and else, where if let serves"
)]

use std::{
  env,
  io::{BufRead, BufReader},
  path::{Path, PathBuf},
  process::{Child, Command, Stdio},
  thread,
};

/// The bun that runs the host, if one is found.
pub fn bun() -> Option<PathBuf> {
  if let Some(named) = env::var_os("FURB_BUN") {
    return Some(named.into());
  }
  let mut folders: Vec<PathBuf> =
    env::var_os("PATH").map(|path| env::split_paths(&path).collect()).unwrap_or_default();
  if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
    folders.push(PathBuf::from(home).join(".bun").join("bin"));
  }
  folders.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
  let name = if cfg!(windows) { "bun.exe" } else { "bun" };
  folders.into_iter().map(|folder| folder.join(name)).find(|path| path.is_file())
}

/// The program of the host.
pub fn program() -> PathBuf {
  env::var_os("FURB_GUI_HOST").map(PathBuf::from).unwrap_or_else(|| {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("host").join("main.ts")
  })
}

/// The address of the socket that the first line of a host gives.
pub fn address(line: &str) -> Result<String, String> {
  let said: serde_json::Value =
    serde_json::from_str(line).map_err(|_| format!("the host said no address: {line}"))?;
  said["url"].as_str().map(str::to_owned).ok_or_else(|| format!("the host said no address: {line}"))
}

/// A host started with the words that it takes, and the address of its socket. What it prints later goes to the
/// stderr of the app.
pub fn start(words: &[String]) -> Result<(Child, String), String> {
  let bun = bun().ok_or(
    "the host of the app runs on bun 1.4.2 or later, and no bun is found: see https://bun.sh",
  )?;
  let program = program();
  if !program.is_file() {
    return Err(format!("there is no host of the app at {}", program.display()));
  }
  let mut child = Command::new(bun)
    .arg(&program)
    .args(words)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    .spawn()
    .map_err(|no| format!("the host of the app did not start: {no}"))?;
  let out = child.stdout.take().ok_or("the host of the app has no stdout")?;
  let mut lines = BufReader::new(out).lines();
  let first = match lines.next() {
    Some(Ok(line)) => line,
    _ => {
      let _ = child.wait();
      return Err("the host of the app ended before it said its address".to_owned());
    }
  };
  let url = address(&first)?;
  thread::spawn(move || {
    for line in lines.map_while(Result::ok) {
      eprintln!("{line}");
    }
  });
  Ok((child, url))
}
