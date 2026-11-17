//! The image of a task, read from its registry with no pull and no docker: the environment that
//! it sets, and the python that its verifier runs.

use std::{
  collections::BTreeMap,
  io::{self, BufRead, BufReader, Read},
  path::{Path, PathBuf},
  process::{ChildStdout, Command, Stdio},
};

use fancy_regex::{Captures, Regex, escape};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use super::{Fault, Rig, captured};

/// The bounds of each read of a registry: a read that moves nothing for a minute ends, as a read
/// that stalls would hold the rig for ever, and a layer of gigabytes still reads whole.
const BOUNDS: [&str; 6] = ["--connect-timeout", "60", "--speed-limit", "1", "--speed-time", "60"];
/// The two forms of the manifest of one image that a registry is asked for.
const MANIFEST: &str = "application/vnd.docker.distribution.manifest.v2+json, application/vnd.oci.image.manifest.v1+json";

/// The image that a task names: the digest of its config, its layers and its environment, read
/// once for a verb.
pub(super) struct Image {
  pub(super) named: String,
  pub(super) digest: String,
  layers: Vec<String>,
  pub(super) env: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Manifest {
  config: Option<Blob>,
  #[serde(default)]
  layers: Vec<Blob>,
}

#[derive(Deserialize)]
struct Blob {
  digest: String,
}

#[derive(Deserialize)]
struct Config {
  #[serde(default)]
  config: Settings,
}

#[derive(Deserialize, Default)]
struct Settings {
  #[serde(rename = "Env", default)]
  env: Option<Vec<String>>,
}

/// What one layer of an image does to its python: each distribution that it writes, by the path
/// of its dist-info, and each path that it takes away, where a path that ends with a slash takes
/// its whole folder. The rig keeps it under `layers`, so each layer is read once: its names are
/// those of each file that the rig wrote before.
#[derive(Serialize, Deserialize, Default)]
struct Layer {
  dists: BTreeMap<String, Dist>,
  gone: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Dist {
  #[serde(default)]
  url: String,
  #[serde(default)]
  editable: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  name: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  version: Option<String>,
}

/// A distribution that the python of an image holds: its version, the URL that it came from,
/// which is empty for one of an index, and whether it was installed editable.
pub(super) struct Held {
  pub(super) version: String,
  pub(super) url: String,
  pub(super) editable: bool,
}

impl Image {
  /// The image of this name, from its registry.
  pub(super) fn of(rig: &Rig, named: &str) -> Result<Self, Fault> {
    let registry = Registry::of(rig, named)?;
    let tag = named.rsplit_once(':').map_or("latest", |(_, tag)| tag);
    let manifest: Manifest = json(&registry.get(&format!("manifests/{tag}"), MANIFEST)?, named)?;
    let config = manifest.config.ok_or_else(|| Fault::Index(named.to_owned()))?;
    let held: Config = json(&registry.get(&format!("blobs/{}", config.digest), "*/*")?, named)?;
    let env = held.config.env.unwrap_or_default();
    Ok(Self {
      named: named.to_owned(),
      digest: config.digest,
      layers: manifest.layers.into_iter().map(|one| one.digest).collect(),
      env: env.iter().map(|one| one.split_once('=').unwrap_or((one, ""))).map(owned).collect(),
    })
  }

