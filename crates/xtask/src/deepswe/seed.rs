//! The seed of a task: its base, a checkout of one commit with the dependencies that its image
//! holds, and the checkout of a run, a copy of the base whose environment is its own.

use std::{
  collections::BTreeMap,
  fs, io,
  path::{Path, PathBuf},
};

use serde::Deserialize;

use super::{
  Fault, Rig, captured,
  image::{Held, Image},
  ran, read, removed, say, spoke, written,
};

/// The repository of tasks, fetched once and kept.
const COLLECTION: &str = "https://github.com/datacurve-ai/deep-swe";
/// What a clean of the checkout leaves standing, since its install costs minutes.
const KEEP: [&str; 4] = ["node_modules", ".venv", "target", "vendor"];
/// Each step of a Dockerfile that only a container can take, which the rig steps over.
const CONTAINER: [&str; 5] = ["apt-get", "apt", "apk", "yum", "dnf"];
/// How a step of a Dockerfile is said on this host, in the order that the swaps apply. A login
/// shell of this host puts the paths of the system first, so its python is the one of the host
/// and not the interpreter of the checkout; in the image both shells find the same python.
const SAID: [(&str, &str); 5] = [
  ("bash -lc ", "bash -c "),
  ("python3 -m pip ", "pip "),
  ("python -m pip ", "pip "),
  ("pip3 ", "pip "),
  ("pip ", "python3 -m pip "),
];
/// The format of the environment that a seed makes, which its seal names: a seal of another
/// format makes the base again.
const FORMAT: u32 = 2;

/// One task of the collection, as its `task.toml` names it.
pub(super) struct Task {
  pub(super) name: String,
  pub(super) dir: PathBuf,
  repository: String,
  base: String,
  pub(super) language: String,
  pub(super) image: String,
}

#[derive(Deserialize, Default)]
struct Told {
  #[serde(default)]
  metadata: BTreeMap<String, toml::Value>,
  #[serde(default)]
  environment: BTreeMap<String, toml::Value>,
}

/// A checkout that a seed installs: its folder, the environment of the image, and how a step of a
/// Dockerfile is said on this host.
struct Checkout<'a> {
  repo: &'a Path,
  env: BTreeMap<String, String>,
  swaps: Vec<(String, String)>,
}

/// A seeded task: its workroot, the task, the commit of its base and its image.
pub(super) struct Seeded {
  pub(super) work: PathBuf,
  pub(super) task: Task,
  pub(super) syn: String,
  pub(super) image: Image,
}

impl Seeded {
  pub(super) fn base(&self) -> PathBuf {
    self.work.join("base")
  }

  pub(super) fn app(&self) -> PathBuf {
    self.work.join("app")
  }
}

