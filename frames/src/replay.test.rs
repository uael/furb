use super::*;
use crate::cast::Event;

fn cast(events: &[(u64, What)]) -> Cast {
  let events =
    events.iter().map(|(at, what)| Event { at: Duration::from_millis(*at), what: what.clone() });
  Cast { cols: 2, rows: 1, events: events.collect() }
}

fn names(keys: &[Key]) -> Vec<(&str, u128, String)> {
  keys.iter().map(|key| (key.name.as_str(), key.at.as_millis(), key.frame.text())).collect()
}

#[test]
fn a_marker_shows_what_was_written_before_it_and_a_label_again_takes_its_count() {
  let out = |text: &str| What::Output(text.to_owned());
  let mark = || What::Marker("a".to_owned());
  let keys =
    keys(&cast(&[(0, out("x")), (100, mark()), (150, out("y")), (300, mark())]), None).unwrap();
  assert_eq!(names(&keys), [("a", 100, "x\n".to_owned()), ("a-2", 300, "xy\n".to_owned())]);
}

#[test]
fn a_step_of_time_gives_a_frame_only_where_the_screen_changed() {
  let out = |text: &str| What::Output(text.to_owned());
  let every = Some(Duration::from_millis(100));
  let keys = keys(&cast(&[(50, out("x")), (420, out("y")), (500, What::Other)]), every).unwrap();
  assert_eq!(
    names(&keys),
    [("t000100", 100, "x\n".to_owned()), ("t000500", 500, "xy\n".to_owned())]
  );
}

#[test]
fn a_text_that_the_screen_shows_after_a_marker_is_in_its_replay() {
  let out = |text: &str| What::Output(text.to_owned());
  let recorded = cast(&[(0, out("x")), (100, What::Marker("a".to_owned())), (300, out("y"))]);
  let at = Duration::from_millis;
  assert!(shows(&recorded, at(100), at(400), "xy\n").unwrap());
  assert!(!shows(&recorded, at(100), at(200), "xy\n").unwrap());
}

#[test]
fn a_synchronized_update_shows_whole_at_its_end() {
  let out = |text: &str| What::Output(text.to_owned());
  let mark = || What::Marker("a".to_owned());
  let events = [
    (0, out("x")),
    (10, out("\x1b[?2026h\ry")),
    (20, mark()),
    (30, out("z\x1b[?2026l")),
    (40, mark()),
  ];
  let keys = keys(&cast(&events), None).unwrap();
  assert_eq!(names(&keys), [("a", 20, "x\n".to_owned()), ("a-2", 40, "yz\n".to_owned())]);
}
