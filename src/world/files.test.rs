//! The paths and the texts of the files, proved where no life reaches the home of the user.

use std::{fs, path::PathBuf};

use super::{crlf, homed, native, read, replace, stored, text};

#[test]
fn a_path_that_is_a_tilde_or_starts_with_one_and_a_slash_resolves_against_the_home_of_the_user() {
  let home = PathBuf::from("/h");
  let at = |path: &str| homed(path, Some(home.clone())).map(Result::unwrap);
  assert_eq!(at("~"), Some(home.clone()));
  assert_eq!(at("~/a.txt"), Some(home.join("a.txt")));
  assert_eq!(at("~/x/../a.txt"), Some(home.join("a.txt")));
  assert_eq!(at("~a.txt"), None, "a name that starts with a tilde is a name");
  assert_eq!(at("a/~/b"), None);
  let windows = cfg!(windows).then(|| home.join("a.txt"));
  assert_eq!(at("~\\a.txt"), windows, "a backslash after a tilde is a slash on Windows alone");
  let none = homed("~/a.txt", None).expect("a path of the home").unwrap_err();
  assert_eq!(none.message(), "There is no home of the user for ~/a.txt.");
}

#[test]
fn a_read_gives_the_whole_text_of_a_file_of_any_size_and_refuses_what_is_no_file() {
  let at = std::env::temp_dir().join(format!("furb-files-large-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  let content = "x".repeat(1_000_000) + "\n";
  fs::write(at.join("large.txt"), &content).unwrap();
  assert_eq!(read(&at.join("large.txt")).unwrap().content, content);
  assert_eq!(read(&at).unwrap_err().message(), format!("Read needs a text file: {}", at.display()));
  fs::remove_dir_all(&at).unwrap();
}

#[test]
fn a_read_of_an_image_keeps_it_in_the_gallery_and_gives_the_markdown_image_that_names_it() {
  let at = std::env::temp_dir().join(format!("furb-files-image-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  fs::write(at.join("a.png"), b"\x89PNG\r\n\x1a\nthe rest").unwrap();
  fs::write(at.join("b.bin"), b"\x89 no image").unwrap();
  let (gallery, name) = (at.join("images"), at.join("a.png").display().to_string());
  let got = text(&at.join("a.png"), name.clone(), Some(&gallery)).unwrap();
  let uri = "furb-image://657177cbeacd727ffb992c7c6911fdd92593fb4f1875fe6ef7f8b4275ca1facb.png";
  assert_eq!((got.path, got.content), (name.clone(), format!("![a.png]({uri})")));
  let kept = fs::read(gallery.join(uri.strip_prefix("furb-image://").unwrap())).unwrap();
  assert_eq!(kept, b"\x89PNG\r\n\x1a\nthe rest");
  let blind = text(&at.join("a.png"), name.clone(), None).unwrap_err();
  let said = format!(
    "{name} is an image, and this life keeps no directory of images to show it to a model."
  );
  assert_eq!(blind.message(), said);
  let binary = text(&at.join("b.bin"), "b.bin".to_owned(), Some(&gallery)).unwrap_err();
  assert_eq!(binary.message(), "Read needs a text file in utf-8: b.bin");
  fs::remove_dir_all(&at).unwrap();
}

#[test]
fn bytes_end_each_line_with_crlf_when_they_hold_a_line_break_and_a_cr_comes_before_each() {
  assert!(crlf(b"a\r\nb\r\n"));
  assert!(crlf(b"a\r\nb"));
  assert!(!crlf(b"a\r\nb\n"));
  assert!(!crlf(b"\na"));
  assert!(!crlf(b"no break"));
  assert!(!crlf(b""));
}

#[test]
fn a_path_of_the_sh_of_git_is_the_path_of_windows_on_windows_alone() {
  assert_eq!(native("/c/Users/a.txt", true), "C:/Users/a.txt");
  assert_eq!(native("/d", true), "D:/");
  assert_eq!(native("/tmp/plan.md", true), "scratch://plan.md");
  assert_eq!(native("/tmp", true), "scratch://");
  assert_eq!(native("/tmpx/a", true), "/tmpx/a");
  assert_eq!(native("/cd/a", true), "/cd/a");
  assert_eq!(native("src/a.txt", true), "src/a.txt");
  assert_eq!(native("C:\\x", true), "C:\\x");
  assert_eq!(native("/c/Users/a.txt", false), "/c/Users/a.txt");
  assert_eq!(native("/tmp/plan.md", false), "/tmp/plan.md");
}

/// A new folder for a test, which the test removes at its end.
fn folder(name: &str) -> PathBuf {
  let at = std::env::temp_dir().join(format!("furb-files-{name}-{}", std::process::id()));
  let _ = fs::remove_dir_all(&at);
  fs::create_dir_all(&at).unwrap();
  at
}

#[test]
fn a_write_replaces_a_file_and_makes_one_that_is_not_there() {
  let at = folder("replace");
  stored(&at.join("a.txt"), b"one\n").unwrap();
  assert_eq!(fs::read(at.join("a.txt")).unwrap(), b"one\n");
  stored(&at.join("a.txt"), b"two\n").unwrap();
  assert_eq!(fs::read(at.join("a.txt")).unwrap(), b"two\n");
  assert_eq!(fs::read_dir(&at).unwrap().count(), 1, "no temporary file stays");
  fs::remove_dir_all(&at).unwrap();
}

#[test]
fn a_write_that_fails_midway_leaves_the_old_file_whole_and_no_temporary_file() {
  use std::io::Write;
  let at = folder("midway");
  fs::write(at.join("a.txt"), "old\n").unwrap();
  let torn = replace(&at.join("a.txt"), b"new content\n", |file, bytes| {
    file.write_all(&bytes[..4])?;
    Err(std::io::Error::other("the disk is full"))
  });
  assert_eq!(torn.unwrap_err().to_string(), "the disk is full");
  assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "old\n");
  assert_eq!(fs::read_dir(&at).unwrap().count(), 1, "no temporary file stays");
  fs::remove_dir_all(&at).unwrap();
}

#[cfg(unix)]
#[test]
fn a_reader_that_holds_a_file_open_during_a_write_reads_the_old_bytes_whole() {
  use std::io::Read;
  let at = folder("reader");
  fs::write(at.join("a.txt"), "old text\n").unwrap();
  let mut held = fs::File::open(at.join("a.txt")).unwrap();
  stored(&at.join("a.txt"), b"new\n").unwrap();
  let mut seen = String::new();
  held.read_to_string(&mut seen).unwrap();
  assert_eq!(seen, "old text\n", "the write never touched the bytes that the reader holds");
  assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "new\n");
  fs::remove_dir_all(&at).unwrap();
}

#[cfg(unix)]
#[test]
fn a_write_keeps_the_mode_of_the_old_file() {
  use std::os::unix::fs::PermissionsExt;
  let at = folder("mode");
  fs::write(at.join("run.sh"), "old\n").unwrap();
  fs::set_permissions(at.join("run.sh"), fs::Permissions::from_mode(0o751)).unwrap();
  stored(&at.join("run.sh"), b"new\n").unwrap();
  assert_eq!(fs::metadata(at.join("run.sh")).unwrap().permissions().mode() & 0o7777, 0o751);
  fs::remove_dir_all(&at).unwrap();
}

#[cfg(unix)]
#[test]
fn a_write_to_a_link_keeps_the_link_and_changes_its_target() {
  use std::os::unix::fs::symlink;
  let at = folder("link");
  fs::create_dir_all(at.join("deep")).unwrap();
  fs::write(at.join("deep/target.txt"), "old\n").unwrap();
  symlink("deep/target.txt", at.join("mid")).unwrap();
  symlink("mid", at.join("link")).unwrap();
  stored(&at.join("link"), b"new\n").unwrap();
  assert!(fs::symlink_metadata(at.join("link")).unwrap().file_type().is_symlink());
  assert!(fs::symlink_metadata(at.join("mid")).unwrap().file_type().is_symlink());
  assert_eq!(fs::read_to_string(at.join("deep/target.txt")).unwrap(), "new\n");
  fs::create_dir_all(at.join("gone")).unwrap();
  symlink("gone/made.txt", at.join("dangling")).unwrap();
  stored(&at.join("dangling"), b"made\n").unwrap();
  assert!(fs::symlink_metadata(at.join("dangling")).unwrap().file_type().is_symlink());
  assert_eq!(fs::read_to_string(at.join("gone/made.txt")).unwrap(), "made\n");
  fs::remove_dir_all(&at).unwrap();
}

#[cfg(unix)]
#[test]
fn a_write_to_a_file_that_the_user_cannot_write_is_refused_as_a_plain_write_refuses_it() {
  use std::os::unix::fs::PermissionsExt;
  let at = folder("readonly");
  fs::write(at.join("a.txt"), "old\n").unwrap();
  fs::set_permissions(at.join("a.txt"), fs::Permissions::from_mode(0o444)).unwrap();
  let root = fs::OpenOptions::new().write(true).open(at.join("a.txt")).is_ok();
  if !root {
    assert!(stored(&at.join("a.txt"), b"new\n").is_err());
    assert_eq!(fs::read_to_string(at.join("a.txt")).unwrap(), "old\n");
  }
  fs::remove_dir_all(&at).unwrap();
}
