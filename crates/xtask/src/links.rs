//! The links of the README that each package carries. The registry of each package shows that
//! README and reads no relative link as GitHub does, so each of its links is absolute.

use std::{cmp::Reverse, collections::BTreeSet, fs, ops::Range, path::Path, process::ExitCode};

use fancy_regex::Regex;
use pulldown_cmark::{Event, Options, Parser, Tag};
use serde_json::Value;

use crate::{Fault, git};

const REPOSITORY: &str = "https://github.com/uael/furb";
const RAW: &str = "https://raw.githubusercontent.com/uael/furb/main/";
const IMAGES: [&str; 6] = ["png", "gif", "jpg", "jpeg", "svg", "webp"];

/// A package that a registry shows, by its manifest, and the README that it carries.
struct Package {
  label: String,
  manifest: String,
  readme: String,
}

pub(crate) fn check(root: &Path, fix: bool) -> Result<ExitCode, Fault> {
  let listed = git(root, &["ls-files", "-z"])?;
  let files: BTreeSet<&str> = listed.split('\0').filter(|one| !one.is_empty()).collect();
  let found = packages(root, &files)?;
  let readmes: BTreeSet<&str> = found.iter().map(|one| one.readme.as_str()).collect();
  let mut wrong = Vec::new();
  for readme in readmes {
    wrong.extend(checked(root, readme, &files, fix)?);
  }
  for Package { label, manifest, readme } in &found {
    println!("{label} ({manifest}) carries {readme}");
  }
  if wrong.is_empty() {
    println!("Each link of each README of a package works on its registry.");
    return Ok(ExitCode::SUCCESS);
  }
  for one in &wrong {
    eprintln!("{one}");
  }
  eprintln!("cargo xtask links --fix writes each relative link as absolute.");
  Ok(ExitCode::FAILURE)
}

/// Each package of the tree: a pyproject.toml that names a readme, a Cargo.toml of a package that
/// is published, with the readme that it names or the README.md beside it, and a package.json that
/// is not private, with the README.md beside it.
fn packages(root: &Path, files: &BTreeSet<&str>) -> Result<Vec<Package>, Fault> {
  let mut found = Vec::new();
  for &manifest in files {
    let (folder, name) = manifest.rsplit_once('/').unwrap_or(("", manifest));
    if !["pyproject.toml", "Cargo.toml", "package.json"].contains(&name) {
      continue;
    }
    let path = root.join(manifest);
    let text = fs::read_to_string(&path).map_err(|source| Fault::Read { path, source })?;
    let (label, readme) = match name {
      "package.json" => {
        let package: Value = serde_json::from_str(&text).unwrap_or_default();
        if package.get("private").and_then(Value::as_bool) == Some(true) {
          continue;
        }
        (
          package.get("name").and_then(Value::as_str).map(str::to_owned),
          Some("README.md".to_owned()),
        )
      }
      "Cargo.toml" => {
        let Some(package) =
          toml::from_str::<toml::Table>(&text).ok().and_then(|one| one.get("package").cloned())
        else {
          continue;
        };
        if package.get("publish").and_then(toml::Value::as_bool) == Some(false) {
          continue;
        }
        let readme = package.get("readme").and_then(toml::Value::as_str).unwrap_or("README.md");
        (
          package.get("name").and_then(toml::Value::as_str).map(str::to_owned),
          Some(readme.to_owned()),
        )
      }
      _ => {
        let project =
          toml::from_str::<toml::Table>(&text).ok().and_then(|one| one.get("project").cloned());
        let project = project.unwrap_or(toml::Value::Table(toml::Table::new()));
        let readme = project.get("readme");
        let readme =
          readme.and_then(|one| one.get("file")).or(readme).and_then(toml::Value::as_str);
        (
          project.get("name").and_then(toml::Value::as_str).map(str::to_owned),
          readme.map(str::to_owned),
        )
      }
    };
    if let Some(readme) = readme.map(|one| normal(&joined(folder, &one)))
      && files.contains(readme.as_str())
    {
      found.push(Package {
        label: label.unwrap_or_default(),
        manifest: manifest.to_owned(),
        readme,
      });
    }
  }
  Ok(found)
}

