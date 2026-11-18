use opentui::{
  native::{Context, ContextOptions, WidthMethod},
  style::Style,
};

use super::{Cells, METHOD};
use crate::line::{run, spread};

/// Characters whose cells differ between a terminal that joins them and one that adds up their
/// code points: a hand with a tone, a joined flag with and without the selectors of an emoji, a
/// flag of two letters, a letter with an accent that joins it, a wide letter, and a sign that the
/// terminal draws as text.
const SHOWN: [&str; 7] = ["👍🏽", "🏳️\u{200d}⚧️", "🏳\u{200d}⚧", "🇫🇷", "e\u{301}", "中", "✡"];

/// The measure of the thread set to a method, as the terminal would tell it.
fn measuring(method: WidthMethod) {
  METHOD.with(|held| held.set(method));
}

#[test]
fn each_character_takes_the_cells_that_the_renderer_draws_by_each_method() {
  let renderer = Context::new(ContextOptions::default()).expect("a native context");
  let mut hand = Vec::new();
  for method in [WidthMethod::Unicode, WidthMethod::Wcwidth] {
    measuring(method);
    for shown in SHOWN {
      let drawn = renderer.text_width(shown, method).expect("a measure");
      assert_eq!(shown.width(), usize::try_from(drawn).unwrap_or_default(), "{shown} {method:?}");
    }
    hand.push(SHOWN[0].width());
    // A row ends at its room whatever it holds, as the renderer counts its cells.
    let left = SHOWN.iter().map(|one| run(*one, Style::new())).collect();
    let row = spread(left, vec![run("end", Style::new())], 40, Style::new());
    let text: String = row.iter().map(|one| one.content.as_ref()).collect();
    assert_eq!(renderer.text_width(&text, method).expect("a measure"), 40, "{text} {method:?}");
  }
  // The two methods differ on the hand, so the measure follows the method.
  assert_eq!(hand, [2, 4]);
  measuring(WidthMethod::Unicode);
}
