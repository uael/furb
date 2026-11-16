//! How a person reads a time, a size and a count, in each client the same words: the time that a
//! work took or takes, how long ago a moment was, a count of bytes, of entries and of tokens, and
//! a path with the home folder as `~`.
#![expect(clippy::indexing_slicing, reason = "debt: an index or a slice that can panic")]

use std::time::Duration;

use serde_json::Value;

/// A count of seconds as a person reads it: `4 s`, `1m 05s`, `2h 03m`.
pub fn timer(seconds: u64) -> String {
  match seconds {
    ..60 => format!("{seconds} s"),
    60..3600 => format!("{}m {:02}s", seconds / 60, seconds % 60),
    3600.. => format!("{}h {:02}m", seconds / 3600, seconds / 60 % 60),
  }
}

/// How many whole seconds passed from a millisecond of the epoch to another, from one second on: a
/// time of a work shows only after a whole second of it.
pub fn since(began: u64, now: u64) -> Option<u64> {
  Some(now.saturating_sub(began) / 1000).filter(|seconds| *seconds > 0)
}

/// How long ago a second of the epoch was at another, from one minute on: `5m`, `3h`, `2d`.
pub fn age(since: u64, now: u64) -> Option<String> {
  match now.saturating_sub(since) {
    ..60 => None,
    age @ ..3600 => Some(format!("{}m", age / 60)),
    age @ ..86400 => Some(format!("{}h", age / 3600)),
    age => Some(format!("{}d", age / 86400)),
  }
}

/// A time that a short work took: tenths of a second under ten seconds, then as [`timer`].
pub fn elapsed(took: Duration) -> String {
  if took < Duration::from_secs(10) {
    let tenths = took.as_millis() / 100;
    format!("{}.{} s", tenths / 10, tenths % 10)
  } else {
    timer(took.as_secs())
  }
}

/// A count of bytes, rounded, in the first unit by steps of a thousand where it stays under a
/// thousand.
pub fn size(bytes: u64) -> String {
  let units = ["B", "KB", "MB", "GB", "TB"];
  let rounded = |unit: u32| (bytes + 1000_u64.pow(unit) / 2) / 1000_u64.pow(unit);
  let unit = (0..4).find(|unit| rounded(*unit) < 1000).unwrap_or(4);
  format!("{} {}", rounded(unit), units[unit as usize])
}

/// A count with a space between each group of three digits, so the counts of a column align.
pub fn grouped(count: u64) -> String {
  let digits = count.to_string();
  let mut out = String::new();
  for (at, digit) in digits.chars().enumerate() {
    if at > 0 && (digits.len() - at).is_multiple_of(3) {
      out.push(' ');
    }
    out.push(digit);
  }
  out
}

/// A count of tokens: `840`, `8.4k`, `124k`, `1.2M`, `1M`, each cut and not rounded up, so a count
/// never shows more than it is.
pub fn tokens(count: u64) -> String {
  // The decimal shows unless it is 0, as a window of `1M`.
  let decimal = |unit: u64| match count / (unit / 10) % 10 {
    0 => (count / unit).to_string(),
    tenth => format!("{}.{tenth}", count / unit),
  };
  match count {
    0..1_000 => count.to_string(),
    1_000..10_000 => format!("{}k", decimal(1_000)),
    10_000..1_000_000 => format!("{}k", count / 1_000),
    _ => format!("{}M", decimal(1_000_000)),
  }
}

/// A count of lines: `1 line`, `297 lines`.
pub fn lines(count: usize) -> String {
  format!("{count} line{}", if count == 1 { "" } else { "s" })
}

/// A part of a whole as a share in percent, rounded to the nearest, where a part that holds
/// something never reads as none: `<1%`, `12%`.
pub fn share(part: u64, whole: u64) -> String {
  match percent(part, whole) {
    0 if part > 0 => "<1%".to_owned(),
    share => format!("{share}%"),
  }
}

/// A part of a whole in percent, rounded to the nearest.
pub fn percent(part: u64, whole: u64) -> u64 {
  (part.saturating_mul(100) + whole.max(1) / 2) / whole.max(1)
}

/// A text with a home folder as `~`, as a person reads a path of his own machine: a path whose
/// parts a `/` parts, or a `\` on Windows.
pub fn homed(text: &str, home: &str) -> String {
  let text = text.replace(&format!("{home}/"), "~/").replace(&format!("{home}\\"), "~\\");
  if text == home { "~".to_owned() } else { text }
}

/// The word of a rung of the operator that names a session, with the name in JSON between its two
/// parts: the act `rename_session` on the chain of the rung, which the root folds.
const RENAMING: (&str, &str) =
  ("act(\"rename_session\", \"\", lambda id: (yield \"done\", id, ", "))");

/// The word of the operator on the root that names the session, which an empty name gives back
/// its default name.
pub fn renaming(name: &str) -> String {
  format!("{}{}{}", RENAMING.0, Value::String(name.to_owned()), RENAMING.1)
}

/// The name that a word says, when it is the word that names a session.
pub fn renamed(word: &str) -> Option<String> {
  let inner = word.strip_prefix(RENAMING.0)?.strip_suffix(RENAMING.1)?;
  serde_json::from_str::<String>(inner).ok()
}
