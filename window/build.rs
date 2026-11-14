//! The native library of OpenTUI, built with Zig from the commit that `Cargo.toml` pins, and
//! linked.
//!
//! The crates of OpenTUI link a library that they do not build, and a git dependency holds no build
//! of it. Cargo checks out the whole repository of the pin, so this script copies its
//! `packages/native` to a scratch folder, unpacks the Zig dependencies that the copy vendors, and
//! runs `zig build` with no video, which needs no FFmpeg and no bun. `build.zig` refuses each Zig
//! but the one that its `build.zig.zon` names.
//!
//! The library stands in `<target>/opentui/<commit>`, outside the folder of a profile: each profile
//! of a tree links one build, a copy of the target carries it, and the gates cache that one folder.
//! The scratch folder goes once the library stands.
//!
//! `.cargo/config.toml` turns the build script of `opentui-sys` off. That script only checks that a
//! library stands, and cargo runs it before this one or after it, since no edge orders them. So
//! this script says the link in its place.

use std::{
  env, fs, io,
  path::{Path, PathBuf},
  process::{self, Command, ExitCode},
};

use serde_json::Value;

#[derive(Debug, thiserror::Error)]
enum Fault {
  #[error("cargo gives the build no variable {0}")]
  Unset(&'static str),
  #[error(
    "the window builds the native library of OpenTUI for the system of the build alone, and \
     this build is for {target} on {host}"
  )]
  Cross { host: String, target: String },
  #[error("`cargo metadata` failed: {0}")]
  Metadata(String),
  #[error("`cargo metadata` does not name the package opentui-sys with its commit")]
  Unpinned,
  #[error("{} cannot be read or written: {source}", path.display())]
  File { path: PathBuf, source: io::Error },
  #[error("`tar` did not start ({0}), and it unpacks the Zig dependencies of OpenTUI")]
  NoTar(io::Error),
  #[error(
    "`zig` did not start ({source}). The window builds the native library of OpenTUI with Zig \
     {wanted}: install it, and put `zig` on PATH"
  )]
  NoZig { wanted: String, source: io::Error },
  #[error(
    "the `zig` on PATH is Zig {found}, and the native library of OpenTUI builds with Zig {wanted} \
     alone: install it, and put it first on PATH"
  )]
  OtherZig { found: String, wanted: String },
  #[error("`{program}` failed in {}", folder.display())]
  Failed { program: &'static str, folder: PathBuf },
}

fn main() -> ExitCode {
  match linked() {
    Ok(()) => ExitCode::SUCCESS,
    Err(fault) => {
      eprintln!("{fault}");
      ExitCode::FAILURE
    }
  }
}

/// The library of the pin, built when the target holds none, and the link of each program of the
/// window with it.
fn linked() -> Result<(), Fault> {
  println!("cargo::rerun-if-changed=build.rs");
  println!("cargo::rerun-if-changed=Cargo.toml");
  let (host, target) = (var("HOST")?, var("TARGET")?);
  if host != target {
    return Err(Fault::Cross { host, target });
  }
  let pin = Pin::read(&target)?;
  let windows = target.contains("-windows-");
  let files: &[&str] = if windows { &["opentui.dll", "opentui.lib"] } else { &["libopentui.a"] };
  if !files.iter().all(|file| pin.home.join(file).is_file()) {
    pin.build(files)?;
  }
  for file in files {
    println!("cargo::rerun-if-changed={}", pin.home.join(file).display());
  }
  println!("cargo::rustc-link-search=native={}", pin.home.display());
  for library in libraries(&target, windows) {
    println!("cargo::rustc-link-lib={library}");
  }
  Ok(())
}

/// What a program links with the library, as the build script of `opentui-sys` says it for a
/// library with no video. The archive is not packed into the crate of the window, so the linker
/// reads it after each crate that calls it, whatever its order of search. Windows links the DLL,
/// since the archive of OpenTUI for Windows is one of MinGW, which no Rust toolchain of Windows
/// links.
fn libraries(target: &str, windows: bool) -> Vec<&'static str> {
  if windows {
    return vec!["dylib=opentui"];
  }
  let mut all = vec!["static:-bundle=opentui"];
  if target.ends_with("-apple-darwin") {
    // The clipboard and the images, then the audio, then the C++ of Yoga and the calls of AppKit.
    all.extend(["framework=AppKit", "framework=Foundation", "framework=ImageIO"]);
    all.extend(["framework=CoreFoundation", "framework=CoreAudio", "framework=AudioToolbox"]);
    all.extend(["c++", "objc"]);
  } else if target.ends_with("-gnu") {
    // The archive for Linux carries the libc++ of Zig.
    all.extend(["m", "dl", "pthread"]);
  }
  all
}

/// The commit of OpenTUI that the lock holds: where cargo checked out its native package, and where
/// its library stands in the target.
struct Pin {
  native: PathBuf,
  home: PathBuf,
}

