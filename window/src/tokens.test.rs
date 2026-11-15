use opentui::style::Color;

use std::time::Duration;

use super::{Loader, Tokens, WORKS};

/// The relative luminance of a color, as WCAG 2 defines it.
fn luminance(color: Color) -> f64 {
  let linear = |channel: u8| {
    let share = f64::from(channel) / 255.0;
    if share <= 0.03928 { share / 12.92 } else { ((share + 0.055) / 1.055).powf(2.4) }
  };
  0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

fn contrast(one: Color, other: Color) -> f64 {
  let (light, dark) = (luminance(one).max(luminance(other)), luminance(one).min(luminance(other)));
  (light + 0.05) / (dark + 0.05)
}

#[test]
fn each_color_of_a_text_keeps_a_contrast_of_4_5_to_1_on_each_surface() {
  let tokens = Tokens::on(None);
  let texts = [tokens.text, tokens.muted, tokens.accent, tokens.attention, tokens.danger];
  for text in texts.into_iter().chain(tokens.hues) {
    for surface in [tokens.base, tokens.panel, tokens.raised, tokens.lifted] {
      let ratio = contrast(text, surface);
      assert!(ratio >= 4.5, "{text:?} on {surface:?} has a contrast of {ratio:.2}");
    }
  }
}

#[test]
fn a_surface_is_the_background_of_the_terminal_mixed_toward_white_by_a_step_of_the_scale() {
  let tokens = Tokens::on(Some(Color::rgb(0, 0, 0)));
  assert_eq!(tokens.base, Color::default_with(0, 0, 0));
  // 3, 7, 13 and 26 parts of 100 of white.
  let steps = [tokens.panel, tokens.raised, tokens.lifted, tokens.border];
  assert_eq!(steps, [7, 17, 33, 66].map(|grey| Color::rgb(grey, grey, grey)));
}

#[test]
fn a_light_terminal_takes_the_dark_background_of_the_window() {
  assert_eq!(Tokens::on(Some(Color::rgb(250, 250, 250))), Tokens::on(None));
}

#[test]
fn a_light_coasts_over_the_word_of_work_from_its_left_then_whitens_it_whole_then_fades() {
  let tokens = Tokens::on(None);
  let cells =
    |millis: u64| Loader::Light(WORKS).cells(Duration::from_millis(millis), tokens.accent, &tokens);
  let bright = |color: Color| u16::from(color.r) + u16::from(color.g) + u16::from(color.b);
  let brightest = |millis: u64| {
    let shown = cells(millis).iter().map(|(_, color)| bright(*color)).collect::<Vec<_>>();
    shown.iter().enumerate().max_by_key(|(at, one)| (**one, usize::MAX - at)).map(|(at, _)| at)
  };
  let fronts = [100, 300, 500, 700, 850].map(brightest);
  assert!(fronts.is_sorted() && fronts[0] < fronts[4], "the light moves right: {fronts:?}");
  let colors = |millis: u64| cells(millis).iter().map(|(_, color)| *color).collect::<Vec<_>>();
  assert_eq!(colors(1000), vec![tokens.hover; WORKS.len()]);
  let fading = colors(1260);
  assert!(fading.iter().all(|one| *one != tokens.hover && *one != tokens.accent));
  assert_eq!(colors(1600), vec![tokens.accent; WORKS.len()]);
  assert_eq!(colors(2400 + 1000), colors(1000));
  let letters: String = cells(0).iter().map(|one| one.0).collect();
  assert_eq!(letters, WORKS);
}

#[test]
fn the_arrow_of_an_agent_pulses_from_muted_to_its_color_and_back() {
  let tokens = Tokens::on(None);
  let at =
    |millis: u64| Loader::Agent.cells(Duration::from_millis(millis), tokens.accent, &tokens)[0].1;
  assert_eq!([at(0), at(600), at(1200)], [tokens.muted, tokens.accent, tokens.muted]);
}
