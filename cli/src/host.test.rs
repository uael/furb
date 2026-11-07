use std::{env, fs, process, sync::mpsc, time::Duration};

use furb_host::{
  access::Access,
  protocol::Role,
  transport::{LocalApi, Tailscale},
};
use tokio::time::timeout;

use super::followed;

/// An access list of the logins that write.
fn list(logins: &[&str]) -> Access {
  logins.iter().map(|one| ((*one).to_owned(), Role::Write)).collect()
}

/// The file of the roles of a list.
fn written(logins: &[&str]) -> String {
  let roles: Vec<String> = logins.iter().map(|one| format!("\"{one}\": \"write\"")).collect();
  format!("{{\"roles\": {{{}}}}}", roles.join(", "))
}

#[expect(
  clippy::disallowed_methods,
  reason = "the test blocks on the runtime that tokio starts for it"
)]
#[tokio::test]
async fn a_change_of_host_json_gives_its_logins_and_a_file_that_does_not_parse_keeps_them() {
  let folder = env::temp_dir().join(format!("furb-host-json-{}", process::id()));
  fs::create_dir_all(&folder).unwrap();
  let path = folder.join("host.json");
  fs::write(&path, written(&["ann", "bob"])).unwrap();
  let identity =
    Tailscale::new(LocalApi::Port { port: 1, token: "none".to_owned() }, list(&["ann", "bob"]));
  let (says, heard) = mpsc::channel();
  let _watch = followed(&path, identity.clone(), move |why| drop(says.send(why.to_owned())))
    .expect("the folder can be watched");
  let mut changes = identity.changes();
  fs::write(&path, written(&["ann", "bob", "cat"])).unwrap();
  timeout(Duration::from_secs(20), changes.changed()).await.expect("the list changes").unwrap();
  assert!(identity.access().role("cat").is_ok(), "the door gives the new login its role");
  assert_eq!(identity.logins(), ["ann", "bob", "cat"], "a life opened after it names the login");
  fs::write(&path, "{ not json").unwrap();
  let why = heard.recv_timeout(Duration::from_secs(20)).expect("the daemon says why");
  assert!(why.contains("keeps the access list"), "{why}");
  assert_eq!(identity.logins(), ["ann", "bob", "cat"], "a file that does not parse keeps the list");
  fs::remove_dir_all(&folder).ok();
}
