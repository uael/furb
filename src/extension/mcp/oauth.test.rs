//! The parts of a login by OAuth that the life of the ear does not show: a challenge, the store of the tokens in a
//! file, the discovery of the endpoints, the refusals of a refresh, and the accounts of the Keychain.

use std::{fs, future::Future, path::PathBuf};

use reqwest::Client;

use super::{
  Endpoints, Filed, Keychain, Tokens, Vault, authority, challenged, protected, refreshed, store,
  stored,
};
use crate::{ear::reactor, extension::mcp::fixture};

/// A directory of its own for one test, and empty.
fn place(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-mcp-oauth-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  at
}

/// What a work on the reactor came to.
fn reacted<T: Send + 'static>(work: impl Future<Output = T> + Send + 'static) -> T {
  futures::executor::block_on(reactor().spawn(work)).unwrap()
}

/// Tokens of the tests, refreshed at an endpoint.
fn tokens(refresh: Option<&str>, endpoint: &str) -> Tokens {
  Tokens {
    access: "a".to_owned(),
    refresh: refresh.map(str::to_owned),
    expires: Some(1),
    client: "client-1".to_owned(),
    secret: None,
    endpoint: endpoint.to_owned(),
    resource: "https://r".to_owned(),
  }
}

#[test]
fn a_challenge_names_the_metadata_of_its_resource_quoted_or_not() {
  let quoted = r#"Bearer error="invalid_token", resource_metadata="https://a/.well-known/x""#;
  assert_eq!(challenged(quoted).as_deref(), Some("https://a/.well-known/x"));
  assert_eq!(
    challenged("Bearer resource_metadata=https://b/m, scope=x").as_deref(),
    Some("https://b/m")
  );
  assert_eq!(challenged("Bearer realm=x"), None);
}

#[test]
fn a_file_of_tokens_keeps_each_by_its_key_and_only_the_user_reads_it() {
  let at = place("file").join("mcp-tokens.json");
  let vault = Filed::new(at.clone());
  assert_eq!(vault.get("a"), None);
  vault.put("a", "one").unwrap();
  vault.put("b", "two").unwrap();
  assert_eq!((vault.get("a").as_deref(), vault.get("b").as_deref()), (Some("one"), Some("two")));
  vault.forget("a").unwrap();
  vault.forget("nothing").unwrap();
  assert_eq!(vault.get("a"), None);
  let held = tokens(Some("r"), "https://t");
  store(&vault, "s", &held).unwrap();
  assert_eq!(stored(&vault, "s"), Some(held));
  #[cfg(unix)]
  assert_eq!(
    std::os::unix::fs::PermissionsExt::mode(&fs::metadata(&at).unwrap().permissions()) & 0o777,
    0o600
  );
}

#[test]
fn the_endpoints_come_from_the_metadata_or_from_the_origin_of_a_server_that_names_none() {
  let program = fixture::served();
  let base = program.address.clone();
  let (asked, origin) = (base.clone(), base.clone());
  let found = reacted(async move {
    let client = Client::new();
    let secure = protected(&client, &format!("{asked}/secure"), None).await.unwrap();
    let plain = protected(&client, &format!("{asked}/plain/"), None).await.unwrap();
    let named = authority(&client, &secure.0).await.unwrap();
    let own = authority(&client, &plain.0).await.unwrap();
    (secure, plain, named, own)
  });
  assert_eq!(found.0, (format!("{base}/auth"), Some("mcp".to_owned())));
  assert_eq!(found.1, (origin, None));
  assert_eq!(
    found.2,
    Endpoints {
      authorize: format!("{base}/auth/authorize"),
      token: format!("{base}/auth/token"),
      register: Some(format!("{base}/auth/register")),
    }
  );
  assert_eq!(
    found.3,
    Endpoints {
      authorize: format!("{base}/authorize"),
      token: format!("{base}/token"),
      register: Some(format!("{base}/register")),
    }
  );
}

#[test]
fn tokens_with_no_refresh_token_are_not_refreshed_and_a_refused_grant_says_why() {
  let program = fixture::served();
  let endpoint = format!("{}/auth/token", program.address);
  let (none, wrong) = (tokens(None, &endpoint), tokens(Some("wrong"), &endpoint));
  let got = reacted(async move { (refreshed(&none).await, refreshed(&wrong).await) });
  assert_eq!(got.0.unwrap_err(), "the login gave no refresh token");
  assert_eq!(
    got.1.unwrap_err(),
    format!("{endpoint} answered 400 Bad Request, invalid_grant: no such grant")
  );
}

#[test]
fn an_account_of_the_keychain_holds_no_sign_that_its_program_reads_as_a_break() {
  assert_eq!(Keychain::account("my server https://x.dev/a b\"c"), "my_server_https://x.dev/a_b_c");
}
