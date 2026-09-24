use std::{
  collections::{HashMap, HashSet},
  ffi::OsString,
  fs,
  path::PathBuf,
  process::Command,
};

use super::*;

/// A directory of one test, emptied first.
fn yard(test: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-extension-{test}"));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).expect("a yard of the test");
  at
}

/// The places of a test, under its directory.
fn places(at: &Path) -> Places {
  Places { config: at.join("config"), cache: at.join("cache"), home: Some(at.join("home")) }
}

/// A file written, with the directories it stands in.
fn wrote(path: &Path, text: &str) {
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(path, text).unwrap();
}

/// An extension in a directory: its manifest and its python part.
fn extension(root: &Path, name: &str, extra: &str, word: &str) {
  wrote(
    &root.join("package.json"),
    &format!(
      r#"{{"name": "{name}", "version": "0.1.0", "furb": {{"name": "{name}", "python": "{name}.py"{extra}}}}}"#
    ),
  );
  wrote(&root.join(format!("{name}.py")), word);
}

/// An environment of the variables it is given.
fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
  let held: HashMap<String, OsString> =
    vars.iter().map(|(k, v)| ((*k).to_owned(), OsString::from(v))).collect();
  move |key| held.get(key).cloned()
}

/// A git command in a directory, which must succeed.
fn git(at: &Path, args: &[&str]) {
  let done = Command::new("git")
    .args([
      "-c",
      "init.defaultBranch=main",
      "-c",
      "user.name=furb",
      "-c",
      "user.email=furb@example.com",
    ])
    .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
    .args(args)
    .current_dir(at)
    .output()
    .expect("git");
  assert!(done.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&done.stderr));
}

/// A git remote of a test, as a file url, whose main branch holds an extension and whose tag v2 holds another
/// word, with a second extension in the folder sub.
fn remote(at: &Path) -> String {
  let work = at.join("work");
  extension(&work, "demo", "", "x = 1\n");
  extension(&work.join("sub"), "deep", "", "y = 2\n");
  git(&work, &["init"]);
  git(&work, &["add", "."]);
  git(&work, &["commit", "-m", "one"]);
  wrote(&work.join("demo.py"), "x = 2\n");
  git(&work, &["commit", "-am", "two"]);
  git(&work, &["tag", "v2"]);
  git(&work, &["reset", "--hard", "HEAD~1"]);
  git(at, &["clone", "--bare", "work", "remote.git"]);
  let path = at.join("remote.git").display().to_string().replace('\\', "/");
  if path.starts_with('/') { format!("file://{path}") } else { format!("file:///{path}") }
}

fn worded(source: &str) -> String {
  word(source).unwrap()
}

/// The path that the url of a git remote or the package of npm names, with nothing else set, which a test compares
/// as a path, since each machine spells a path with its own separator.
fn spelled(setting: &Setting) -> &Path {
  match setting {
    Setting::From(
      Source::Git { url: one, reference: None, path: None }
      | Source::Npm { package: one, version: None },
    ) => Path::new(one),
    other => panic!("no url or package of a path: {other:?}"),
  }
}

#[test]
fn a_word_without_an_import_of_furb_is_itself() {
  let plain = "import re\nfrom dataclasses import dataclass\n\nx = 1\n";
  assert_eq!(worded(plain), plain);
  assert_eq!(worded("x = 1"), "x = 1");
}

#[test]
fn a_top_level_import_from_furb_leaves_no_line_of_its_own() {
  let module = "import re\nfrom furb.engine import ask\nfrom furb.builtin.files import Text\n\
    from furb.extensions.x import y\nfrom furb import engine\nx = ask\n";
  assert_eq!(worded(module), "import re\nx = ask\n");
}

#[test]
fn a_parenthesized_or_continued_import_leaves_no_line_of_its_own() {
  let cut = "import re\nx = ask\n";
  assert_eq!(worded("import re\nfrom furb.engine import (\n  ask,\n  tell,\n)\nx = ask\n"), cut);
  assert_eq!(worded("import re\nfrom furb.engine import ask, \\\n  tell\nx = ask\n"), cut);
  assert_eq!(worded("import re\nfrom furb.engine import ask  # the bus\nx = ask\n"), cut);
}