impl Rig {
  /// The task of this name, from the collection, which is fetched once. A task that holds its own
  /// toml says that the collection stands: the reaper of a host takes the files of an old cache
  /// and leaves its folders.
  pub(super) fn task(&self, name: &str) -> Result<Task, Fault> {
    let tasks = self.collection.join("tasks");
    let stands = fs::read_dir(&tasks).is_ok_and(|mut each| {
      each.any(|one| one.is_ok_and(|one| one.path().join("task.toml").is_file()))
    });
    if !stands {
      self.extracted(COLLECTION, "refs/heads/main", &self.collection)?;
    }
    let dir = tasks.join(name);
    if !dir.is_dir() {
      return Err(Fault::NoTask { task: name.to_owned(), tasks });
    }
    let text = String::from_utf8_lossy(&read(&dir.join("task.toml"))?).into_owned();
    let told: Told =
      toml::from_str(&text).map_err(|source| Fault::Toml { task: name.to_owned(), source })?;
    let named = |table: &BTreeMap<String, toml::Value>, key: &'static str| {
      let value = table.get(key).and_then(toml::Value::as_str).unwrap_or_default();
      if value.is_empty() {
        Err(Fault::Unnamed { task: name.to_owned(), key })
      } else {
        Ok(value.to_owned())
      }
    };
    Ok(Task {
      name: name.to_owned(),
      repository: named(&told.metadata, "repository_url")?,
      base: named(&told.metadata, "base_commit_hash")?,
      language: named(&told.metadata, "language")?,
      image: named(&told.environment, "docker_image")?,
      dir,
    })
  }

  /// All that a run wants before a model is asked: the base, its dependencies and the checkout.
  /// The base is kept when its seal agrees, no tracked file is gone and its python stands.
  pub(super) fn seeded(&self, name: &str, keep_app: bool) -> Result<Seeded, Fault> {
    let task = self.task(name)?;
    let work = self.work.join(name);
    let (base, app, seal) = (work.join("base"), work.join("app"), work.join(".seeded"));
    fs::create_dir_all(&work).map_err(|source| Fault::File { path: work.clone(), source })?;
    let image = Image::of(self, &task.image)?;
    let quoted = |text: &str| serde_json::Value::from(text).to_string();
    let stamp = format!(
      r#"{{"base": {}, "environment": {FORMAT}, "image": {}}}"#,
      quoted(&task.base),
      quoted(&image.digest)
    );
    let python = task.language != "python" || base.join(".venv/bin/python").is_file();
    if !(python && self.whole(&base, &seal, &stamp)) {
      removed(&seal)?;
      self.based(&task, &base)?;
      say!("installing dependencies ({}, from the Dockerfile of the task)", task.language);
      self.installed(&task, &base, &image)?;
      written(&seal, stamp.as_bytes())?;
    }
    self.submodules(&task, &base)?;
    let syn = spoke(self.git(&base).args(["rev-parse", "HEAD"]))?;
    say!("base {}, whose tree is upstream {}", short(&syn, 10), short(&task.base, 10));
    let kept = keep_app
      && captured(self.git(&app).args(["rev-parse", "--verify", "-q", "HEAD"]))?.status.success();
    if kept {
      say!("keeping the checkout as it stands");
    } else {
      say!("seeding the checkout from the base");
      self.pristine(&base, &syn, &app)?;
    }
    Ok(Seeded { work, task, syn, image })
  }

  /// Whether the base stands as its seed left it: the seal names its commit and its image, and no
  /// tracked file is gone. The seal is written last, so a seed that stopped halfway leaves none.
  fn whole(&self, base: &Path, seal: &Path, stamp: &str) -> bool {
    fs::read_to_string(seal).is_ok_and(|said| said == stamp)
      && captured(self.git(base).args(["diff", "--quiet", "--diff-filter=D", "HEAD", "--"]))
        .is_ok_and(|done| done.status.success())
  }

  /// A tree of GitHub at one ref, with no clone: the tarball of codeload, unwrapped where it
  /// stands.
  fn extracted(&self, url: &str, at: &str, into: &Path) -> Result<(), Fault> {
    let slug = url.trim_start_matches("https://github.com/").trim_end_matches(".git");
    say!("codeload {slug} at {} into {}", short(at, 12), into.display());
    removed(into)?;
    fs::create_dir_all(into).map_err(|source| Fault::File { path: into.to_owned(), source })?;
    let name = into.file_name().map(|one| one.to_string_lossy().into_owned()).unwrap_or_default();
    let tgz = into.with_file_name(format!(".{name}.tar.gz"));
    let fetch = [
      "-fsSL",
      "--retry",
      "6",
      "--retry-delay",
      "3",
      "--retry-all-errors",
      "--retry-max-time",
      "240",
    ];
    let url = format!("https://codeload.github.com/{slug}/tar.gz/{at}");
    if !ran(self.command("curl").args(fetch).arg(&url).arg("-o").arg(&tgz))? {
      return Err(Fault::Fetch(url));
    }
    // codeload wraps the tree in one folder named for the repository and the ref.
    let mut unpacked = self.command("tar");
    unpacked.arg("xzf").arg(&tgz).arg("-C").arg(into).arg("--strip-components=1");
    if !ran(&mut unpacked)? {
      return Err(Fault::Unpacked(tgz));
    }
    removed(&tgz)
  }

  /// The repository at one commit, as a repository of one commit, so a submission is a diff
  /// against a base that is real.
  fn based(&self, task: &Task, into: &Path) -> Result<(), Fault> {
    self.extracted(&task.repository, &task.base, into)?;
    let message = format!("deepswe base: {} at {}", task.repository, task.base);
    let steps: [&[&str]; 5] = [
      &["init", "-q"],
      &["config", "user.email", "deepswe@local"],
      &["config", "user.name", "deepswe"],
      &["add", "-A"],
      &["commit", "-q", "-m", &message, "--no-verify"],
    ];
    for step in steps {
      ran(self.git(into).args(step))?;
    }
    Ok(())
  }

  /// The steps of the Dockerfile of the task, taken in the checkout, so the tree is the one that
  /// its verifier wants. A python task first gets an interpreter of its own, at the version that
  /// its image runs, with each distribution that the image took from an index at its version.
  fn installed(&self, task: &Task, repo: &Path, image: &Image) -> Result<(), Fault> {
    let mut env = self.task_env(&image.env, repo)?;
    env.insert("npm_config_confirm_modules_purge".to_owned(), "false".to_owned());
    // A step of poetry reads and writes its config in the workroot, and no setting of the host.
    env.insert("POETRY_CONFIG_DIR".to_owned(), repo.with_file_name("poetry").display().to_string());
    let swaps = SAID.iter().map(|(was, now)| ((*was).to_owned(), (*now).to_owned())).collect();
    let mut checkout = Checkout { repo, env, swaps };
    let mut dists = BTreeMap::new();
    if task.language == "python" {
      let (version, held) = image.interpreter(self)?;
      let pins = self.pinned(&version, &held, repo)?;
      // A constraint in the environment of pip reaches the builds that pip isolates, where the
      // image had none, so it is told on the command line of each install.
      checkout.swaps.push((
        "python3 -m pip install ".to_owned(),
        format!("python3 -m pip install -c {} ", quote(&pins)),
      ));
      dists = held;
    }
    let told = steps(&task.dir)?;
    if told.is_empty() {
      say!("{}: the Dockerfile says no step; installing by language instead", task.name);
      self.generic(&checkout, &task.language)?;
    }
    for step in &told {
      if CONTAINER.contains(&step.split(' ').next().unwrap_or_default()) {
        say!("over (a container alone takes it): {}", short(step, 80));
        continue;
      }
      let app = fancy_regex::Regex::new(r"(?<![\w./])/app\b")?;
      let command = app
        .try_replacen(&hosted(step, &checkout.swaps), 0, repo.display().to_string())?
        .into_owned();
      self.shell(&checkout, &command, "the step failed; the rig goes on with what did land")?;
    }
    let js = matches!(task.language.as_str(), "typescript" | "javascript");
    if !told.is_empty() && js && !repo.join("node_modules").is_dir() {
      self.generic(&checkout, &task.language)?;
    }
    if task.language == "python" {
      for need in requirements(repo)? {
        let command =
          hosted(&format!("pip install -q -r {}", quote(Path::new(&need))), &checkout.swaps);
        self.shell(
          &checkout,
          &command,
          "the requirements failed; the rig goes on with what did land",
        )?;
      }
    }
    self.rebuilt(repo, &dists)
  }

  /// The interpreter of the checkout at the version of the image, with each distribution that the
  /// image took from an index, and the file of constraints that holds pip to those versions. A
  /// step resolves its requirements now and the image resolved them when it was built, so a free
  /// step takes newer releases than the verifier runs. A distribution that the image built from
  /// the tree or took from a URL is left free, since the step names where it comes from.
  fn pinned(
    &self,
    version: &str,
    held: &BTreeMap<String, Held>,
    repo: &Path,
  ) -> Result<PathBuf, Fault> {
    say!("making the interpreter of the checkout: python {version}, as the image of the task runs");
    let venv = repo.join(".venv");
    let mut made = self.command("uv");
    made.args(["venv", "--no-project", "--seed", "--python", version]).arg(&venv);
    if !ran(&mut made)? {
      return Err(Fault::Install(format!("an interpreter of python {version}")));
    }
    let pinned: Vec<String> = held
      .iter()
      .filter(|(_, one)| one.url.is_empty())
      .map(|(name, one)| format!("{name}=={}", one.version))
      .collect();
    let pins = repo.with_file_name("constraints.txt");
    written(&pins, format!("{}\n", pinned.join("\n")).as_bytes())?;
    say!("installing the {} index distributions of the image", pinned.len());
    let python = venv.join("bin").join("python");
    let mut install = self.command("uv");
    install
      .args(["pip", "install", "-q", "--python"])
      .arg(&python)
      .args(["--no-deps", "-r"])
      .arg(&pins);
    if !ran(&mut install)? {
      return Err(Fault::Install("the distributions of the image".to_owned()));
    }
    Ok(pins)
  }

  /// One command of a seed in a shell of the checkout, with the python of the checkout first. A
  /// command that fails is told, and the seed goes on with what did land.
  fn shell(&self, checkout: &Checkout<'_>, command: &str, failed: &str) -> Result<(), Fault> {
    say!("RUN {}", short(command, 90));
    let (repo, env) = (checkout.repo, &checkout.env);
    let venv = repo.join(".venv").join("bin");
    let first = if venv.is_dir() {
      format!("export PATH=\"{}:$PATH\"; ", venv.display())
    } else {
      String::new()
    };
    if !ran(
      self.command("bash").args(["-lc", &format!("{first}{command}")]).current_dir(repo).envs(env),
    )? {
      say!("  {failed}");
    }
    Ok(())
  }

  /// What is installed for a task whose Dockerfile says nothing that this host can take.
  fn generic(&self, checkout: &Checkout<'_>, language: &str) -> Result<(), Fault> {
    let npm = "npm ci --include=dev --no-audit --no-fund || npm install --no-audit --no-fund";
    let words = match language {
      "typescript" | "javascript" => npm,
      "python" => "python3 -m pip install -q -e '.[dev,test]' || python3 -m pip install -q -e .",
      "go" => "go mod download",
      "rust" => "cargo fetch",
      _ => {
        say!("unknown language {language:?}; nothing installed");
        return Ok(());
      }
    };
    self.shell(
      checkout,
      &hosted(words, &checkout.swaps),
      "the install failed; the rig goes on with what did land",
    )
  }

  /// Each distribution that the image built from its tree, installed again from the tree at the
  /// version that the image holds. The base holds one commit and no tag, so a step builds the tree
  /// at a version that git makes up, and a release that wants a newer one goes over the editable
  /// install of the tree. The version is told to each build backend in this install alone, since a
  /// version told to a step reaches the build of an sdist that the step fetches too.
  fn rebuilt(&self, repo: &Path, dists: &BTreeMap<String, Held>) -> Result<(), Fault> {
    let python = repo.join(".venv").join("bin").join("python");
    for (name, held) in dists {
      let Some(within) = held
        .url
        .strip_prefix("file:///app")
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
      else {
        continue;
      };
      let place = repo.join(within.trim_start_matches('/'));
      say!("{name} {} from the tree, as the image holds it", held.version);
      let mut install = self.command(&python);
      install.args(["-m", "pip", "install", "-q", "--no-deps"]);
      if held.editable {
        install.arg("-e");
      }
      install
        .arg(&place)
        .env("SETUPTOOLS_SCM_PRETEND_VERSION", &held.version)
        .env("PDM_BUILD_SCM_VERSION", &held.version);
      if !ran(&mut install)? {
        return Err(Fault::Install(format!("{name} from {}", place.display())));
      }
    }
    Ok(())
  }

  /// The submodules that a tarball cannot carry, pinned, without which a suite that wants one
  /// cannot even be collected.
  fn submodules(&self, task: &Task, into: &Path) -> Result<(), Fault> {
    if !self.found("gh") {
      return Ok(());
    }
    let slug = task.repository.trim_start_matches("https://github.com/").trim_end_matches(".git");
    let raw = ["api", "-H", "Accept: application/vnd.github.raw"];
    let told = spoke(
      self
        .command("gh")
        .args(raw)
        .arg(format!("repos/{slug}/contents/.gitmodules?ref={}", task.base)),
    )?;
    let mut path = String::new();
    for line in told.lines().map(str::trim) {
      if let Some(one) = line.strip_prefix("path = ") {
        one.clone_into(&mut path);
      } else if let Some(url) = line.strip_prefix("url = ").filter(|_| !path.is_empty()) {
        let place = into.join(&path);
        if !place.join(".git").exists() {
          let jq = r#"select(.type=="submodule") | .sha"#;
          let asked = format!("repos/{slug}/contents/{path}?ref={}", task.base);
          let sha = spoke(self.command("gh").args(["api", &asked, "--jq", jq]))?;
          if !sha.is_empty() {
            say!("submodule {path} from {url} at {}", short(&sha, 10));
            let pinned = ran(self.command("git").args(["clone", "-q", url]).arg(&place))?
              && ran(self.git(&place).args(["checkout", "-q", &sha]))?;
            if !pinned {
              say!("  could not pin {path}; a suite that wants it cannot be collected");
              removed(&place)?;
            }
          }
        }
        path.clear();
      }
    }
    Ok(())
  }

  /// A fresh copy of the base at its commit, whose environment is its own. A venv writes its own
  /// path into the scripts that it installs and into the hook of an editable package, so each name
  /// of the base in the venv of the copy moves to the copy: its scripts then run the python of the
  /// copy, an editable package imports the code of the copy, and an install lands in the copy. A
  /// compiled module that holds the name goes, and python compiles it again.
  pub(super) fn pristine(&self, base: &Path, syn: &str, into: &Path) -> Result<(), Fault> {
    removed(into)?;
    ran(self.command("cp").arg("-a").arg(base).arg(into))?;
    ran(self.git(into).args(["reset", "-q", "--hard", syn]))?;
    let kept: Vec<&str> = KEEP.iter().flat_map(|one| ["-e", one]).collect();
    ran(self.git(into).args(["clean", "-qfd"]).args(kept))?;
    moved_venv(
      &into.join(".venv"),
      base.as_os_str().as_encoded_bytes(),
      into.as_os_str().as_encoded_bytes(),
    )
  }
}

