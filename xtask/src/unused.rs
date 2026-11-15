//! The names that nothing but a test uses: each exported name of the TypeScript side and each
//! public item of the crates that no file but a test names, its own file included.

use std::{
  collections::{BTreeSet, HashMap, HashSet},
  fs,
  path::Path,
  process::ExitCode,
};

use fancy_regex::{Captures, Regex, escape};

use crate::{Fault, git, written};

const PACKAGE: &str = "bind/typescript/src/";
const SOURCES: [&str; 6] = [PACKAGE, "src/", "cli/src/", "host/src/", "window/src/", "build/"];
const WHOLE: [&str; 6] = ["interface", "type", "enum", "struct", "trait", "class"];

/// The items that nothing but a test names, as lines, and those that their own file alone names.
type Found = (Vec<String>, Vec<(String, String)>);

/// An item of a file: its line from 0, its kind, and whether a file that no test is names it.
struct Item {
  name: String,
  line: usize,
  kind: String,
  seen: bool,
}

/// What the sources say, which decides whether each item is used.
struct World {
  imported: HashSet<(String, String)>,
  documented: HashSet<String>,
  words: HashMap<String, HashSet<String>>,
  calls: HashMap<String, HashSet<(String, String)>>,
  ts: HashSet<String>,
  py: HashSet<String>,
}

/// The patterns that the scan reads, made once.
struct Patterns {
  ts: Regex,
  rs: Regex,
  implemented: Regex,
  from: Regex,
  inline: Regex,
  door: Regex,
  word: Regex,
  test: Regex,
}