#[test]
fn the_empty_lines_where_an_import_stood_are_the_most_of_those_on_either_side() {
  let module = "import re\nfrom dataclasses import dataclass\n\nfrom furb.engine import ask\n\
    from furb.builtin.files import read\n\n\ndef f():\n  return ask\n";
  assert_eq!(
    worded(module),
    "import re\nfrom dataclasses import dataclass\n\n\ndef f():\n  return ask\n"
  );
  assert_eq!(
    worded("import re\n\nfrom furb.engine import ask\n\nimport os\n"),
    "import re\n\nimport os\n"
  );
}

#[test]
fn a_word_starts_and_ends_with_its_code() {
  assert_eq!(
    worded("from furb.engine import ask\n\n\ndef f():\n  return ask\n"),
    "def f():\n  return ask\n"
  );
  assert_eq!(worded("x = 1\n\n\nfrom furb.engine import ask\n"), "x = 1\n");
}

#[test]
fn an_import_of_another_package_a_relative_import_and_import_furb_stay() {
  let module = "from furbish import x\nfrom .engine import y\nimport furb\nimport furb.engine\n";
  assert_eq!(worded(module), module);
}

#[test]
fn an_import_of_furb_inside_a_function_or_a_block_stays() {
  let module =
    "def f():\n  from furb.engine import ask\n  return ask\nif True:\n  from furb import engine\n";
  assert_eq!(worded(module), module);
}

#[test]
fn an_import_that_shares_its_line_takes_its_semicolon_with_it() {
  assert_eq!(worded("from furb.engine import ask; x = 1\n"), "x = 1\n");
  assert_eq!(worded("x = 1; from furb.engine import ask\n"), "x = 1\n");
}

#[test]
fn a_line_end_is_lf_on_every_machine() {
  assert_eq!(worded("x = 1\r\ny = 2\r\n"), "x = 1\ny = 2\n");
  assert_eq!(worded("from furb.engine import ask\r\nx = ask\r\n"), "x = ask\n");
}

#[test]
fn every_other_byte_stays_as_it_is() {
  let module = "x = '''a  \nb\t'''   \nfrom furb.engine import ask\ny = 2  \n";
  assert_eq!(worded(module), "x = '''a  \nb\t'''   \ny = 2  \n");
  let plain = "x = 1   \n\n\ny = 2";
  assert_eq!(worded(plain), plain);
}

#[test]
fn a_module_that_does_not_parse_is_refused_with_its_line() {
  let refused = word("from furb.engine import ask\nx = 1\ny = (\n").unwrap_err();
  assert!(
    matches!(&refused, Error::Word { name, line: 3 | 4, .. } if name.is_empty()),
    "{refused:?}"
  );
  assert!(Fault::from(refused).name == "Refused");
}

#[test]
fn the_config_directory_is_furb_config_dir_when_it_is_set() {
  let got = Places::of(
    env(&[("FURB_CONFIG_DIR", "/c"), ("XDG_CONFIG_HOME", "/x")]),
    Some("/h".into()),
    false,
  );
  assert_eq!(got.config, PathBuf::from("/c"));
}

#[test]
fn the_config_directory_is_under_xdg_config_home_and_then_under_the_home() {
  assert_eq!(
    Places::of(env(&[("XDG_CONFIG_HOME", "/x")]), Some("/h".into()), false).config,
    PathBuf::from("/x/furb")
  );
  assert_eq!(
    Places::of(env(&[]), Some("/h".into()), false).config,
    PathBuf::from("/h/.config/furb")
  );
}

#[test]
fn the_config_directory_on_windows_is_under_appdata_when_xdg_is_unset() {
  assert_eq!(
    Places::of(env(&[("APPDATA", "/a")]), Some("/h".into()), true).config,
    PathBuf::from("/a/furb")
  );
  assert_eq!(
    Places::of(env(&[("APPDATA", "/a")]), Some("/h".into()), false).config,
    PathBuf::from("/h/.config/furb")
  );
}

#[test]
fn the_cache_directory_is_furb_cache_dir_then_xdg_cache_home_then_the_home() {
  let all = env(&[("FURB_CACHE_DIR", "/c"), ("XDG_CACHE_HOME", "/x")]);
  assert_eq!(Places::of(all, Some("/h".into()), false).cache, PathBuf::from("/c"));
  assert_eq!(
    Places::of(env(&[("XDG_CACHE_HOME", "/x")]), Some("/h".into()), false).cache,
    PathBuf::from("/x/furb")
  );
  assert_eq!(Places::of(env(&[]), Some("/h".into()), false).cache, PathBuf::from("/h/.cache/furb"));
}

