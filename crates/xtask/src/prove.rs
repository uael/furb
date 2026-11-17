//! The proof of the crates of furb as a user takes them: as crates.io will give them, from their
//! packs alone.

use std::{
  env, fs, io,
  path::{Path, PathBuf},
  process::{Command as Process, ExitCode, Stdio},
};

use fancy_regex::Regex;
use flate2::read::GzDecoder;

#[cfg(unix)]
use std::os::unix::fs::symlink;

use crate::{Fault, git};

/// What the example chains say on a claude command line that answers from a script.
const SAID: &str = "The notes read well.\nreviewer under root";
/// The two answers of that script.
const WORDS: [&str; 2] =
  [r#""close(\"Login is twice as fast.\")""#, r#""close(\"The notes read well.\")""#];

/// Prove the crates of furb as a user will take them from crates.io, from the packed crates alone.
///
/// crates.io takes no crate that stands on git, and furb stands on the fork of monty by git, so the
/// proof stands in for the day when monty stands on crates.io with what furb needs: it copies this
/// checkout, and there it gives each dependency on monty the version of the fork at the revision
/// that Cargo.toml names. cargo packs the crates of the fork that furb runs at that revision, and
/// furb-wire, furb, furb-host and furb-cli on them. Each packed crate stands in for crates.io as a
/// patch of crates.io, and cargo takes every other crate from crates.io and resolves each version
/// afresh, with no lock of furb, as a user gets them.
pub(crate) fn crates(root: &Path, work: Option<PathBuf>) -> Result<ExitCode, Fault> {
  let work = fresh(&match work {
    Some(work) => work,
    None => kept(root)?,
  })?;
  let manifest = read(&root.join("Cargo.toml"))?;
  let version = workspace_version(&manifest)?;
  let target = work.join("target");
  let (fork, forked) = monty(root, &work, &manifest)?;
  copied(root, &work.join("furb"))?;
  versioned(&work.join("furb/Cargo.toml"), &manifest, &fork)?;
  fs::remove_file(work.join("furb/Cargo.lock"))
    .map_err(|source| Fault::Read { path: work.join("furb/Cargo.lock"), source })?;
  // A crate of crates.io takes no dependency on furb-hack, which only the workspace holds, as
  // `cargo hakari publish` drops it.
  let mut unhacked = cargo(&target);
  ran(unhacked.args(["hakari", "remove-deps", "--yes"]).current_dir(work.join("furb")))?;
  let mut patches = vec!["[patch.crates-io]".to_owned()];
  for one in &forked {
    unpacked(&work, &target.join(format!("package/{one}-{fork}.crate")), &mut patches)?;
  }
  let config = work.join("config.toml");
  let configured = |patches: &[String]| {
    fs::write(&config, patches.join("\n") + "\n")
      .map_err(|source| Fault::Read { path: config.clone(), source })
  };
  let packed = |name: &str, patches: &mut Vec<String>| -> Result<(), Fault> {
    configured(patches)?;
    let mut packing = cargo(&target);
    packing.arg("--config").arg(&config).args(["package", "--quiet", "--no-verify", "-p", name]);
    ran(packing.current_dir(work.join("furb")))?;
    unpacked(&work, &target.join(format!("package/{name}-{version}.crate")), patches)?;
    configured(patches)
  };
  packed("furb-wire", &mut patches)?;
  packed("furb", &mut patches)?;
  demo(root, &work, &version)?;
  packed("furb-host", &mut patches)?;
  packed("furb-cli", &mut patches)?;
  configured(&patches)?;
  let mut install = cargo(&target);
  install.arg("--config").arg(&config).args(["install", "--quiet", "--debug", "--root"]);
  install
    .arg(work.join("bin"))
    .arg("--path")
    .arg(work.join(format!("unpacked/furb-cli-{version}")));
  ran(&mut install)?;
  ran(Process::new(work.join("bin/bin/furb")).arg("--help").stdout(Stdio::null()))?;
  println!(
    "furb {version} packs on monty {fork}, adds to a new crate that resolves its dependencies afresh, runs the example chains, and cargo install gives furb"
  );
  Ok(ExitCode::SUCCESS)
}

/// The folder of the proof, with none of the folders of a proof before.
fn fresh(work: &Path) -> Result<PathBuf, Fault> {
  let made = |path: &Path| {
    fs::create_dir_all(path).map_err(|source| Fault::Read { path: path.to_owned(), source })
  };
  made(work)?;
  let work =
    fs::canonicalize(work).map_err(|source| Fault::Read { path: work.to_owned(), source })?;
  for one in ["monty", "furb", "unpacked", "crates", "demo", "yard", "bin"] {
    let path = work.join(one);
    if path.exists() {
      fs::remove_dir_all(&path).map_err(|source| Fault::Read { path, source })?;
    }
  }
  for one in ["crates", "unpacked", "furb"] {
    made(&work.join(one))?;
  }
  Ok(work)
}

/// The crates of the fork of monty that furb runs, packed at the revision that the manifest names,
/// and the version of the fork.
fn monty(root: &Path, work: &Path, manifest: &str) -> Result<(String, Vec<String>), Fault> {
  let rev =
    Regex::new(r#"(?m)^monty = \{ git = "https://github\.com/uael/monty", rev = "([0-9a-f]*)""#)?
      .captures(manifest)?
      .map(|one| crate::group(&one, 1).to_owned())
      .unwrap_or_default();
  let monty = work.join("monty");
  ran(Process::new("git").args(["clone", "-q", "https://github.com/uael/monty"]).arg(&monty))?;
  ran(Process::new("git").args(["checkout", "-q", &rev]).current_dir(&monty))?;
  let fork = workspace_version(&read(&monty.join("Cargo.toml"))?)?;
  let forked = forked(&read(&root.join("Cargo.lock"))?)?;
  let mut packing = cargo(&work.join("target"));
  packing.args(["package", "--quiet", "--no-verify"]).current_dir(&monty);
  ran(packing.args(forked.iter().flat_map(|one| ["-p", one.as_str()])))?;
  Ok((fork, forked))
}

/// A new crate that takes furb as cargo add writes it and runs the example chains of
/// docs/examples/rust, on its runtime of tokio.
fn demo(root: &Path, work: &Path, version: &str) -> Result<(), Fault> {
  let (target, demo, config) = (work.join("target"), work.join("demo"), work.join("config.toml"));
  ran(cargo(&target).args(["new", "--quiet", "--vcs", "none"]).arg(&demo))?;
  ran(
    cargo(&target).args(["add", "--quiet", "tokio", "--features", "rt,macros"]).current_dir(&demo),
  )?;
  // cargo add reads the index of crates.io and no patch, so the line that it would write for furb
  // is written here.
  let manifest = read(&demo.join("Cargo.toml"))?.replacen(
    "[dependencies]\n",
    &format!("[dependencies]\nfurb = \"{version}\"\n"),
    1,
  );
  fs::write(demo.join("Cargo.toml"), manifest)
    .map_err(|source| Fault::Read { path: demo.join("Cargo.toml"), source })?;
  let main = demo.join("src/main.rs");
  fs::copy(root.join("docs/examples/rust/chains.rs"), &main)
    .map_err(|source| Fault::Read { path: main.clone(), source })?;
  ran(cargo(&target).arg("--config").arg(&config).args(["build", "--quiet"]).current_dir(&demo))?;
  let claude = yard(work, &root.join("src/world/provider/claude/fake-claude.sh"))?;
  chains(
    Process::new(target.join("debug/demo")).current_dir(work.join("yard/work")),
    &claude,
    &work.join("yard"),
  )
}

/// The folder of the run of the chains, with the fake claude and its answers.
fn yard(work: &Path, script: &Path) -> Result<PathBuf, Fault> {
  let fake = work.join("yard/fake");
  for one in [fake.clone(), work.join("yard/work")] {
    fs::create_dir_all(&one).map_err(|source| Fault::Read { path: one.clone(), source })?;
  }
  let claude = fake.join("claude");
  fs::copy(script, &claude).map_err(|source| Fault::Read { path: claude.clone(), source })?;
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755))
      .map_err(|source| Fault::Read { path: claude.clone(), source })?;
  }
  words(&fake)?;
  Ok(claude)
}

