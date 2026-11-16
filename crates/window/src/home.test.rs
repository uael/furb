use super::shown_folder;

#[test]
fn a_folder_shows_from_the_home_of_the_window_whatever_home_its_machine_told_it_from() {
  // Windows tells the folder of a chain from the home of its user, with its separators.
  let (told, here) = (r"C:\Users\me", r"C:\Users\me\AppData\Local\Temp\fw1");
  let tilded = r"~\AppData\Local\Temp\fw1\furb";
  assert_eq!(shown_folder(tilded, Some(told), Some(here)), "~/furb");
  assert_eq!(shown_folder(tilded, Some(told), Some(told)), "~/AppData/Local/Temp/fw1/furb");
  assert_eq!(shown_folder(r"~\src\furb", None, None), "~/src/furb");
  // An absolute folder, and one that only starts with the name of the home.
  assert_eq!(shown_folder("/Users/me/furb", Some("/Users/me"), Some("/Users/me")), "~/furb");
  assert_eq!(shown_folder("/Users/me", None, Some("/Users/me")), "~");
  assert_eq!(shown_folder("/Users/meg/furb", None, Some("/Users/me")), "/Users/meg/furb");
  assert_eq!(shown_folder("/opt/furb", Some("/Users/me"), Some("/Users/me")), "/opt/furb");
}