impl Patterns {
  fn new() -> Result<Self, Fault> {
    Ok(Self {
      ts: Regex::new(
        r"^export (?:declare )?(?:async )?(?:abstract )?(function\*?|const|let|class|interface|type|enum) (\w+)",
      )?,
      rs: Regex::new(
        r"^(\s*)pub (?:const |async |unsafe )*(fn|struct|enum|trait|const|static|type) (\w+)",
      )?,
      implemented: Regex::new(r"^impl(?:<[^>]*>)? (?:[\w:<>, ]+ for )?(\w+)")?,
      from: Regex::new(
        r#"(import|export)\s+(?:type\s+)?(?:\*\s*(?:as\s+(\w+))?|\{([^}]*)\})\s*from\s*"([^"]+)""#,
      )?,
      inline: Regex::new(r#"import\("([^"]+)"\)\.(\w+)"#)?,
      door: Regex::new(r"napi|pyo3|pyclass|pyfunction|pymethods")?,
      word: Regex::new(r"\w+")?,
      test: Regex::new(r"(^|/)(test|tests)/|\.test\.(ts|tsx|rs)$|(^|/)test_[^/]*\.py$")?,
    })
  }

  fn words(&self, text: &str) -> Result<Vec<String>, Fault> {
    self.word.find_iter(text).map(|one| Ok(one?.as_str().to_owned())).collect()
  }

  fn tests(&self, path: &str) -> Result<bool, Fault> {
    Ok(self.test.is_match(path)?)
  }
}

pub(crate) fn check(root: &Path, tests: bool) -> Result<ExitCode, Fault> {
  let listed = git(root, &["ls-files"])?;
  let mut text = HashMap::new();
  for name in listed.lines() {
    let source =
      [".ts", ".rs", ".py"].iter().any(|kind| name.ends_with(kind)) && !name.ends_with(".d.ts");
    if source && let Ok(bytes) = fs::read(root.join(name)) {
      text.insert(name.to_owned(), String::from_utf8_lossy(&bytes).into_owned());
    }
  }
  let readme = fs::read_to_string(root.join("bind/typescript/README.md")).unwrap_or_default();
  let (found, alone) = unused(&text, &readme)?;
  if tests {
    let patterns = Patterns::new()?;
    for (path, name) in alone.into_iter().collect::<BTreeSet<_>>() {
      let named = Regex::new(&format!(r"\b{}\b", escape(&name)))?;
      let mut files = Vec::new();
      for (file, body) in &text {
        if patterns.tests(file)? && named.is_match(body)? {
          files.push(file.as_str());
        }
      }
      files.sort_unstable();
      if !files.is_empty() {
        println!("{name} {path} {}", files.join(" "));
      }
    }
    return Ok(ExitCode::SUCCESS);
  }
  println!("{}", found.join("\n"));
  Ok(if found.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

/// Each item that no file but a test names, as `path:line: name`, and each item that its own file
/// names and no other file that is no test, as `(path, name)`.
pub(crate) fn unused(text: &HashMap<String, String>, readme: &str) -> Result<Found, Fault> {
  let patterns = Patterns::new()?;
  let world = sources(&patterns, text, readme)?;
  let (mut found, mut alone) = (Vec::new(), Vec::new());
  let mut paths: Vec<&String> = text.keys().collect();
  paths.sort_unstable();
  for path in paths {
    let generated = path == &format!("{PACKAGE}protocol.ts");
    let scanned = SOURCES.iter().any(|source| path.starts_with(source)) && !written(path, "py");
    if scanned && !generated && !patterns.tests(path)? {
      let (more, solo) = scan(&patterns, path, &text[path], &world)?;
      found.extend(more);
      alone.extend(solo);
    }
  }
  Ok((found, alone))
}

fn sources(
  patterns: &Patterns,
  text: &HashMap<String, String>,
  readme: &str,
) -> Result<World, Fault> {
  let mut world = World {
    imported: imports(patterns, text)?,
    documented: patterns.words(readme)?.into_iter().collect(),
    words: HashMap::new(),
    calls: HashMap::new(),
    ts: HashSet::new(),
    py: HashSet::new(),
  };
  let owned = Regex::new(r"\b([A-Z]\w*)::(\w+)")?;
  let called = Regex::new(r"\.(\w+)(?:::<[^(]*>)?\(")?;
  for (path, body) in text {
    if patterns.tests(path)? {
      continue;
    }
    if written(path, "rs") {
      let code: Vec<&str> =
        body.split('\n').filter(|line| !line.trim_start().starts_with("pub use ")).collect();
      let code = code.join("\n");
      for word in patterns.words(&code)? {
        world.words.entry(word).or_default().insert(path.clone());
      }
      for one in owned.captures_iter(code.as_str()) {
        let one = one?;
        world.calls.entry(one[2].to_owned()).or_default().insert((path.clone(), one[1].to_owned()));
      }
      for one in called.captures_iter(code.as_str()) {
        world.calls.entry(one?[1].to_owned()).or_default().insert((path.clone(), String::new()));
      }
    } else if written(path, "py") {
      world.py.extend(patterns.words(body)?);
    } else if written(path, "ts") {
      world.ts.extend(patterns.words(body)?);
    }
  }
  Ok(world)
}

/// Each `(file, name)` that a file of TypeScript that is no test imports, through a barrel or not.
fn imports(
  patterns: &Patterns,
  text: &HashMap<String, String>,
) -> Result<HashSet<(String, String)>, Fault> {
  let mut used = HashSet::new();
  let mut stars: HashMap<String, Vec<String>> = HashMap::new();
  let mut renamed: HashMap<String, HashSet<(String, String)>> = HashMap::new();
  for (path, body) in text {
    if !written(path, "ts") || patterns.tests(path)? {
      continue;
    }
    for one in patterns.inline.captures_iter(body.as_str()) {
      let one = one?;
      if let Some(there) = resolve(text, path, &one[1]) {
        used.insert((there, one[2].to_owned()));
      }
    }
    for one in patterns.from.captures_iter(body.as_str()) {
      let one: Captures<'_, str> = one?;
      let Some(there) = resolve(text, path, &one[4]) else { continue };
      if let Some(space) = one.get(2) {
        let member = Regex::new(&format!(r"\b{}\.(\w+)", escape(space.as_str())))?;
        for name in member.captures_iter(body.as_str()) {
          used.insert((there.clone(), name?[1].to_owned()));
        }
      } else if let Some(names) = one.get(3) {
        let into =
          if &one[1] == "export" { renamed.entry(path.clone()).or_default() } else { &mut used };
        into.extend(listed(names.as_str()).into_iter().map(|name| (there.clone(), name)));
      } else if &one[1] == "export" {
        stars.entry(path.clone()).or_default().push(there);
      }
    }
  }
  loop {
    let mut more = HashSet::new();
    for (there, name) in &used {
      more.extend(
        stars.get(there).into_iter().flatten().map(|source| (source.clone(), name.clone())),
      );
      for (source, was) in renamed.get(there).into_iter().flatten() {
        if was == name {
          more.insert((source.clone(), name.clone()));
        }
      }
    }
    if more.is_subset(&used) {
      return Ok(used);
    }
    used.extend(more);
  }
}

/// The file that an import names: a barrel of the package, or a relative path to a file or to the
/// index of a folder.
fn resolve(text: &HashMap<String, String>, here: &str, spec: &str) -> Option<String> {
  let barrel = match spec {
    "furb" => Some("index.ts"),
    "furb/life" => Some("life/index.ts"),
    "furb/protocol" => Some("protocol.ts"),
    "furb/skill" => Some("skill.ts"),
    _ => None,
  };
  if let Some(barrel) = barrel {
    return Some(format!("{PACKAGE}{barrel}"));
  }
  if !spec.starts_with('.') {
    return None;
  }
  let spec = spec.strip_suffix(".js").or_else(|| spec.strip_suffix(".ts")).unwrap_or(spec);
  // The paths of git name each file with `/` on every system, so the import resolves on them, and
  // not on the paths of the machine, which Windows writes with `\`.
  let mut base: Vec<&str> =
    here.rsplit_once('/').map_or_else(Vec::new, |(folder, _)| folder.split('/').collect());
  for part in spec.split('/') {
    match part {
      "." | "" => {}
      ".." => drop(base.pop()),
      one => base.push(one),
    }
  }
  let base = base.join("/");
  [format!("{base}.ts"), format!("{base}/index.ts")].into_iter().find(|one| text.contains_key(one))
}

fn listed(names: &str) -> Vec<String> {
  names
    .split(',')
    .map(str::trim)
    .filter(|one| !one.is_empty())
    .map(|one| {
      let one = one
        .strip_prefix("type")
        .filter(|rest| rest.starts_with(char::is_whitespace))
        .unwrap_or(one);
      one.trim().split(" as ").next().unwrap_or_default().trim().to_owned()
    })
    .collect()
}

fn scan(patterns: &Patterns, path: &str, body: &str, world: &World) -> Result<Found, Fault> {
  let lines: Vec<&str> = body.split('\n').collect();
  let items = items_of(patterns, path, &lines, world)?;
  let code = Regex::new(r"(?s)/\*.*?\*/")?.replace_all(body, "");
  let code = Regex::new(r"//[^\n]*")?.replace_all(&code, "").into_owned();
  let mut counts = HashMap::new();
  for item in &items {
    let named = Regex::new(&format!(r"\b{}\b", escape(&item.name)))?;
    counts.insert(item.name.as_str(), named.find_iter(&code).count());
  }
  let at: HashMap<&str, &Item> = items.iter().map(|item| (item.name.as_str(), item)).collect();
  let mut used: HashSet<&str> = items
    .iter()
    .filter(|item| item.seen || counts[item.name.as_str()] > 1)
    .map(|item| item.name.as_str())
    .collect();
  let mut queue: Vec<&str> = used.iter().copied().collect();
  while let Some(name) = queue.pop() {
    let item = at[name];
    for other in declared(patterns, &lines, item.line, WHOLE.contains(&item.kind.as_str()))? {
      if let Some((&other, _)) = at.get_key_value(other.as_str())
        && used.insert(other)
      {
        queue.push(other);
      }
    }
  }
  let found = items
    .iter()
    .filter(|item| !used.contains(item.name.as_str()))
    .map(|item| format!("{path}:{}: {}", item.line + 1, item.name))
    .collect();
  let alone = items
    .iter()
    .filter(|item| !item.seen && counts[item.name.as_str()] > 1)
    .map(|item| (path.to_owned(), item.name.clone()))
    .collect();
  Ok((found, alone))
}

fn items_of(
  patterns: &Patterns,
  path: &str,
  lines: &[&str],
  world: &World,
) -> Result<Vec<Item>, Fault> {
  let mut items: Vec<Item> = Vec::new();
  let mut owner = String::new();
  for (n, line) in lines.iter().enumerate() {
    let made = if written(path, "ts") {
      patterns.ts.captures(*line)?.map(|one| {
        let seen = world.imported.contains(&(path.to_owned(), one[2].to_owned()))
          || world.documented.contains(&one[2]);
        Item { name: one[2].to_owned(), line: n, kind: one[1].to_owned(), seen }
      })
    } else {
      if line.starts_with('}') {
        owner.clear();
      }
      if let Some(one) = patterns.implemented.captures(*line)? {
        one[1].clone_into(&mut owner);
      }
      match patterns.rs.captures(*line)? {
        Some(one) => {
          let near = lines[n.saturating_sub(12)..n].join("\n");
          let door = patterns.door.is_match(&near)?;
          let seen = (door && world.hosted(&one[2], &one[3])) || world.named(path, &one, &owner);
          Some(Item { name: one[3].to_owned(), line: n, kind: one[2].to_owned(), seen })
        }
        None => None,
      }
    };
    if let Some(item) = made {
      match items.iter_mut().find(|one| one.name == item.name) {
        Some(held) => *held = item,
        None => items.push(item),
      }
    }
  }
  Ok(items)
}

impl World {
  /// Whether a source of the language of the host names an item of Rust of the door.
  fn hosted(&self, kind: &str, name: &str) -> bool {
    self.ts.contains(&if kind == "fn" { camel(name) } else { name.to_owned() })
      || self.py.contains(name)
  }

  /// Whether a file that is no test, and is not the file of the item, names a public item of Rust,
  /// an associated item through its type or its method call.
  fn named(&self, path: &str, one: &Captures<'_, str>, owner: &str) -> bool {
    let (indent, name) = (&one[1], &one[3]);
    if !indent.is_empty() && !owner.is_empty() {
      let calls = self.calls.get(name).into_iter().flatten();
      return calls.into_iter().any(|(file, by)| file != path && (by.is_empty() || by == owner));
    }
    self.words.get(name).is_some_and(|files| files.iter().any(|file| file != path))
  }
}

/// The words of the declaration of an item: its lines up to the next item of its indent, or up to
/// the end of its parameters when it is no whole type.
fn declared(
  patterns: &Patterns,
  lines: &[&str],
  at: usize,
  whole: bool,
) -> Result<Vec<String>, Fault> {
  let indent = |line: &str| line.len() - line.trim_start().len();
  let (mut out, mut depth) = (Vec::new(), 0_i64);
  for (j, line) in lines.iter().enumerate().skip(at) {
    let ends = !line.trim().is_empty() && indent(line) <= indent(lines[at]);
    let goes_on = ["}", ")", "]", ">", "|", ".", "&", "+", "?", ":", "="]
      .iter()
      .any(|one| line.trim_start().starts_with(one));
    if j > at && ends && !goes_on {
      break;
    }
    out.push(*line);
    depth += i64::try_from(line.matches('(').count()).unwrap_or(0)
      - i64::try_from(line.matches(')').count()).unwrap_or(0);
    if !whole && depth <= 0 {
      break;
    }
  }
  patterns.words(&out.join("\n"))
}

fn camel(name: &str) -> String {
  let mut parts = name.split('_');
  let head = parts.next().unwrap_or_default().to_owned();
  parts.fold(head, |mut made, part| {
    let mut chars = part.chars();
    if let Some(first) = chars.next() {
      made.extend(first.to_uppercase());
      made.push_str(&chars.as_str().to_lowercase());
    }
    made
  })
}

#[cfg(test)]
#[path = "unused.test.rs"]
mod test;
