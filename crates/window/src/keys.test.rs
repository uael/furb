use super::{Does, System, each};

/// A law that no frame of the suite shows, since a headless terminal tells no capability: the form
/// with command of Go to shows only on macOS while the terminal tells the keys that hold super.
#[test]
fn the_form_with_command_shows_only_on_macos_where_the_terminal_tells_super() {
  let go_to = |system, supers| {
    each(system, supers).find(|one| one.1.does == Does::GoTo).map(|one| one.2.join(" or "))
  };
  assert_eq!(go_to(System::MacOs, true).as_deref(), Some("⌃P or ⌘P"));
  assert_eq!(go_to(System::MacOs, false).as_deref(), Some("⌃P"));
  assert_eq!(go_to(System::Other, true).as_deref(), Some("Ctrl+P"));
}
