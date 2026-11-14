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