/// The faults of the links of one README, each as `readme:line: ...`, after the README takes the
/// absolute form of each relative link when `fix`.
fn checked(
  root: &Path,
  readme: &str,
  files: &BTreeSet<&str>,
  fix: bool,
) -> Result<Vec<String>, Fault> {
  let path = root.join(readme);
  let text =
    fs::read_to_string(&path).map_err(|source| Fault::Read { path: path.clone(), source })?;
  let (mut wrong, mut edits) = (Vec::new(), Vec::new());
  for at in targets(&text)? {
    let target = text.get(at.clone()).unwrap_or_default();
    let line = text.get(..at.start).unwrap_or_default().matches('\n').count() + 1;
    if target.starts_with('#') || schemed(target) {
      if let Some(ours) = ours(target)
        && !stands(ours, files)
      {
        wrong
          .push((line, format!("{readme}:{line}: {target} points at no file of the repository")));
      }
      continue;
    }
    let (file, anchor) =
      target.split_once('#').map_or((target, ""), |(file, anchor)| (file, anchor));
    let folder = readme.rsplit_once('/').map_or("", |(folder, _)| folder);
    let resolved = normal(&joined(folder, file.split('?').next().unwrap_or(file)));
    if resolved.starts_with("..") || !stands(&resolved, files) {
      wrong.push((
        line,
        format!("{readme}:{line}: {target} is relative and points at no file of the repository"),
      ));
      continue;
    }
    let fixed = absolute(&resolved, anchor, files);
    if !fix {
      wrong.push((
        line,
        format!("{readme}:{line}: {target} is relative, so it breaks on a registry: write {fixed}"),
      ));
    }
    edits.push((at, fixed));
  }
  if fix && !edits.is_empty() {
    let mut text = text;
    edits.sort_by_key(|(at, _)| Reverse(at.start));
    for (at, fixed) in edits {
      text.replace_range(at, &fixed);
    }
    fs::write(&path, text).map_err(|source| Fault::Read { path, source })?;
  }
  wrong.sort_by_key(|(line, _)| *line);
  Ok(wrong.into_iter().map(|(_, one)| one).collect())
}

/// Where each target of a link or an image stands in a README: of a link, an image, a definition of
/// a reference, and a `src` or an `href` of its html. A link in code is no link.
fn targets(text: &str) -> Result<Vec<Range<usize>>, Fault> {
  let html = Regex::new(r#"\b(?:src|href)\s*=\s*["']([^"']*)["']"#)?;
  let within = |span: &Range<usize>, target: &str| {
    text.get(span.clone())?.rfind(target).map(|at| span.start + at..span.start + at + target.len())
  };
  let parsed = Parser::new_ext(text, Options::all()).into_offset_iter();
  let mut found: Vec<Range<usize>> = parsed
    .reference_definitions()
    .iter()
    .filter_map(|(_, one)| within(&one.span, &one.dest))
    .collect();
  for (event, span) in parsed {
    match event {
      Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
        found.extend(within(&span, &dest_url).filter(|_| !dest_url.is_empty()));
      }
      Event::Html(_) | Event::InlineHtml(_) => {
        for one in html.captures_iter(text.get(span.clone()).unwrap_or_default()) {
          if let Some(target) = one?.get(1) {
            found.push(span.start + target.start()..span.start + target.end());
          }
        }
      }
      _ => {}
    }
  }
  found.sort_by_key(|at| at.start);
  found.dedup();
  Ok(found)
}

/// Whether a target names its scheme, as `https:` or `mailto:`.
fn schemed(target: &str) -> bool {
  target.split_once(':').is_some_and(|(scheme, _)| {
    scheme.starts_with(|first: char| first.is_ascii_alphabetic())
      && scheme.chars().all(|one| one.is_ascii_alphanumeric() || "+.-".contains(one))
  })
}

/// The path of this repository that an absolute link names, if it names one.
fn ours(target: &str) -> Option<&str> {
  let blob = format!("{REPOSITORY}/blob/main/");
  let tree = format!("{REPOSITORY}/tree/main/");
  let path =
    [blob.as_str(), tree.as_str(), RAW].iter().find_map(|prefix| target.strip_prefix(prefix))?;
  path.split(['#', '?']).next()
}

fn stands(path: &str, files: &BTreeSet<&str>) -> bool {
  let path = path.trim_matches('/');
  path.is_empty()
    || files.contains(path)
    || files
      .range(path..)
      .take_while(|one| one.starts_with(path))
      .any(|one| one.get(path.len()..).is_some_and(|rest| rest.starts_with('/')))
}

fn absolute(path: &str, anchor: &str, files: &BTreeSet<&str>) -> String {
  let mark = if anchor.is_empty() { String::new() } else { format!("#{anchor}") };
  let image = Path::new(path)
    .extension()
    .is_some_and(|one| IMAGES.iter().any(|kind| one.eq_ignore_ascii_case(kind)));
  if image {
    format!("{RAW}{path}")
  } else {
    let kind = if files.contains(path) { "blob" } else { "tree" };
    format!("{REPOSITORY}/{kind}/main/{path}{mark}")
  }
}

fn joined(folder: &str, path: &str) -> String {
  if folder.is_empty() { path.to_owned() } else { format!("{folder}/{path}") }
}

/// A path with no `.` and each `..` that it can go up, as posix normalizes it.
fn normal(path: &str) -> String {
  let mut kept: Vec<&str> = Vec::new();
  for one in path.split('/') {
    match one {
      "" | "." => {}
      ".." if kept.last().is_some_and(|last| *last != "..") => {
        kept.pop();
      }
      _ => kept.push(one),
    }
  }
  kept.join("/")
}