  /// The version of the python of the image, and each distribution that its site holds, by name.
  /// The config names the version in `PYTHON_VERSION`, as the official python image sets it, and
  /// the venv in `VIRTUAL_ENV` when the image makes one. The layers say what the site holds, each
  /// in order over the ones before it.
  pub(super) fn interpreter(&self, rig: &Rig) -> Result<(String, BTreeMap<String, Held>), Fault> {
    let set = |key: &str| self.env.get(key).filter(|one| !one.is_empty());
    let version = set("PYTHON_VERSION").ok_or_else(|| Fault::Unversioned(self.named.clone()))?;
    let prefix = set("VIRTUAL_ENV").map_or("usr/local", |one| one.trim_matches('/'));
    let minor: Vec<&str> = version.split('.').take(2).collect();
    let site = format!("{prefix}/lib/python{}/site-packages/", minor.join("."));
    let registry = Registry::of(rig, &self.named)?;
    let mut held: BTreeMap<String, Dist> = BTreeMap::new();
    for digest in &self.layers {
      let layer = layer(rig, &registry, digest)?;
      for path in &layer.gone {
        let folder = format!("{}/", path.trim_end_matches('/'));
        held.retain(|at, _| at != path && !at.starts_with(&folder));
      }
      held.extend(layer.dists);
    }
    let dists = held
      .into_iter()
      .filter(|(at, _)| at.strip_prefix(&site).is_some_and(|rest| !rest.contains('/')))
      .filter_map(|(_, dist)| {
        let held = Held { version: dist.version?, url: dist.url, editable: dist.editable };
        Some((dist.name?, held))
      })
      .collect();
    Ok((version.clone(), dists))
  }
}

fn owned((key, value): (&str, &str)) -> (String, String) {
  (key.to_owned(), value.to_owned())
}

fn json<T: for<'a> Deserialize<'a>>(bytes: &[u8], what: &str) -> Result<T, Fault> {
  serde_json::from_slice(bytes).map_err(|source| Fault::Json { what: what.to_owned(), source })
}

/// The registry of an image, read through curl, with the anonymous token that it asks for.
struct Registry<'a> {
  rig: &'a Rig,
  url: String,
  token: Option<String>,
}

impl<'a> Registry<'a> {
  fn of(rig: &'a Rig, named: &str) -> Result<Self, Fault> {
    let (host, rest) = named.split_once('/').unwrap_or(("", named));
    let repo = rest.rsplit_once(':').map_or(rest, |(repo, _)| repo);
    let probe = captured(
      rig
        .command("curl")
        .args(BOUNDS)
        .args(["-sS", "-o", "/dev/null", "-D", "-"])
        .arg(format!("https://{host}/v2/")),
    )?;
    let headers = String::from_utf8_lossy(&probe.stdout);
    let asked = headers.lines().find(|line| line.to_lowercase().starts_with("www-authenticate:"));
    let token = match asked {
      None => None,
      Some(line) => {
        let told: BTreeMap<String, String> = Regex::new(r#"(\w+)="([^"]*)""#)?
          .captures_iter(line)
          .filter_map(Result::ok)
          .map(|one| (crate::group(&one, 1).to_owned(), crate::group(&one, 2).to_owned()))
          .collect();
        let field = |key: &str| told.get(key).cloned().unwrap_or_default();
        let got = captured(
          rig
            .command("curl")
            .args(BOUNDS)
            .args(["-fsSG", &field("realm"), "--data-urlencode"])
            .arg(format!("service={}", field("service")))
            .arg("--data-urlencode")
            .arg(format!("scope=repository:{repo}:pull")),
        )?;
        let said: serde_json::Value = json(&got.stdout, &format!("the token of {host}"))?;
        said.get("token").and_then(|one| one.as_str()).map(str::to_owned)
      }
    };
    Ok(Self { rig, url: format!("https://{host}/v2/{repo}"), token })
  }

  /// The body of one read, whole.
  fn get(&self, path: &str, accept: &str) -> Result<Vec<u8>, Fault> {
    let got = captured(&mut self.curl(path, accept))?;
    if got.status.success() {
      Ok(got.stdout)
    } else {
      Err(Fault::Fetch(format!("{}/{path}", self.url)))
    }
  }

  fn curl(&self, path: &str, accept: &str) -> Command {
    let mut curl = self.rig.command("curl");
    curl.args(BOUNDS).args(["-fsSL", "-H", &format!("Accept: {accept}")]);
    if let Some(token) = &self.token {
      curl.args(["-H", &format!("Authorization: Bearer {token}")]);
    }
    curl.arg(format!("{}/{path}", self.url));
    curl
  }
}

/// What one layer of the image does to its python, read once and kept under `layers`.
fn layer(rig: &Rig, registry: &Registry<'_>, digest: &str) -> Result<Layer, Fault> {
  let kept = rig.homes.join("layers").join(format!("{}.json", digest.replace(':', "-")));
  if kept.is_file() {
    return json(&super::read(&kept)?, digest);
  }
  let mut curl = registry.curl(&format!("blobs/{digest}"), "*/*");
  let mut child = curl
    .stdout(Stdio::piped())
    .spawn()
    .map_err(|source| Fault::Spawn { program: "curl".to_owned(), source })?;
  let stdout = child.stdout.take().ok_or_else(|| Fault::Fetch(digest.to_owned()))?;
  let read = listed(stdout).map_err(|source| Fault::File { path: kept.clone(), source });
  let ended = child.wait().map_err(|source| Fault::Spawn { program: "curl".to_owned(), source })?;
  let (Ok(layer), true) = (read, ended.success()) else {
    return Err(Fault::Fetch(format!("{}/blobs/{digest}", registry.url)));
  };
  let text = serde_json::to_string(&layer)
    .map_err(|source| Fault::Json { what: digest.to_owned(), source })?;
  super::written(&kept, text.as_bytes())?;
  Ok(layer)
}

/// The distributions and the removals of a layer, from its tar, which may be gzipped.
fn listed(stdout: ChildStdout) -> io::Result<Layer> {
  let mut reader = BufReader::new(stdout);
  let gzipped = reader.fill_buf()?.starts_with(&[0x1f, 0x8b]);
  let stream: Box<dyn Read> =
    if gzipped { Box::new(GzDecoder::new(reader)) } else { Box::new(reader) };
  let mut layer = Layer::default();
  for entry in tar::Archive::new(stream).entries()? {
    let mut entry = entry?;
    let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
    let name = name.strip_prefix("./").unwrap_or(&name);
    let (parent, leaf) = name.rsplit_once('/').unwrap_or(("", name));
    if leaf == ".wh..wh..opq" {
      layer.gone.push(format!("{parent}/"));
    } else if let Some(gone) = leaf.strip_prefix(".wh.") {
      layer.gone.push(format!("{parent}/{gone}"));
    } else if parent.ends_with(".dist-info")
      && matches!(leaf, "METADATA" | "direct_url.json")
      && entry.header().entry_type().is_file()
    {
      let mut text = Vec::new();
      entry.read_to_end(&mut text)?;
      let text = String::from_utf8_lossy(&text);
      let dist = layer.dists.entry(parent.to_owned()).or_default();
      if leaf == "METADATA" {
        dist.name = header(&text, "Name");
        dist.version = header(&text, "Version");
      } else if let Ok(came) = serde_json::from_str::<serde_json::Value>(&text) {
        came
          .get("url")
          .and_then(serde_json::Value::as_str)
          .unwrap_or_default()
          .clone_into(&mut dist.url);
        dist.editable = came
          .pointer("/dir_info/editable")
          .and_then(serde_json::Value::as_bool)
          .unwrap_or_default();
      }
    }
  }
  Ok(layer)
}

/// A header of the metadata of a distribution, which stands before its first blank line.
fn header(text: &str, key: &str) -> Option<String> {
  let start = format!("{key}:");
  let line =
    text.lines().take_while(|line| !line.is_empty()).find(|line| line.starts_with(&start))?;
  Some(line.strip_prefix(&start)?.trim().to_owned())
}

impl Rig {
  /// The environment of the image of a task, with its paths of the project and of the tools moved
  /// to this host, and the python of the checkout first on its PATH, then the tools of the host.
  pub(super) fn task_env(
    &self,
    held: &BTreeMap<String, String>,
    repo: &Path,
  ) -> Result<BTreeMap<String, String>, Fault> {
    let shown = |path: PathBuf| path.display().to_string();
    let paths = BTreeMap::from([
      ("/app", shown(repo.to_owned())),
      ("/opt/venv", shown(repo.join(".venv"))),
      ("/root", shown(self.home.clone())),
      ("/usr/local/cargo", shown(self.home.join(".cargo"))),
      ("/usr/local/rustup", shown(self.home.join(".rustup"))),
      ("/opt/gocache", shown(self.homes.join("go-cache"))),
      ("/deno-cache", shown(self.homes.join("deno"))),
    ]);
    let roots: Vec<_> = paths.keys().map(|one| escape(one)).collect();
    let pattern = Regex::new(&format!(r"(?<![\w.-])({})(?=/|:|$)", roots.join("|")))?;
    let moved = |value: &str| -> Result<String, Fault> {
      let swap = |found: &Captures<'_, str>| paths.get(&found[0]).cloned().unwrap_or_default();
      Ok(pattern.try_replacen(value, 0, swap)?.into_owned())
    };
    let mut env = BTreeMap::new();
    for (key, value) in held {
      env.insert(key.clone(), moved(value)?);
    }
    let venv = repo.join(".venv").join("bin");
    let first = venv.is_dir().then(|| shown(venv));
    let mut path: Vec<String> = Vec::new();
    let image =
      held.get("PATH").map(|one| one.split(':').map(&moved).collect::<Result<Vec<_>, _>>());
    let host = self.path.to_string_lossy().split(':').map(str::to_owned).collect::<Vec<_>>();
    for one in first.into_iter().chain(image.transpose()?.unwrap_or_default()).chain(host) {
      if !path.contains(&one) {
        path.push(one);
      }
    }
    env.insert("PATH".to_owned(), path.join(":"));
    Ok(env)
  }
}
