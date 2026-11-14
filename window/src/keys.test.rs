use super::{Does, System, drawn};

/// The keys that the window names, as a system writes them.
fn named(system: System) -> Vec<String> {
  let does =
    [Does::Commands, Does::Needed, Does::Next, Does::Keys, Does::List, Does::Stop, Does::Jump];
  does.map(|one| drawn(one, system)).to_vec()
}

#[test]
fn on_macos_a_key_shows_the_signs_of_its_keys_then_its_key_with_no_plus() {
  assert_eq!(named(System::MacOs), ["⌃P", "⌥A", "⌥↓", "⌃X ?", "⌃X L", "⎋ ⎋", "↘"]);
}

#[test]
fn on_another_system_a_key_shows_each_key_that_it_holds_as_a_word() {
  assert_eq!(
    named(System::Other),
    ["Ctrl+P", "Alt+A", "Alt+↓", "Ctrl+X ?", "Ctrl+X L", "Esc Esc", "End"]
  );
}