impl Pin {
  /// The pin as `cargo metadata` says it, for the packages of one system alone, which the build
  /// holds already: the folder of `opentui-sys`, the commit after the `#` of its source, and the
  /// target.
  fn read(target: &str) -> Result<Self, Fault> {
    let cargo = var("CARGO")?;
    let asked = Command::new(&cargo)
      .args(["metadata", "--format-version", "1", "--locked", "--filter-platform", target])
      .output()
      .map_err(|source| Fault::File { path: cargo.into(), source })?;
    if !asked.status.success() {
      return Err(Fault::Metadata(String::from_utf8_lossy(&asked.stderr).into_owned()));
    }
    let metadata: Value =
      serde_json::from_slice(&asked.stdout).map_err(|no| Fault::Metadata(no.to_string()))?;
    let packages = metadata["packages"].as_array().into_iter().flatten();
    let found = packages.filter(|package| package["name"] == "opentui-sys").find_map(|package| {
      // The manifest stands at `packages/rust/crates/opentui-sys/Cargo.toml` of the checkout.
      let packages = Path::new(package["manifest_path"].as_str()?).ancestors().nth(4)?;
      let (_, commit) = package["source"].as_str()?.rsplit_once('#')?;
      Some((packages.join("native"), commit.to_owned()))
    });
    let (native, commit) = found.ok_or(Fault::Unpinned)?;
    let target = metadata["target_directory"].as_str().ok_or(Fault::Unpinned)?;
    Ok(Self { native, home: Path::new(target).join("opentui").join(commit) })
  }

  /// The library built in a scratch folder of this build, then moved to its home in one step, so a
  /// build that stops leaves no part of a library there, and two builds at once leave one whole.
  fn build(&self, files: &[&str]) -> Result<(), Fault> {
    self.zig()?;
    let scratch = PathBuf::from(var("OUT_DIR")?).join("opentui");
    // What a build that stopped left.
    if scratch.exists() {
      fs::remove_dir_all(&scratch).map_err(at(&scratch))?;
    }
    let source = scratch.join("native");
    copied(&self.native, &source)?;
    let deps = source.join("zig-deps");
    fs::create_dir(&deps).map_err(at(&deps))?;
    // Both names are relative to the folder: the tar of GNU reads `C:/...` as a remote host.
    let untar = ["-xzf", "src/vendor/zig-deps.tar.gz", "-C", "zig-deps"];
    run("tar", Command::new("tar").args(untar), &source).map_err(Fault::NoTar)??;
    let jobs = format!("-j{}", var("NUM_JOBS")?);
    let mut zig = Command::new("zig");
    zig.args(["build", "-Dvideo=false", "-Doptimize=ReleaseFast", &jobs]);
    zig.arg("--prefix").arg(scratch.join("zig-out"));
    zig.arg("--cache-dir").arg(scratch.join("zig-cache"));
    zig.env("ZIG_GLOBAL_CACHE_DIR", scratch.join("zig-global"));
    run("zig", &mut zig, &source).map_err(|source| self.no_zig(source))??;
    // `build.zig` installs one folder, named after the system, beside its prefix.
    let installed = scratch.join("lib");
    let made = fs::read_dir(&installed).map_err(at(&installed))?.flatten().next();
    let made = made.ok_or(Fault::Failed { program: "zig", folder: installed })?.path();
    let fresh = self.home.with_extension(process::id().to_string());
    fs::create_dir_all(&fresh).map_err(at(&fresh))?;
    for file in files {
      fs::copy(made.join(file), fresh.join(file)).map_err(at(&made.join(file)))?;
    }
    if let Err(source) = fs::rename(&fresh, &self.home) {
      fs::remove_dir_all(&fresh).map_err(at(&fresh))?;
      // A home that another build made in the meantime holds the same library.
      if !files.iter().all(|file| self.home.join(file).is_file()) {
        return Err(Fault::File { path: self.home.clone(), source });
      }
    }
    fs::remove_dir_all(&scratch).map_err(at(&scratch))
  }

  /// The Zig that the commit builds with, which its `build.zig.zon` names.
  fn wanted(&self) -> Option<String> {
    let zon = fs::read_to_string(self.native.join("build.zig.zon")).ok()?;
    let (_, rest) = zon.split_once(".minimum_zig_version = \"")?;
    Some(rest.split('"').next()?.to_owned())
  }

  /// The `zig` on PATH checked against the Zig of the commit. `build.zig` takes no other, and an
  /// older Zig fails on it with an error that names no version.
  fn zig(&self) -> Result<(), Fault> {
    let said = Command::new("zig").arg("version").output().map_err(|source| self.no_zig(source))?;
    let found = String::from_utf8_lossy(&said.stdout).trim().to_owned();
    match self.wanted() {
      Some(wanted) if wanted != found => Err(Fault::OtherZig { found, wanted }),
      _ => Ok(()),
    }
  }

  fn no_zig(&self, source: io::Error) -> Fault {
    let wanted = self.wanted().unwrap_or_else(|| "of its build.zig.zon".to_owned());
    Fault::NoZig { wanted, source }
  }
}

/// A program run in a folder to its end, with what it says on the output of the build: whether it
/// started, then whether it passed.
fn run(
  program: &'static str,
  command: &mut Command,
  folder: &Path,
) -> io::Result<Result<(), Fault>> {
  let status = command.current_dir(folder).status()?;
  let failed = Fault::Failed { program, folder: folder.to_path_buf() };
  Ok(if status.success() { Ok(()) } else { Err(failed) })
}

/// A folder copied with each folder and each file beneath it.
fn copied(from: &Path, to: &Path) -> Result<(), Fault> {
  fs::create_dir_all(to).map_err(at(to))?;
  for entry in fs::read_dir(from).map_err(at(from))? {
    let entry = entry.map_err(at(from))?;
    let (source, copy) = (entry.path(), to.join(entry.file_name()));
    if source.is_dir() {
      copied(&source, &copy)?;
    } else {
      fs::copy(&source, &copy).map_err(at(&source))?;
    }
  }
  Ok(())
}

fn var(name: &'static str) -> Result<String, Fault> {
  env::var(name).ok().ok_or(Fault::Unset(name))
}

fn at(path: &Path) -> impl FnOnce(io::Error) -> Fault + '_ {
  move |source| Fault::File { path: path.to_path_buf(), source }
}
