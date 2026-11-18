//! The paths and the texts of the files of the World, through a life that reads and writes with
//! them. A test of the home of the user runs again as a child of itself, whose home is a folder of
//! its yard, as a user starts a host.

use std::{
  fs,
  path::{MAIN_SEPARATOR, Path},
  process,
};

use furb::skill::Places;
use furb::{Ear, Engine, Object, Text, world};

use super::provider::closed;
use crate::{
  child::{child, here, ran},
  yard::Yard,
};

/// An empty folder of the test, named with the id of the process.
fn yard(name: &str) -> Yard {
  Yard::new(format!("furb-files-{name}-{}", process::id()))
}

/// A life on the files of the World alone, whose gallery of images is this one, when it keeps one.
fn life(at: &Path, gallery: Option<&Path>) -> Engine {
  let scratch = world::Scratch::at(at.join("scratch")).expect("the scratch directory");
  let files =
    world::files(scratch, Places::under(at.join("config")), gallery.map(Path::to_path_buf));
  let ears: [(&str, Box<dyn Ear>); 1] = [("files", files)];
  Engine::boot(Vec::<Object>::new(), ears).expect("the life opens")
}

/// A path as python writes it in a word.
fn quoted(path: &Path) -> String {
  serde_json::to_string(&path.display().to_string()).expect("a path as a string")
}

/// The path and the content of the text that a word of the root closes with, or why it raised.
fn text(engine: &mut Engine, word: &str) -> Result<(String, String), String> {
  let got = closed(engine, &format!("close({word})"))?;
  let text = Text::of(got.as_ref()).expect("a text");
  Ok((text.path, text.content))
}

/// The child of this test, whose home is the folder `h` of its yard.
fn homed(test: &str, at: &Path) {
  let home = at.join("h").display().to_string();
  ran(test, at, &[("HOME", &home), ("USERPROFILE", &home)]);
}

#[test]
fn a_tilde_names_the_home_that_the_places_give_and_a_path_under_it_shows_from_that_home() {
  let at = yard("given");
  let home = at.join("h");
  fs::create_dir_all(&home).expect("the home");
  fs::write(home.join("a.txt"), "home").expect("a file of the home");
  let scratch = world::Scratch::at(at.join("scratch")).expect("the scratch directory");
  let places = Places { config: at.join("config"), home: Some(home.clone()) };
  let ears: [(&str, Box<dyn Ear>); 1] = [("files", world::files(scratch, places, None))];
  let mut engine = Engine::boot(Vec::<Object>::new(), ears).expect("the life opens");
  let shown = Ok((format!("~{MAIN_SEPARATOR}a.txt"), "home".to_owned()));
  assert_eq!(text(&mut engine, "read('~/a.txt')"), shown);
  let whole = format!("read({})", quoted(&home.join("a.txt")));
  assert_eq!(text(&mut engine, &whole), shown, "a path under the home shows from it");
}