fn words(fake: &Path) -> Result<(), Fault> {
  for (n, word) in WORDS.iter().enumerate() {
    let path = fake.join(format!("word.{}", n + 1));
    fs::write(&path, word).map_err(|source| Fault::Read { path, source })?;
  }
  Ok(())
}

/// The run of the example chains, which must say what they say on the fake claude.
fn chains(run: &mut Process, claude: &Path, yard: &Path) -> Result<(), Fault> {
  for name in ["FURB_LOCAL_URL", "FURB_LOCAL_KEY", "FURB_WINDOW"] {
    run.env_remove(name);
  }
  run
    .env("FURB_CLAUDE_BIN", claude)
    .env("FURB_CONFIG_DIR", yard.join("config"))
    .env("XDG_CACHE_HOME", yard.join("cache"));
  let out = run
    .stdin(Stdio::null())
    .output()
    .map_err(|source| Fault::Spawn { program: "the example chains".to_owned(), source })?;
  let said = String::from_utf8_lossy(&out.stdout);
  if said.trim_end_matches('\n') != SAID {
    return Err(Fault::Unproven(format!("the chains said:\n{said}")));
  }
  Ok(())
}

/// The folder of a proof when the caller names none: one for each tree, which stays between the
/// runs, so only the first build of the proof is cold.
fn kept(root: &Path) -> Result<PathBuf, Fault> {
  let tree =
    fs::canonicalize(root).map_err(|source| Fault::Read { path: root.to_owned(), source })?;
  let name = tree.file_name().map_or_else(|| "root".into(), |name| name.to_string_lossy());
  Ok(env::temp_dir().join(format!("prove-crate-{name}")))
}