#[test]
fn the_cache_directory_on_windows_is_under_localappdata() {
  assert_eq!(
    Places::of(env(&[("LOCALAPPDATA", "/l")]), Some("/h".into()), true).cache,
    PathBuf::from("/l/furb")
  );
}

#[test]
fn an_empty_variable_counts_as_unset() {
  let got =
    Places::of(env(&[("FURB_CONFIG_DIR", ""), ("XDG_CONFIG_HOME", "")]), Some("/h".into()), false);
  assert_eq!(got.config, PathBuf::from("/h/.config/furb"));
  assert_eq!(got.home, Some(PathBuf::from("/h")));
}

#[test]
fn a_missing_config_file_holds_no_setting() {
  let at = yard("missing-config");
  assert_eq!(read_settings(&at.join("config.json"), None), Ok(vec![]));
  assert_eq!(settings("{}", &at.join("config.json"), None), Ok(vec![]));
}

#[test]
fn each_form_of_an_entry_is_read() {
  let file = PathBuf::from("/c/config.json");
  let text = r#"{"extensions": {"a": false, "b": true, "c": "p", "d": {"path": "q"},
    "e": {"git": "https://x/y.git", "ref": "v1", "path": "sub"}, "f": {"npm": "@s/n", "version": "1.0.0"}}}"#;
  assert_eq!(
    settings(text, &file, None),
    Ok(vec![
      ("a".to_owned(), Setting::Off),
      ("b".to_owned(), Setting::On),
      ("c".to_owned(), Setting::From(Source::Path("/c/p".into()))),
      ("d".to_owned(), Setting::From(Source::Path("/c/q".into()))),
      (
        "e".to_owned(),
        Setting::From(Source::Git {
          url: "https://x/y.git".to_owned(),
          reference: Some("v1".to_owned()),
          path: Some("sub".into())
        })
      ),
      (
        "f".to_owned(),
        Setting::From(Source::Npm {
          package: "@s/n".to_owned(),
          version: Some("1.0.0".to_owned())
        })
      ),
    ])
  );
}