#[test]
fn a_path_that_is_a_tilde_or_starts_with_one_and_a_slash_names_the_home_and_a_path_under_it_shows_so()
 {
  if let Some(at) = child() {
    let mut engine = life(&at, None);
    let read = |engine: &mut Engine, path: &str| text(engine, &format!("read({path:?})"));
    if at.join("root").exists() {
      let file = at.join("ha").join("a.txt");
      let shown = read(&mut engine, &file.display().to_string()).expect("a file").0;
      assert_eq!(shown, file.display().to_string(), "a home at the root holds no tilde");
      return;
    }
    let slash = MAIN_SEPARATOR;
    assert_eq!(read(&mut engine, "~"), Ok(("~".to_owned(), "a.txt\nx/".to_owned())));
    let home = Ok((format!("~{slash}a.txt"), "home".to_owned()));
    assert_eq!(read(&mut engine, "~/a.txt"), home);
    assert_eq!(read(&mut engine, "~/x/../a.txt"), home, "a path of the home is resolved");
    let under = format!("~{slash}x{slash}b.txt");
    assert_eq!(read(&mut engine, "~/x/b.txt"), Ok((under, "deep".to_owned())));
    closed(&mut engine, &format!("cd({})\nclose(0)", quoted(&at.join("w"))))
      .expect("the chain moves");
    let named =
      text(&mut engine, "write(Text('~a.txt', 'a name'))").expect("a name that starts so");
    assert_eq!(named.0, at.join("w").join("~a.txt").display().to_string());
    let inside = read(&mut engine, "a/~/b").expect_err("no such file");
    assert!(
      inside.contains(&format!(
        "There is no file at {}.",
        at.join("w").join("a").join("~").join("b").display()
      )),
      "{inside}"
    );
    let back = read(&mut engine, "~\\a.txt");
    assert_eq!(
      back.is_ok(),
      cfg!(windows),
      "a backslash after a tilde is a slash on Windows alone: {back:?}"
    );
    let sibling = at.join("ha").join("a.txt");
    let shown = read(&mut engine, &sibling.display().to_string()).expect("a file beside the home");
    assert_eq!(
      shown.0,
      sibling.display().to_string(),
      "a folder whose name only starts with the home is no folder of it"
    );
    return;
  }
  let at = yard("home");
  for (path, content) in [("h/a.txt", "home"), ("h/x/b.txt", "deep"), ("ha/a.txt", "beside")] {
    fs::create_dir_all(at.join(path).parent().expect("a folder")).expect("the folder");
    fs::write(at.join(path), content).expect("the file");
  }
  fs::create_dir_all(at.join("w")).expect("the working directory");
  homed(here!(), &at);
  #[cfg(unix)]
  {
    fs::write(at.join("root"), "").expect("the mark");
    ran(here!(), &at, &[("HOME", "/")]);
  }
}

#[test]
fn a_read_gives_the_whole_text_of_a_file_of_any_size_and_the_entries_of_a_folder() {
  let at = yard("large");
  let content = "x".repeat(1_000_000) + "\n";
  fs::write(at.join("large.txt"), &content).expect("the file");
  fs::create_dir_all(at.join("sub").join("deep")).expect("the folders");
  fs::write(at.join("B.txt"), "b").expect("the file");
  let mut engine = life(&at, None);
  let large = text(&mut engine, &format!("read({}, HIDDEN)", quoted(&at.join("large.txt"))));
  assert_eq!(large.map(|one| one.1 == content), Ok(true));
  let listing = text(&mut engine, &format!("read({})", quoted(&at))).expect("a folder reads");
  assert_eq!(listing.1, "B.txt\nlarge.txt\nsub/");
}

#[test]
fn a_read_of_an_image_keeps_it_in_the_gallery_and_gives_the_markdown_image_that_names_it() {
  // The yard stands under the home of the user on a runner of Windows, so the test runs again as a
  // child whose home is a folder of its yard.
  let Some(at) = child() else {
    let at = yard("image");
    homed(here!(), &at);
    return;
  };
  fs::write(at.join("a.png"), b"\x89PNG\r\n\x1a\nthe rest").expect("the image");
  fs::write(at.join("b.bin"), b"\x89 no image").expect("the bytes");
  let gallery = at.join("images");
  let read =
    |engine: &mut Engine, name: &str| text(engine, &format!("read({})", quoted(&at.join(name))));
  let uri = "furb-image://657177cbeacd727ffb992c7c6911fdd92593fb4f1875fe6ef7f8b4275ca1facb.png";
  let name = at.join("a.png").display().to_string();
  assert_eq!(
    read(&mut life(&at, Some(&gallery)), "a.png"),
    Ok((name.clone(), format!("![a.png]({uri})")))
  );
  let kept = fs::read(gallery.join(uri.strip_prefix("furb-image://").expect("a uri")))
    .expect("the kept image");
  assert_eq!(kept, b"\x89PNG\r\n\x1a\nthe rest");
  let blind = read(&mut life(&at, None), "a.png").expect_err("no gallery");
  let said = format!(
    "{name} is an image, and this life keeps no directory of images to show it to a model."
  );
  assert!(blind.contains(&said), "{blind}");
  let binary = read(&mut life(&at, Some(&gallery)), "b.bin").expect_err("no text");
  let bin = at.join("b.bin").display().to_string();
  assert!(binary.contains(&format!("Read needs a text file in utf-8: {bin}")), "{binary}");
}

