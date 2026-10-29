//! The config of the servers: the scopes and their order, the definitions and why one cannot run, the variables, and
//! the choices of the operator in the local scope.

use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
  time::Duration,
};

use serde_json::{Value, json};

use super::{CALL, Choice, Places, Scope, Server, Transport, choose, expand, servers};

/// A directory of its own for one test, and empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-mcp-config-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
}

/// A file, written with its folder.
fn file(path: &Path, text: &str) {
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(path, text).unwrap();
}

/// The places of a directory of the tests.
fn places(at: &Path) -> Places {
  Places { config: at.join("config"), project: at.join("project") }
}

/// The variables of a test.
fn variables(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
  let held: HashMap<String, String> =
    pairs.iter().map(|(name, value)| ((*name).to_owned(), (*value).to_owned())).collect();
  move |name| held.get(name).cloned()
}

#[test]
fn a_variable_expands_to_its_value_or_its_default_and_one_that_is_not_set_has_no_value() {
  let set = variables(&[("A", "one"), ("EMPTY", "")]);
  assert_eq!(expand("x ${A} y", &set).unwrap(), "x one y");
  assert_eq!(expand("${B:-two}", &set).unwrap(), "two");
  assert_eq!(expand("${A:-two}", &set).unwrap(), "one");
  assert_eq!(
    expand("${EMPTY:-three}", &set).unwrap(),
    "three",
    "an empty variable takes the default"
  );
  assert_eq!(expand("${EMPTY}", &set).unwrap(), "", "an empty variable with no default is empty");
  assert_eq!(
    expand("$A and ${A", &set).unwrap(),
    "$A and ${A",
    "a mark that is not whole stays as it is"
  );
  assert_eq!(expand("${B}", &set).unwrap_err(), "the variable B is not set");
}