#[test]
fn an_entry_of_no_known_form_is_refused_with_its_file_and_its_name() {
  let file = PathBuf::from("/c/config.json");
  for form in ["3", r#"{"url": "x"}"#, r#"{"git": 1}"#] {
    let got =
      settings(&format!(r#"{{"extensions": {{"odd": {form}}}}}"#), &file, None).unwrap_err();
    assert!(
      matches!(&got, Error::Entry { name, file: at, .. } if name == "odd" && at == &file),
      "{got:?}"
    );
    assert!(got.to_string().starts_with("the extension odd in the config /c/config.json: "));
  }
}

#[test]
fn a_config_that_is_no_json_is_refused_with_its_file() {
  let file = PathBuf::from("/c/config.json");
  assert!(matches!(settings("{", &file, None), Err(Error::Config { file: at, .. }) if at == file));
  assert!(matches!(settings(r#"{"extensions": []}"#, &file, None), Err(Error::Config { .. })));
}

#[test]
fn a_path_resolves_against_the_directory_of_its_config_file() {
  let file = PathBuf::from("/c/d/config.json");
  let text = r#"{"extensions": {"a": "../p", "b": {"git": "./r.git"}, "c": {"npm": "../n.tgz"}, "d": {"npm": "name"}}}"#;
  let got = settings(text, &file, None).unwrap();
  assert_eq!(got[0].1, Setting::From(Source::Path("/c/p".into())));
  assert_eq!(spelled(&got[1].1), Path::new("/c/d/r.git"));
  assert_eq!(spelled(&got[2].1), Path::new("/c/n.tgz"));
  assert_eq!(got[3].1, Setting::From(Source::Npm { package: "name".to_owned(), version: None }));
}

#[test]
fn a_path_reads_as_the_directory_it_is_before_the_disk_is_asked() {
  assert_eq!(tidy(Path::new("/c/d/../p/./q")), PathBuf::from("/c/p/q"));
  assert_eq!(tidy(Path::new("../../p")), PathBuf::from("../../p"));
  assert_eq!(tidy(Path::new("/../p")), PathBuf::from("/p"));
}

#[test]
fn a_tilde_expands_to_the_home() {
  let file = PathBuf::from("/c/config.json");
  let text = r#"{"extensions": {"a": "~", "b": "~/x", "c": {"git": "~/r.git"}, "d": "~user/x"}}"#;
  let got = settings(text, &file, Some(Path::new("/h"))).unwrap();
  assert_eq!(got[0].1, Setting::From(Source::Path("/h".into())));
  assert_eq!(got[1].1, Setting::From(Source::Path("/h/x".into())));
  assert_eq!(spelled(&got[2].1), Path::new("/h/r.git"));
  assert_eq!(got[3].1, Setting::From(Source::Path("/c/~user/x".into())));
}

#[test]
fn the_local_config_overrides_the_home_config_by_name() {
  let at = yard("local-over-home");
  let places = places(&at);
  let (one, two, three) = (at.join("p/one"), at.join("p/two"), at.join("q/two"));
  wrote(
    &places.config.join("config.json"),
    &serde_json::json!({"extensions": {"one": one, "two": two}}).to_string(),
  );
  wrote(
    &at.join("project/.furb/config.json"),
    &serde_json::json!({"extensions": {"one": false, "two": three}}).to_string(),
  );
  let got = config(&places, &at.join("project")).unwrap();
  let picked: Vec<_> =
    got.iter().skip(3).map(|one| (one.name.as_str(), one.on, one.source.clone())).collect();
  assert_eq!(
    picked,
    [("one", false, Some(Source::Path(one))), ("two", true, Some(Source::Path(three)))]
  );
  assert_eq!(got[4].file, Some(at.join("project/.furb/config.json")));
}

#[test]
fn the_builtins_come_first_then_the_home_names_then_the_local_names_in_file_order() {
  let home = (
    PathBuf::from("/h/config.json"),
    vec![("z".to_owned(), Setting::From(Source::Path("/z".into())))],
  );
  let local = (
    PathBuf::from("/p/config.json"),
    vec![
      ("a".to_owned(), Setting::From(Source::Path("/a".into()))),
      ("z".to_owned(), Setting::Off),
    ],
  );
  let got = merged(&[home, local]).unwrap();
  assert_eq!(
    got.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(),
    ["files", "bash", "grant", "z", "a"]
  );
}

#[test]
fn a_builtin_is_on_unless_false() {
  let got =
    merged(&[(PathBuf::from("/h/config.json"), vec![("grant".to_owned(), Setting::Off)])]).unwrap();
  assert_eq!(got.iter().map(|one| one.on).collect::<Vec<_>>(), [true, true, false]);
  assert!(got.iter().all(|one| one.source == Some(Source::Builtin)));
}

#[test]
fn true_keeps_the_source_an_earlier_file_gave() {
  let home = (
    PathBuf::from("/h/config.json"),
    vec![("x".to_owned(), Setting::From(Source::Path("/x".into())))],
  );
  let local = (PathBuf::from("/p/config.json"), vec![("x".to_owned(), Setting::Off)]);
  let again = (PathBuf::from("/q/config.json"), vec![("x".to_owned(), Setting::On)]);
  let got = merged(&[home, local, again]).unwrap();
  assert_eq!((got[3].on, got[3].source.clone()), (true, Some(Source::Path("/x".into()))));
}

#[test]
fn true_for_a_name_that_no_file_sources_and_no_builtin_is_refused() {
  let got = merged(&[(PathBuf::from("/h/config.json"), vec![("ghost".to_owned(), Setting::On)])]);
  assert!(matches!(got, Err(Error::Entry { name, .. }) if name == "ghost"));
}

#[test]
fn a_manifest_is_read_from_the_furb_field_of_package_json() {
  let root = Path::new("/r");
  let text = r#"{"name": "@s/x", "furb": {"name": "x", "python": "x.py", "life": "x()",
    "world": {"ts": "world.ts", "py": "world.py"}, "tui": "tui.ts", "requires": ["files"]}}"#;
  assert_eq!(
    manifest_of(text, root),
    Ok(Manifest {
      name: "x".to_owned(),
      python: Some("/r/x.py".into()),
      life: Some("x()".to_owned()),
      world: Worlds { ts: Some("/r/world.ts".into()), py: Some("/r/world.py".into()) },
      tui: Some("/r/tui.ts".into()),
      requires: vec!["files".to_owned()],
    })
  );
  let tui = manifest_of(r#"{"furb": {"name": "t", "tui": "tui.ts"}}"#, root).unwrap();
  assert_eq!((tui.python, tui.tui), (None, Some("/r/tui.ts".into())));
}

#[test]
fn a_manifest_without_a_furb_field_or_a_part_is_refused() {
  let root = Path::new("/r");
  assert!(matches!(manifest_of(r#"{"name": "x"}"#, root), Err(Error::Manifest { .. })));
  assert!(matches!(manifest_of(r#"{"furb": {"name": "x"}}"#, root), Err(Error::Manifest { .. })));
  assert!(matches!(
    manifest_of(r#"{"furb": {"python": "x.py"}}"#, root),
    Err(Error::Manifest { .. })
  ));
  let at = yard("no-manifest");
  assert!(matches!(manifest(&at), Err(Error::Manifest { .. })));
}

#[test]
fn a_manifest_whose_name_is_not_its_key_is_refused() {
  let at = yard("name-not-key");
  extension(&at, "real", "", "x = 1\n");
  let got = loaded("other", &at).unwrap_err();
  assert!(got.to_string().contains("the name real is not the key other of its config"), "{got}");
}

#[test]
fn a_manifest_path_that_leaves_its_root_is_refused() {
  let root = Path::new("/r");
  for path in ["../x.py", "/abs/x.py", "a/../../x.py"] {
    let text = format!(r#"{{"furb": {{"name": "x", "python": "{path}"}}}}"#);
    assert!(matches!(manifest_of(&text, root), Err(Error::Manifest { .. })), "{path}");
  }
  assert!(manifest_of(r#"{"furb": {"name": "x", "python": "a/../x.py"}}"#, root).is_ok());
}

#[test]
fn the_builtins_are_files_bash_and_grant_and_bash_requires_files() {
  let held = builtins();
  assert_eq!(
    held.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(),
    ["files", "bash", "grant"]
  );
  assert_eq!(
    held.iter().map(|one| one.requires.clone()).collect::<Vec<_>>(),
    [vec![], vec!["files".to_owned()], vec![]]
  );
  assert!(held.iter().all(|one| {
    one.root.is_none()
      && one.word.is_none()
      && one.life.is_none()
      && one.world == Worlds::default()
      && one.tui.is_none()
  }));
  assert!(held.iter().all(|one| one.life.is_none()));
}

/// The names that each top-level statement of a source binds, in order.
fn bound(source: &str) -> Vec<Vec<String>> {
  let parsed = ruff_python_parser::parse_module(source).unwrap();
  parsed
    .syntax()
    .body
    .iter()
    .map(|one| binds(one).into_iter().map(str::to_owned).collect())
    .collect()
}

#[test]
fn each_name_a_builtin_defines_is_bound_at_the_top_of_the_engine_by_statements_of_that_builtin_alone()
 {
  let statements = bound(crate::ENGINE);
  let mut seen = HashSet::new();
  for one in builtins() {
    let names = defined(&one.name);
    assert!(!names.is_empty(), "{}", one.name);
    for name in names {
      assert!(seen.insert(*name), "{name} is defined by two builtins");
      let binding: Vec<&Vec<String>> =
        statements.iter().filter(|all| all.iter().any(|x| x == name)).collect();
      assert!(!binding.is_empty(), "the engine binds no {name}");
      assert!(
        binding.iter().all(|all| all.iter().all(|x| names.contains(&x.as_str()))),
        "{name} is bound beside a name of no builtin {}",
        one.name
      );
    }
  }
  assert!(defined("skills").is_empty());
}

#[test]
fn the_prompt_with_every_builtin_taken_and_no_word_is_the_engine_it_was_given() {
  let taken = ["files", "bash", "grant"];
  assert_eq!(system(crate::ENGINE, &taken, &[]).unwrap(), crate::ENGINE);
  assert_eq!(system("x=1", &taken, &[]).unwrap(), "x=1");
}

#[test]
fn a_builtin_that_is_off_leaves_the_prompt_with_its_definitions_and_nothing_else() {
  let prompt = system(crate::ENGINE, &["files", "bash"], &[]).unwrap();
  assert!(!prompt.contains("def grant(") && !prompt.contains("WINDOW = "));
  assert!(
    prompt.contains("def bash(") && prompt.contains("class Text:") && prompt.contains("def chain(")
  );
  let bare = system(crate::ENGINE, &[], &[]).unwrap();
  for gone in
    ["def read(", "def write(", "def cd(", "def cwd(", "def bash(", "@dataclass", "class Text:"]
  {
    assert!(!bare.contains(gone), "{gone}");
  }
  for gone in
    ["class Exit:", "HEAD, TAIL, HIDDEN", "TIMEOUT = ", "from dataclasses import dataclass"]
  {
    assert!(!bare.contains(gone), "{gone}");
  }
  assert!(bare.contains("def shown(") && bare.contains("def take(") && bare.contains("def boot("));
  let left: HashSet<String> = bound(&bare).into_iter().flatten().collect();
  let whole: HashSet<String> = bound(crate::ENGINE).into_iter().flatten().collect();
  let cut: HashSet<&str> = builtins().iter().flat_map(|one| defined(&one.name)).copied().collect();
  assert_eq!(left, whole.iter().filter(|x| !cut.contains(x.as_str())).cloned().collect());
  assert!(!bare.contains("\n\n\n\n"), "no cut leaves more empty lines than stood there");
}

#[test]
fn the_prompt_cuts_a_statement_that_shares_its_line_and_keeps_the_other() {
  assert_eq!(system("WINDOW=1;x=2\ny=3\n", &["files", "bash"], &[]).unwrap(), "x=2\ny=3\n");
  assert_eq!(system("a=1\ndef grant():\n\tpass\nb=2\n", &[], &[]).unwrap(), "a=1\nb=2\n");
}

#[test]
fn the_words_follow_the_engine_after_an_empty_line_in_their_order() {
  let words = ["a = 1\n".to_owned(), "b = 2\n".to_owned()];
  assert_eq!(system("x=1", &[], &words).unwrap(), "x=1\n\na = 1\n\nb = 2\n");
  assert_eq!(source(&words), format!("{}\na = 1\n\nb = 2\n", crate::ENGINE));
  assert_eq!(source(&[]), crate::ENGINE);
}

#[test]
fn an_engine_that_does_not_parse_is_refused_with_its_line() {
  let refused = system("x = 1\ny = (\n", &[], &[]).unwrap_err();
  assert!(
    matches!(&refused, Error::Word { name, line: 2 | 3, .. } if name == "the engine"),
    "{refused:?}"
  );
}

fn named(name: &str, requires: &[&str]) -> Extension {
  Extension {
    name: name.to_owned(),
    root: None,
    word: Some(format!("{name} = 1")),
    life: None,
    requires: requires.iter().map(|&one| one.to_owned()).collect(),
    world: Worlds::default(),
    tui: None,
  }
}

fn entry(name: &str, on: bool) -> Entry {
  Entry { name: name.to_owned(), on, source: Some(Source::Builtin), file: None }
}

#[test]
fn an_extension_stands_after_what_it_requires_and_otherwise_keeps_its_place() {
  let entries = [entry("a", true), entry("b", true), entry("c", true), entry("d", true)];
  let got =
    ordered(&entries, vec![named("d", &[]), named("a", &["c"]), named("b", &[]), named("c", &[])])
      .unwrap();
  assert_eq!(got.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(), ["b", "c", "a", "d"]);
}

#[test]
fn an_extension_whose_requirement_is_off_is_refused() {
  let entries = [entry("files", false), entry("bash", true)];
  let got = ordered(&entries, vec![named("bash", &["files"])]).unwrap_err();
  assert_eq!(
    got,
    Error::Requires { name: "bash".to_owned(), needs: "files".to_owned(), off: true }
  );
  assert_eq!(
    got.to_string(),
    "the extension bash requires files, which is off: turn bash off too, or turn files on"
  );
}

#[test]
fn an_extension_whose_requirement_is_missing_is_refused() {
  let got = ordered(&[entry("x", true)], vec![named("x", &["ghost"])]).unwrap_err();
  assert_eq!(got, Error::Requires { name: "x".to_owned(), needs: "ghost".to_owned(), off: false });
}

#[test]
fn a_cycle_of_requirements_is_refused() {
  let entries = [entry("a", true), entry("b", true)];
  let got = ordered(&entries, vec![named("a", &["b"]), named("b", &["a"])]).unwrap_err();
  assert_eq!(got, Error::Cycle { names: vec!["a".to_owned(), "b".to_owned()] });
}

#[test]
fn the_life_words_are_the_life_of_each_extension_in_order() {
  let mut one = named("one", &[]);
  one.life = Some("one()".to_owned());
  assert_eq!(lives(&[named("zero", &[]), one]), ["one()"]);
}

#[test]
fn a_path_entry_is_fetched_in_place_and_a_missing_directory_is_refused() {
  let at = yard("fetch-path");
  let places = places(&at);
  assert_eq!(fetched("x", &Source::Path(at.clone()), &places, false), Ok(at.clone()));
  let got = fetched("x", &Source::Path(at.join("ghost")), &places, false);
  assert!(matches!(got, Err(Error::Fetch { name, .. }) if name == "x"));
}

#[test]
fn a_git_entry_is_cloned_once_into_the_cache_under_the_hash_of_its_url_and_ref() {
  let at = yard("fetch-git");
  let url = remote(&at);
  let places = places(&at);
  let source = Source::Git { url: url.clone(), reference: None, path: None };
  let root = fetched("demo", &source, &places, false).unwrap();
  assert_eq!(root, places.cache.join("extensions/git").join(hashed(&format!("{url}#"))));
  assert_eq!(fs::read_to_string(root.join("demo.py")).unwrap(), "x = 1\n");
  fs::write(root.join("mark"), "").unwrap();
  assert_eq!(fetched("demo", &source, &places, false).unwrap(), root);
  assert!(root.join("mark").exists());
}

#[test]
fn a_git_entry_takes_its_ref_and_its_path() {
  let at = yard("fetch-git-ref");
  let url = remote(&at);
  let places = places(&at);
  let tagged = fetched(
    "demo",
    &Source::Git { url: url.clone(), reference: Some("v2".to_owned()), path: None },
    &places,
    false,
  );
  assert_eq!(fs::read_to_string(tagged.unwrap().join("demo.py")).unwrap(), "x = 2\n");
  let deep = fetched(
    "deep",
    &Source::Git { url, reference: None, path: Some("sub".into()) },
    &places,
    false,
  )
  .unwrap();
  assert_eq!(loaded("deep", &deep).unwrap().word, Some("y = 2\n".to_owned()));
}

#[test]
fn a_git_entry_is_cloned_again_on_a_refresh() {
  let at = yard("fetch-git-refresh");
  let url = remote(&at);
  let places = places(&at);
  let source = Source::Git { url, reference: None, path: None };
  let root = fetched("demo", &source, &places, false).unwrap();
  fs::write(root.join("mark"), "").unwrap();
  assert_eq!(fetched("demo", &source, &places, true).unwrap(), root);
  assert!(!root.join("mark").exists() && root.join("demo.py").exists());
}

#[test]
fn a_git_fetch_that_fails_is_refused_with_what_git_said() {
  let at = yard("fetch-git-fails");
  let source = Source::Git {
    url: format!("file://{}/nowhere.git", at.display()),
    reference: None,
    path: None,
  };
  let got = fetched("gone", &source, &places(&at), false).unwrap_err();
  assert!(
    matches!(&got, Error::Fetch { name, why } if name == "gone" && why.contains("git")),
    "{got:?}"
  );
  assert!(!places(&at).cache.join("extensions/git").exists());
}

#[test]
fn an_npm_entry_is_packed_and_unpacked_once_into_the_cache() {
  let at = yard("fetch-npm");
  let folder = at.join("pkg");
  extension(&folder, "demo", "", "x = 1\n");
  let places = places(&at);
  let source = Source::Npm { package: folder.display().to_string(), version: None };
  let root = fetched("demo", &source, &places, false).unwrap();
  assert!(root.starts_with(places.cache.join("extensions/npm")));
  assert_eq!(loaded("demo", &root).unwrap().word, Some("x = 1\n".to_owned()));
  fs::write(root.join("mark"), "").unwrap();
  assert_eq!(fetched("demo", &source, &places, false).unwrap(), root);
  assert!(root.join("mark").exists());
}

#[test]
fn an_npm_entry_of_a_registry_name_is_kept_under_its_name_and_version() {
  let places = places(Path::new("/t"));
  assert_eq!(
    npm_dir(&places, "@furb/skills", Some("0.1.0")),
    PathBuf::from("/t/cache/extensions/npm/@furb/skills@0.1.0")
  );
  assert_eq!(
    npm_dir(&places, "plain", None),
    PathBuf::from("/t/cache/extensions/npm/plain@latest")
  );
  assert_eq!(
    npm_dir(&places, "./x.tgz", None),
    places.cache.join("extensions/npm").join(hashed("./x.tgz@"))
  );
}

#[test]
fn extensions_gives_the_builtins_and_the_path_extensions_of_both_configs_in_order_with_their_words()
{
  let at = yard("extensions");
  let places = places(&at);
  extension(&at.join("one"), "one", r#", "requires": ["two"], "life": "one()""#, "one = 1\n");
  extension(&at.join("two"), "two", "", "from furb.engine import ask\ntwo = ask\n");
  wrote(&places.config.join("config.json"), r#"{"extensions": {"grant": false, "one": "../one"}}"#);
  wrote(&at.join("project/.furb/config.json"), r#"{"extensions": {"two": "../../two"}}"#);
  let got = extensions(&places, &at.join("project"), false, false).unwrap();
  assert_eq!(
    got.iter().map(|one| one.name.as_str()).collect::<Vec<_>>(),
    ["files", "bash", "two", "one"]
  );
  assert_eq!(words(&got), ["two = ask\n".to_owned(), "one = 1\n".to_owned()]);
  assert_eq!(lives(&got), ["one()"]);
}

#[test]
fn extensions_refuses_an_extension_whose_manifest_name_is_not_its_key() {
  let at = yard("extensions-name");
  let places = places(&at);
  extension(&at.join("x"), "real", "", "x = 1\n");
  wrote(&places.config.join("config.json"), r#"{"extensions": {"other": "../x"}}"#);
  assert!(matches!(
    extensions(&places, &at.join("project"), false, false),
    Err(Error::Manifest { .. })
  ));
}

#[test]
fn extensions_refuses_a_python_part_that_does_not_parse_with_its_name_and_its_line() {
  let at = yard("extensions-broken");
  let places = places(&at);
  extension(&at.join("x"), "x", "", "a = 1\nb = (\n");
  wrote(&places.config.join("config.json"), r#"{"extensions": {"x": "../x"}}"#);
  let got = extensions(&places, &at.join("project"), false, false).unwrap_err();
  assert!(matches!(&got, Error::Word { name, .. } if name == "x"), "{got:?}");
}

/// The folder of the skills extension of the repository.
fn skills() -> PathBuf {
  PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/extensions/skills"))
}

#[test]
fn the_skills_extension_of_the_repository_loads_the_same_by_a_path_a_git_remote_and_an_npm_package()
{
  let at = yard("skills");
  let places = places(&at);
  let work = at.join("work");
  for file in ["package.json", "skills.py", "skills.pyi", "world.ts", "tui.ts"] {
    wrote(
      &work.join("extensions/skills").join(file),
      &fs::read_to_string(skills().join(file)).unwrap(),
    );
  }
  git(&work, &["init"]);
  git(&work, &["add", "."]);
  git(&work, &["commit", "-m", "skills"]);
  git(&at, &["clone", "--bare", "work", "remote.git"]);
  let path = at.join("remote.git").display().to_string().replace('\\', "/");
  let url =
    if path.starts_with('/') { format!("file://{path}") } else { format!("file:///{path}") };
  let sources = [
    Source::Path(skills()),
    Source::Git { url, reference: None, path: Some("extensions/skills".into()) },
    Source::Npm { package: skills().display().to_string(), version: None },
  ];
  let got: Vec<Extension> = sources
    .iter()
    .map(|source| loaded("skills", &fetched("skills", source, &places, false).unwrap()).unwrap())
    .collect();
  for one in &got {
    assert_eq!(one.word, got[0].word);
    assert_eq!(one.life.as_deref(), Some("skills()"));
    assert_eq!(one.requires, ["files"]);
    assert!(one.world.ts.as_ref().is_some_and(|ts| ts.ends_with("world.ts") && ts.is_file()));
    assert!(one.tui.as_ref().is_some_and(|tui| tui.ends_with("tui.ts") && tui.is_file()));
  }
  let word = got[0].word.clone().unwrap();
  assert!(word.starts_with("from dataclasses import dataclass\n\n\ndef skills("), "{word}");
  assert!(!word.contains("furb"));
}
