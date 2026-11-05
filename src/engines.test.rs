use super::{archive, named, this};

#[test]
fn each_engine_of_the_archive_is_named_by_its_digest() {
  assert!(!archive().is_empty(), "the archive keeps the engines of the major");
  for one in archive() {
    assert!(std::ptr::eq(named(one.digest).unwrap(), one) || named(one.digest).unwrap().this());
  }
  assert!(named(this().digest).unwrap().this(), "this build names its own engine");
  assert!(named("0").is_none(), "no engine has the digest 0");
}
