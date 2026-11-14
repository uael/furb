use opentui::style::Color;

use super::Tokens;

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
  for text in texts.into_iter().chain([tokens.done]).chain(tokens.hues) {
    for surface in [tokens.base, tokens.raised, tokens.lifted] {
      let ratio = contrast(text, surface);
      assert!(ratio >= 4.5, "{text:?} on {surface:?} has a contrast of {ratio:.2}");
    }
  }
}

#[test]
fn a_surface_is_the_background_of_the_terminal_mixed_toward_the_text() {
  let tokens = Tokens::on(Some(Color::rgb(0, 0, 0)));
  assert_eq!(tokens.base, Color::default_with(0, 0, 0));
  // 6 and 11 parts of 100 of the text, 0xe3e3e6.
  assert_eq!(tokens.raised, Color::rgb(13, 13, 13));
  assert_eq!(tokens.lifted, Color::rgb(24, 24, 25));
}

#[test]
fn a_light_terminal_takes_the_dark_background_of_the_window() {
  assert_eq!(Tokens::on(Some(Color::rgb(250, 250, 250))), Tokens::on(None));
}