/// cargo with its target in the folder of the proof.
fn cargo(target: &Path) -> Process {
  let mut cargo = Process::new("cargo");
  cargo.env("CARGO_TARGET_DIR", target);
  cargo
}

fn ran(command: &mut Process) -> Result<(), Fault> {
  let program = command.get_program().to_string_lossy().into_owned();
  let status =
    command.status().map_err(|source| Fault::Spawn { program: program.clone(), source })?;
  if status.success() {
    Ok(())
  } else {
    Err(Fault::Unproven(format!("{program} failed: {status}")))
  }
}

fn read(path: &Path) -> Result<String, Fault> {
  fs::read_to_string(path).map_err(|source| Fault::Read { path: path.to_owned(), source })
}

fn workspace_version(manifest: &str) -> Result<String, Fault> {
  let table: toml::Table = toml::from_str(manifest)
    .map_err(|fault| Fault::Unproven(format!("a manifest is no toml: {fault}")))?;
  let version = table.get("workspace").and_then(|one| one.get("package")?.get("version")?.as_str());
  Ok(version.unwrap_or_default().to_owned())
}

/// The crates of the fork that furb runs: those that its lock takes from the fork.
fn forked(lock: &str) -> Result<Vec<String>, Fault> {
  let table: toml::Table = toml::from_str(lock)
    .map_err(|fault| Fault::Unproven(format!("Cargo.lock is no toml: {fault}")))?;
  let packages = table.get("package").and_then(toml::Value::as_array).into_iter().flatten();
  Ok(
    packages
      .filter(|one| {
        one
          .get("source")
          .and_then(toml::Value::as_str)
          .is_some_and(|source| source.starts_with("git+https://github.com/uael/monty?"))
      })
      .filter_map(|one| one.get("name").and_then(toml::Value::as_str).map(str::to_owned))
      .collect(),
  )
}

/// Each file that git tracks, as it stands now, copied into the folder.
fn copied(root: &Path, into: &Path) -> Result<(), Fault> {
  let listed = git(root, &["ls-files", "-z"])?;
  for name in listed.split('\0').filter(|one| !one.is_empty()) {
    let (from, to) = (root.join(name), into.join(name));
    if let Some(folder) = to.parent() {
      fs::create_dir_all(folder)
        .map_err(|source| Fault::Read { path: folder.to_owned(), source })?;
    }
    let failed = |source| Fault::Read { path: from.clone(), source };
    // A file that the change deletes and git still tracks stands no more.
    let Ok(kind) = fs::symlink_metadata(&from) else { continue };
    if kind.is_symlink() {
      linked(&fs::read_link(&from).map_err(failed)?, &to).map_err(failed)?;
    } else if kind.is_file() {
      fs::copy(&from, &to).map_err(failed)?;
    }
  }
  Ok(())
}

#[cfg(unix)]
fn linked(target: &Path, at: &Path) -> io::Result<()> {
  symlink(target, at)
}

/// Windows takes no link with no right of its own, and no proof of the crate runs there.
#[cfg(not(unix))]
#[expect(clippy::unnecessary_wraps, reason = "it stands in for the link of unix, which can fail")]
fn linked(_target: &Path, _at: &Path) -> io::Result<()> {
  Ok(())
}

/// The manifest of the copy, where each dependency on monty by git takes the version of the fork.
fn versioned(path: &Path, manifest: &str, fork: &str) -> Result<(), Fault> {
  let line = Regex::new(
    r#"(?m)^(monty[a-z-]* = \{ git = "https://github\.com/uael/monty", rev = "[0-9a-f]*") \}$"#,
  )?;
  let made =
    line.replace_all(manifest, format!("$1, version = \"{fork}\" }}").as_str()).into_owned();
  let (taken, named) = (
    made.matches(&format!("version = \"{fork}\" }}")).count(),
    manifest.matches("git = \"https://github.com/uael/monty\"").count(),
  );
  if taken != named {
    return Err(Fault::Unproven(format!(
      "{taken} of the {named} dependencies on monty take the version of the fork"
    )));
  }
  fs::write(path, made).map_err(|source| Fault::Read { path: path.to_owned(), source })
}

/// A packed crate, unpacked as a patch of crates.io, and kept in crates/.
fn unpacked(work: &Path, packed: &Path, patches: &mut Vec<String>) -> Result<(), Fault> {
  let failed = |source| Fault::Read { path: packed.to_owned(), source };
  let file = fs::File::open(packed).map_err(failed)?;
  tar::Archive::new(GzDecoder::new(file)).unpack(work.join("unpacked")).map_err(failed)?;
  let name = packed.file_stem().map(|one| one.to_string_lossy().into_owned()).unwrap_or_default();
  fs::copy(packed, work.join("crates").join(format!("{name}.crate"))).map_err(failed)?;
  let package = name.rsplit_once('-').map_or(name.as_str(), |(package, _)| package);
  patches
    .push(format!("{package} = {{ path = \"{}\" }}", work.join("unpacked").join(&name).display()));
  Ok(())
}
