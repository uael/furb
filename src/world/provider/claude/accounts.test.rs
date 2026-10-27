//! The accounts of the claude command line: which one serves a turn, when one is parked, and what an account gives to
//! a process.

use std::{
  fs,
  process::Command,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::{Account, Accounts, CONFIG, Source, Switch, TOKEN};

fn listed(names: &[&str]) -> Vec<Account> {
  let source = Source::TokenEnv("PATH".to_owned());
  names.iter().map(|name| Account { name: (*name).to_owned(), source: source.clone() }).collect()
}

fn at(seconds: u64) -> SystemTime {
  UNIX_EPOCH + Duration::from_secs(seconds)
}

fn envs(command: &Command) -> Vec<(String, Option<String>)> {
  let text = |one: &std::ffi::OsStr| one.to_string_lossy().into_owned();
  command.get_envs().map(|(key, value)| (text(key), value.map(text))).collect()
}

#[test]
fn failover_takes_the_first_account_that_is_not_parked_until_a_limit_ends_at_its_reset() {
  let accounts = Accounts::new(listed(&["a", "b", "c"]), Switch::Failover);
  assert_eq!(accounts.pick(at(100)), Some(0));
  accounts.park(0, Some(at(160)));
  assert_eq!(accounts.pick(at(100)), Some(1));
  accounts.park(1, None);
  assert_eq!(accounts.pick(at(100)), Some(2));
  accounts.park(2, Some(at(130)));
  assert_eq!(accounts.pick(at(100)), None, "every account is parked");
  assert_eq!(accounts.earliest(at(100)), Some(at(130)));
  assert_eq!(accounts.pick(at(130)), Some(2), "a limit ends at its reset");
  assert_eq!(accounts.pick(at(160)), Some(0));
  assert!(!accounts.free(1, at(1_000_000)), "a failed sign in lasts until a new command line");
}

#[test]
fn spread_takes_the_account_that_a_conversation_took_least_recently() {
  let accounts = Accounts::new(listed(&["a", "b"]), Switch::Spread);
  let order: Vec<_> = (0..5).filter_map(|_| accounts.pick(at(0))).collect();
  assert_eq!(order, [0, 1, 0, 1, 0]);
  assert_eq!(accounts.peek(at(0)), Some(1), "a look counts nothing");
  assert_eq!(accounts.peek(at(0)), Some(1));
  accounts.park(1, None);
  assert_eq!(accounts.pick(at(0)), Some(0));
}

#[test]
fn no_account_listed_is_the_login_of_the_environment_and_is_never_moved() {
  let accounts = Accounts::new(Vec::new(), Switch::Failover);
  assert_eq!((accounts.count(), accounts.switchable(), accounts.name(0)), (1, false, ""));
  assert_eq!(accounts.pick(at(0)), Some(0));
  let mut command = Command::new("claude");
  accounts.apply(0, &mut command).unwrap();
  assert!(envs(&command).is_empty(), "the environment stays as it is");
}

#[test]
fn an_account_gives_its_token_or_its_config_dir_and_takes_away_each_other_credential() {
  let folder = std::env::temp_dir().join(format!("furb-accounts-{}", std::process::id()));
  fs::create_dir_all(&folder).unwrap();
  let file = folder.join("token");
  fs::write(&file, "  tok-file \n").unwrap();
  let empty = folder.join("empty");
  fs::write(&empty, "\n").unwrap();
  let account = |source| Account { name: "a".to_owned(), source };
  let accounts = |source| Accounts::new(vec![account(source)], Switch::Failover);

  let mut command = Command::new("claude");
  accounts(Source::TokenFile(file.clone())).apply(0, &mut command).unwrap();
  let got = envs(&command);
  assert!(got.contains(&(TOKEN.to_owned(), Some("tok-file".to_owned()))), "{got:?}");
  assert!(got.contains(&("ANTHROPIC_API_KEY".to_owned(), None)), "{got:?}");

  let mut command = Command::new("claude");
  accounts(Source::TokenEnv("PATH".to_owned())).apply(0, &mut command).unwrap();
  let path = std::env::var("PATH").unwrap();
  assert!(envs(&command).contains(&(TOKEN.to_owned(), Some(path.trim().to_owned()))));

  let mut command = Command::new("claude");
  accounts(Source::ConfigDir(folder.clone())).apply(0, &mut command).unwrap();
  let got = envs(&command);
  assert!(got.contains(&(CONFIG.to_owned(), Some(folder.display().to_string()))), "{got:?}");
  assert!(got.contains(&(TOKEN.to_owned(), None)), "a token of the environment would win: {got:?}");

  for (source, said) in [
    (Source::TokenFile(folder.join("none")), "its token file"),
    (Source::TokenFile(empty), "is empty"),
    (Source::TokenEnv("FURB_NO_SUCH_TOKEN".to_owned()), "its token variable FURB_NO_SUCH_TOKEN"),
  ] {
    let no = accounts(source).apply(0, &mut Command::new("claude")).unwrap_err();
    assert!(no.starts_with("The account a has no token: "), "{no}");
    assert!(no.contains(said), "{no}");
  }
  fs::remove_dir_all(folder).unwrap();
}