#[test]
fn a_later_scope_wins_for_a_name_which_keeps_its_first_place_and_a_server_of_the_project_waits_for_approval()
 {
  let at = place("scopes");
  let places = places(&at);
  file(
    &places.file(Scope::User),
    r#"{"mcpServers": {"a": {"command": "a"}, "b": {"command": "b-user"}}}"#,
  );
  file(
    &places.file(Scope::Project),
    r#"{"mcpServers": {"b": {"command": "b-project"}, "p": {"command": "p"}}}"#,
  );
  file(
    &places.file(Scope::Local),
    r#"{"mcpServers": {"c": {"type": "sse", "url": "http://c"}}, "approved": ["p"]}"#,
  );
  let got = servers(&places, &variables(&[]));
  let seen: Vec<(&str, Scope, bool, String)> = got
    .iter()
    .map(|one| (one.name.as_str(), one.scope, one.approved, one.transport.target()))
    .collect();
  assert_eq!(
    seen,
    [
      ("a", Scope::User, true, "a".to_owned()),
      ("b", Scope::Project, false, "b-project".to_owned()),
      ("p", Scope::Project, true, "p".to_owned()),
      ("c", Scope::Local, true, "http://c".to_owned()),
    ]
  );
  file(&places.file(Scope::Local), r#"{"approveAll": true}"#);
  assert!(
    servers(&places, &variables(&[])).iter().all(|one| one.approved),
    "approveAll approves each server"
  );
}

#[test]
fn a_definition_gives_its_transport_with_its_variables_expanded_its_timeout_and_its_denied_tools() {
  let at = place("definitions");
  let places = places(&at);
  let user = json!({"mcpServers": {
    "local": {
      "command": "${BIN:-node}",
      "args": ["serve", "${PORT}"],
      "env": {"KEY": "${KEY}"},
      "cwd": "tools",
      "timeout": 2.5,
      "deny": ["drop"],
    },
    "remote": {"type": "http", "url": "https://${HOST}/mcp", "headers": {"authorization": "Bearer ${KEY}"}},
  }});
  file(&places.file(Scope::User), &user.to_string());
  let got = servers(&places, &variables(&[("PORT", "8080"), ("KEY", "k"), ("HOST", "h.dev")]));
  let local = Server {
    name: "local".to_owned(),
    scope: Scope::User,
    transport: Transport::Stdio {
      command: "node".to_owned(),
      args: vec!["serve".to_owned(), "8080".to_owned()],
      env: vec![("KEY".to_owned(), "k".to_owned())],
      cwd: Some(at.join("project").join("tools")),
    },
    disabled: false,
    approved: true,
    timeout: Duration::from_millis(2500),
    deny: vec!["drop".to_owned()],
    error: String::new(),
  };
  assert_eq!(got[0], local);
  assert_eq!(
    got[1].transport,
    Transport::Http {
      url: "https://h.dev/mcp".to_owned(),
      headers: vec![("authorization".to_owned(), "Bearer k".to_owned())]
    }
  );
  assert_eq!(
    (got[1].timeout, got[1].transport.kind(), got[0].transport.kind()),
    (CALL, "http", "stdio")
  );
  assert_eq!(got[1].transport.url(), Some("https://h.dev/mcp"));
}

#[test]
fn a_definition_that_cannot_run_and_a_file_that_is_not_of_the_form_each_say_why() {
  let at = place("errors");
  let places = places(&at);
  let user = json!({"mcpServers": {
    "nothing": {},
    "args": {"command": "x", "args": "-v"},
    "nourl": {"type": "http"},
    "badurl": {"type": "sse", "url": "not a url"},
    "slow": {"command": "x", "timeout": -1},
    "deny": {"command": "x", "deny": "all"},
    "map": 3,
  }});
  file(&places.file(Scope::User), &user.to_string());
  file(&places.file(Scope::Project), "{not json");
  let got = servers(&places, &variables(&[]));
  let errors: Vec<(&str, &str)> =
    got.iter().map(|one| (one.name.as_str(), one.error.as_str())).collect();
  let project = places.file(Scope::Project).display().to_string();
  assert_eq!(
    errors[..7],
    [
      ("nothing", "the server names no command"),
      ("args", "args is no list"),
      ("nourl", "the server names no url"),
      ("badurl", "the url not a url is not valid: relative URL without a base"),
      ("slow", "timeout is no number of seconds above 0"),
      ("deny", "deny is no list"),
      ("map", "the server is no map"),
    ]
  );
  assert_eq!(got[7].name, project);
  assert!(
    got[7].error.starts_with(&format!("{project}: key must be a string")),
    "{}",
    got[7].error
  );
}

#[test]
fn a_choice_of_the_operator_writes_its_list_of_the_local_scope_and_keeps_every_other_key() {
  let at = place("choices");
  let places = places(&at);
  file(&places.file(Scope::User), r#"{"mcpServers": {"s": {"command": "s", "disabled": true}}}"#);
  file(&places.file(Scope::Local), r#"{"theme": "kept", "disabled": "broken"}"#);
  let read = || {
    serde_json::from_str::<Value>(&fs::read_to_string(places.file(Scope::Local)).unwrap()).unwrap()
  };
  let off = || servers(&places, &variables(&[]))[0].disabled;
  assert!(off(), "the definition turns it off");
  choose(&places, Choice::Enable, "s").unwrap();
  assert_eq!(read(), json!({"theme": "kept", "disabled": [], "enabled": ["s"]}));
  assert!(!off(), "the local scope turns it on");
  choose(&places, Choice::Disable, "s").unwrap();
  assert_eq!(read(), json!({"theme": "kept", "disabled": ["s"], "enabled": []}));
  assert!(off());
  choose(&places, Choice::Approve, "s").unwrap();
  choose(&places, Choice::Approve, "s").unwrap();
  assert_eq!(read(), json!({"theme": "kept", "disabled": ["s"], "enabled": [], "approved": ["s"]}));
}