/// Each name of the base in a folder of the venv of a copy, moved to the copy.
fn moved_venv(folder: &Path, was: &[u8], now: &[u8]) -> Result<(), Fault> {
  let Ok(entries) = fs::read_dir(folder) else { return Ok(()) };
  for entry in entries.flatten() {
    let path = entry.path();
    let failed = |source| Fault::File { path: path.clone(), source };
    let kind = entry.file_type().map_err(failed)?;
    if kind.is_dir() {
      moved_venv(&path, was, now)?;
    } else if kind.is_symlink() {
      let to = fs::read_link(&path).map_err(failed)?;
      if let Some(moved) = moved(to.as_os_str().as_encoded_bytes(), was, now) {
        removed(&path)?;
        symlink(Path::new(&*String::from_utf8_lossy(&moved)), &path).map_err(failed)?;
      }
    } else {
      let bytes = read(&path)?;
      match moved(&bytes, was, now) {
        None => {}
        Some(_) if path.extension().is_some_and(|one| one == "pyc") => removed(&path)?,
        Some(_) if bytes.contains(&0) => {
          let into = PathBuf::from(&*String::from_utf8_lossy(now));
          return Err(Fault::Binary { path, into });
        }
        Some(said) => written(&path, &said)?,
      }
    }
  }
  Ok(())
}