#[test]
fn a_file_whose_every_line_ends_with_crlf_reads_with_lf_and_a_write_gives_each_line_its_crlf_back()
{
  let at = yard("crlf");
  let held = [
    ("crlf.txt", "a\r\nb\r\n", "a\nb\n", "c\r\nd\r\n"),
    ("unended.txt", "a\r\nb", "a\nb", "c\r\nd\r\n"),
    ("mixed.txt", "a\r\nb\n", "a\r\nb\n", "c\nd\n"),
    ("first.txt", "\na", "\na", "c\nd\n"),
    ("one.txt", "no break", "no break", "c\nd\n"),
  ];
  let mut engine = life(&at, None);
  for (name, bytes, read, written) in held {
    let path = at.join(name);
    fs::write(&path, bytes).expect("the file");
    let got = text(&mut engine, &format!("read({})", quoted(&path))).expect("a text").1;
    assert_eq!(got, read, "{name}");
    text(&mut engine, &format!("write(Text({}, 'c\\nd\\n'))", quoted(&path))).expect("a write");
    assert_eq!(fs::read_to_string(&path).expect("the file"), written, "{name}");
  }
}

#[test]
fn a_path_of_the_sh_of_git_names_a_drive_and_the_scratch_directory_on_windows_alone() {
  let at = yard("native");
  let mut engine = life(&at, None);
  let name = format!("furb-native-{}.md", process::id());
  let wrote = text(&mut engine, &format!("write(Text('/tmp/{name}', 'x'))"));
  let windows = cfg!(windows);
  let path = wrote.as_ref().map(|one| one.0.as_str());
  if windows {
    assert_eq!(path, Ok(format!("scratch://{name}").as_str()));
    assert_eq!(text(&mut engine, "read('/tmp')").map(|one| one.0), Ok("scratch://".to_owned()));
    let drive = at.display().to_string().replace('\\', "/");
    let (letter, rest) = drive.split_once(":/").expect("a path of a drive");
    fs::write(at.join("a.txt"), "on the drive").expect("the file");
    let read = text(&mut engine, &format!("read('/{}/{rest}/a.txt')", letter.to_lowercase()));
    assert_eq!(read.map(|one| one.1), Ok("on the drive".to_owned()));
    let other = text(&mut engine, "read('/tmpx/a')").expect_err("no file");
    assert!(!other.contains("scratch://"), "{other}");
  } else {
    let real = format!("/tmp/{name}");
    assert_eq!(path, Ok(real.as_str()), "a path of another machine stands as it is");
    fs::remove_file(&real).expect("the file of the test goes");
  }
}

#[test]
fn a_path_resolves_against_its_directory_with_every_dot_read_and_an_absolute_path_stands_as_it_is()
{
  // The yard stands under the home of the user on a runner of Windows, so the test runs again as a
  // child whose home is a folder of its yard.
  let Some(at) = child() else {
    let at = yard("resolved");
    homed(here!(), &at);
    return;
  };
  fs::create_dir(at.join("w")).expect("the folder of the chain");
  let mut engine = life(&at, None);
  closed(&mut engine, &format!("cd({})\nclose(0)", quoted(&at.join("w"))))
    .expect("the chain moves");
  let written = |engine: &mut Engine, path: &str| {
    text(engine, &format!("write(Text({path}, 'x'))")).expect("a write").0
  };
  let shown = |path: &Path| path.display().to_string();
  assert_eq!(written(&mut engine, "'a/../b/./c'"), shown(&at.join("w").join("b").join("c")));
  assert_eq!(written(&mut engine, "'../x'"), shown(&at.join("x")));
  let elsewhere = at.join("elsewhere");
  assert_eq!(written(&mut engine, &quoted(&elsewhere)), shown(&elsewhere));
  assert_eq!(fs::read_to_string(at.join("w").join("b").join("c")).expect("the file"), "x");
}
