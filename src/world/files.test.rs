//! The paths and the texts of the files, proved where no life reaches the home of the user.

use std::{fs, path::PathBuf};

use super::{crlf, homed, native, placed, read, text, tilded};
use crate::testing::Yard;

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
fn a_path_under_the_home_of_the_user_is_written_with_a_tilde_in_place_of_the_home() {
  let home = PathBuf::from("/h");
  let at = |path: &str| tilded(&PathBuf::from(path), Some(&home));
  let slash = std::path::MAIN_SEPARATOR;
  assert_eq!(at("/h"), "~");
  assert_eq!(at("/h/a.txt"), format!("~{slash}a.txt"));
  assert_eq!(at("/h/x/a.txt"), format!("~{slash}{}", PathBuf::from("x/a.txt").display()));
  assert_eq!(
    at("/ha/a.txt"),
    "/ha/a.txt",
    "a folder whose name only starts with the home is no folder of it"
  );
  assert_eq!(at("/w/a.txt"), "/w/a.txt");
  assert_eq!(tilded(&PathBuf::from("/h/a.txt"), None), "/h/a.txt");
  assert_eq!(
    tilded(&PathBuf::from("/a.txt"), Some(&PathBuf::from("/"))),
    "/a.txt",
    "a home at the root holds no tilde"
  );
  let back = homed(&at("/h/x/a.txt"), Some(home.clone())).map(Result::unwrap);
  assert_eq!(
    back,
    Some(home.join("x").join("a.txt")),
    "a path that it writes is read back as the same place"
  );
}

#[test]
fn a_path_that_a_chain_names_is_placed_against_the_home_or_else_against_its_directory() {
  let here = PathBuf::from("/w");
  assert_eq!(placed(&here, "a/../b.txt").unwrap(), here.join("b.txt"));
  assert_eq!(placed(&here, "/x/b.txt").unwrap(), PathBuf::from("/x/b.txt"));
  if let Some(home) = std::env::home_dir() {
    assert_eq!(placed(&here, "~/b.txt").unwrap(), home.join("b.txt"));
  }
}

#[test]
fn a_read_gives_the_whole_text_of_a_file_of_any_size_and_the_entries_of_a_folder() {
  let at = Yard::unmade(format!("furb-files-large-{}", std::process::id()));
  fs::create_dir_all(&at).unwrap();
  let content = "x".repeat(1_000_000) + "\n";
  fs::write(at.join("large.txt"), &content).unwrap();
  assert_eq!(read(&at.join("large.txt")).unwrap().content, content);
  let shown = tilded(&at, std::env::home_dir().as_deref());
  fs::create_dir(at.join("sub")).unwrap();
  fs::create_dir(at.join("sub").join("deep")).unwrap();
  fs::write(at.join("B.txt"), "b").unwrap();
  let listing = read(&at).unwrap();
  assert_eq!((listing.path, listing.content), (shown, "B.txt\nlarge.txt\nsub/".to_owned()));
}

#[test]
fn a_read_of_an_image_keeps_it_in_the_gallery_and_gives_the_markdown_image_that_names_it() {
  let at = Yard::unmade(format!("furb-files-image-{}", std::process::id()));
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