fn symlink(target: &Path, path: &Path) -> io::Result<()> {
  #[cfg(unix)]
  {
    use std::os::unix::fs::symlink as link;
    link(target, path)
  }
  #[cfg(windows)]
  {
    use std::os::windows::fs::symlink_file;
    symlink_file(target, path)
  }
}

/// The bytes with each name of the base moved, where the name ends a path, and none when they
/// name no base.
fn moved(said: &[u8], was: &[u8], now: &[u8]) -> Option<Vec<u8>> {
  let (mut out, mut at, mut hit) = (Vec::new(), 0, false);
  let rest = |at: usize| said.get(at..).unwrap_or_default();
  while let Some(found) = rest(at).windows(was.len()).position(|one| one == was) {
    let end = at + found + was.len();
    out.extend_from_slice(rest(at).get(..found).unwrap_or_default());
    let goes_on = said
      .get(end)
      .is_some_and(|one| one.is_ascii_alphanumeric() || matches!(one, b'_' | b'.' | b'-'));
    out.extend_from_slice(if goes_on { was } else { now });
    hit |= !goes_on;
    at = end;
  }
  out.extend_from_slice(rest(at));
  hit.then_some(out)
}

/// The steps of the Dockerfile of a task, less the clone that the tarball stands for.
fn steps(task: &Path) -> Result<Vec<String>, Fault> {
  let file = task.join("environment").join("Dockerfile");
  if !file.is_file() {
    return Ok(Vec::new());
  }
  let text = String::from_utf8_lossy(&read(&file)?).replace("\\\n", " ");
  Ok(
    text
      .lines()
      .filter_map(|line| line.trim().strip_prefix("RUN "))
      .map(|one| one.trim().to_owned())
      .filter(|one| !one.contains("git clone"))
      .collect(),
  )
}

/// Each requirements file at the root of a python checkout whose name holds dev or test, which its
/// Dockerfile may leave out, by name, in order.
pub(super) fn requirements(repo: &Path) -> Result<Vec<String>, Fault> {
  let entries =
    fs::read_dir(repo).map_err(|source| Fault::File { path: repo.to_owned(), source })?;
  let mut found: Vec<String> = entries
    .flatten()
    .map(|one| one.file_name().to_string_lossy().into_owned())
    .filter(|name| name.starts_with("requirements"))
    .filter(|name| Path::new(name).extension().is_some_and(|one| one.eq_ignore_ascii_case("txt")))
    .filter(|name| name.contains("dev") || name.contains("test"))
    .collect();
  found.sort();
  Ok(found)
}

/// A command of a Dockerfile, as this host says it.
fn hosted(command: &str, swaps: &[(String, String)]) -> String {
  swaps.iter().fold(command.to_owned(), |command, (was, now)| command.replace(was, now))
}

/// A path as one word of a shell.
fn quote(path: &Path) -> String {
  format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

/// The first characters of a text, at most this many.
pub(super) fn short(text: &str, most: usize) -> &str {
  text.char_indices().nth(most).and_then(|(at, _)| text.get(..at)).unwrap_or(text)
}
