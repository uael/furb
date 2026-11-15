use std::{env, fs, process};

use super::file;

#[test]
fn a_paste_of_the_path_of_an_image_file_gives_its_bytes_in_each_form_that_a_terminal_pastes() {
  let folder = env::temp_dir().join(format!("furb-picture-{}", process::id()));
  fs::create_dir_all(&folder).unwrap();
  let png = b"\x89PNG\r\n\x1a\nrest";
  fs::write(folder.join("a shot.png"), png).unwrap();
  fs::write(folder.join("notes.txt"), "no image").unwrap();
  let full = folder.join("a shot.png").display().to_string();
  let forms = [
    "a shot.png".to_owned(),
    format!("'{full}'"),
    format!("{}\n", full.replace(' ', "\\ ")),
    format!("file://{full}"),
  ];
  let got: Vec<_> = forms.iter().map(|one| file(one, &folder)).collect();
  let others = ["notes.txt", "a shot.png\nnotes.txt", "missing.png"].map(|one| file(one, &folder));
  fs::remove_dir_all(&folder).unwrap();
  assert_eq!(got, [Some(png.to_vec()), Some(png.to_vec()), Some(png.to_vec()), Some(png.to_vec())]);
  assert_eq!(others, [None, None, None]);
}
