use std::time::Duration;

use super::*;
use crate::cast::Event;

fn at(millis: u64, what: What) -> Event {
  Event { at: Duration::from_millis(millis), what }
}

#[test]
fn the_measures_say_how_soon_a_key_shows_and_each_row_that_flickers() {
  let wrote = |text: &str| What::Output(text.to_owned());
  let cast = Cast {
    cols: 4,
    rows: 1,
    events: vec![
      at(0, wrote("ab")),
      at(100, What::Input("<Enter>".to_owned())),
      at(130, wrote("\rcd")),
      at(150, wrote("\rab")),
      at(400, wrote("\rc")),
    ],
  };
  let span = Span {
    from: Duration::ZERO,
    until: Duration::from_millis(500),
    every: Duration::ZERO,
    rows: None,
  };
  let (screens, keys) = shown(&cast, span).unwrap();
  let measures = measures(&screens, &keys, span, moving(&screens, None));
  let said = [
    "span 0.000 s to 0.500 s: 5 screens",
    "moving cells: rows 0 to 0, columns 0 to 1",
    "key \"<Enter>\" at 0.100 s: first change after 30 ms",
    "changes of the moving cells: 4 from 0.000 s to 0.400 s, 7.5 per second while they move",
    "intervals: mean 133 ms, least 20, most 250, spread 93 ms (69% of the mean)",
    "flicker: 2 cells on 1 row",
    "  row 0 columns 0 to 1, 2 cells: \"cd\" for 20 ms at 0.130 s",
  ];
  assert_eq!(measures, said.map(|line| line.to_owned() + "\n").concat());
}
