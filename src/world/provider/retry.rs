//! The failures of a call of a model that pass, and when the provider calls the model again.

use std::time::Duration;

use jiff::{Zoned, civil, fmt::strtime, tz::TimeZone};
use rig_core::{completion::CompletionError, http_client};

use super::claude;

/// A retry of a call of a model: the attempt that comes next, the most attempts of the call, the
/// time the provider waits before that attempt, and why the attempt before it failed, in a few
/// words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retry {
  pub attempt: u32,
  pub most: u32,
  pub wait: Duration,
  pub why: String,
}

/// How the provider asks a model again after a failure that passes: the most attempts of one call,
/// and the wait before the second attempt, which doubles before each later attempt, to a minute at
/// most.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Backoff {
  pub most: u32,
  pub first: Duration,
}

/// The backoff of a provider that is given none: ten attempts, and four seconds before the second.
pub const BACKOFF: Backoff = Backoff { most: 10, first: Duration::from_secs(4) };

impl Backoff {
  /// The wait before an attempt, from the second on.
  pub(super) fn wait(self, attempt: u32) -> Duration {
    let doubled = self.first.saturating_mul(1 << attempt.saturating_sub(2).min(16));
    doubled.min(Duration::from_secs(60))
  }
}

/// A cause of a failed call that passes, so the provider asks the model again. Its text is the why
/// of the retry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum Transient {
  /// The program of the claude command line was missing, as while an update replaces it.
  #[error("the program is missing")]
  Missing,
  /// The account of the claude command line was parked, and the next call runs as another one.
  #[error("{0}")]
  Switched(String),
  #[error("connection reset")]
  Reset,
  #[error("timed out")]
  TimedOut,
  #[error("the provider is overloaded")]
  Overloaded,
  #[error("rate limited")]
  RateLimited,
  #[error("connection lost")]
  Lost,
  /// A status of the side of the provider: 500, 502, 503 or 504.
  #[error("server error {0}")]
  Server(u16),
  #[error("network error")]
  Network,
}

/// The HTTP status that a failure carries: the status of an HTTP error of rig, or the status that
/// heads the text of a provider, as `503 Service Unavailable`, or that follows `status code`, as
/// rig words it. A number elsewhere in the text is no status.
pub(super) fn status(no: &CompletionError) -> Option<u16> {
  let said = match no {
    CompletionError::HttpError(
      http_client::Error::InvalidStatusCode(code)
      | http_client::Error::InvalidStatusCodeWithMessage(code, _)
      | http_client::Error::InvalidStatusCodeWithDetails { status: code, .. },
    ) => return Some(code.as_u16()),
    CompletionError::HttpError(no) => no.to_string(),
    CompletionError::ProviderError(why) => why.clone(),
    _ => return None,
  };
  let lower = said.to_lowercase();
  let after = lower.split_once("status code").map_or(lower.as_str(), |(_, rest)| rest.trim_start());
  let head: String = after.chars().take_while(char::is_ascii_digit).collect();
  let ends = after[head.len()..].chars().next().is_none_or(|one| !one.is_ascii_alphanumeric());
  if head.len() == 3 && ends { head.parse().ok() } else { None }
}

/// Why a call of a model failed, when the cause passes: the connection was lost or timed out, the
/// provider is overloaded, limits the rate, or failed on its side, or the program of the claude
/// command line was missing, as while an update replaces it. A failure of the request, of the turn,
/// or of a model that the provider cannot make does not pass, and the model is not asked again.
pub(super) fn passing(no: &CompletionError) -> Option<Transient> {
  match claude::error(no) {
    Some(claude::Error::Missing(_)) => return Some(Transient::Missing),
    Some(why @ claude::Error::Switched(_)) => return Some(Transient::Switched(why.to_string())),
    Some(_) => return None,
    None => {}
  }
  let said = match no {
    CompletionError::HttpError(no) => no.to_string(),
    CompletionError::ProviderError(why) => why.clone(),
    _ => return None,
  }
  .to_lowercase();
  let has = |words: &[&str]| words.iter().any(|one| said.contains(one));
  let status = status(no);
  let why = if has(&["connection reset", "reset by peer"]) {
    Transient::Reset
  } else if has(&["timed out", "timeout"]) || status == Some(408) {
    Transient::TimedOut
  } else if has(&["overloaded"]) || status == Some(529) {
    Transient::Overloaded
  } else if has(&["rate limit", "too many requests"]) || status == Some(429) {
    Transient::RateLimited
  } else if has(&["connection closed", "connection error", "broken pipe", "error sending request"])
  {
    Transient::Lost
  } else if let Some(code @ (500 | 502 | 503 | 504)) = status {
    Transient::Server(code)
  } else if has(&[
    "connection refused",
    "could not resolve",
    "dns error",
    "tls handshake",
    "network is unreachable",
  ]) {
    Transient::Network
  } else {
    return None;
  };
  Some(why)
}

/// A failed call in words: a failure of the claude command line as a provider error, as it always
/// read.
pub(super) fn failure(no: &CompletionError) -> String {
  claude::error(no).map_or_else(|| no.to_string(), |why| format!("ProviderError: {why}"))
}

/// Whether a failed call can be corrected by another turn of the model: a transient provider fault,
/// or a cut turn. A request that does not fit the context of the model is not: the next turn holds
/// the same turns and more, so it fails the same way, and a chain that asks again at once loops.
pub(super) fn recoverable(no: &CompletionError) -> bool {
  if passing(no).is_some() {
    return true;
  }
  let text = match no {
    CompletionError::ResponseError(text) | CompletionError::ProviderError(text) => {
      text.to_lowercase()
    }
    CompletionError::HttpError(error) => error.to_string().to_lowercase(),
    _ => return false,
  };
  ["reached the most it writes before its end", "stopped before its end"]
    .iter()
    .any(|phrase| text.contains(phrase))
}

/// What the refusal of a reply says first when its prompt passes the window of its model, which the
/// extension compaction reads to cut the chain before its next reply.
pub(super) const OVERFLOW: &str = "the prompt passes the window of the model";

/// Whether a failed call refused a prompt that passes the context of the model.
pub(super) fn overflows(no: &CompletionError) -> bool {
  let text = failure(no).to_lowercase().replace('_', " ");
  ["prompt is too long", "context window", "context length", "maximum context", "too many tokens"]
    .iter()
    .any(|phrase| text.contains(phrase))
}

/// The wait before the claude command line is asked again when its message gives no moment for the
/// end of a limit.
pub(super) const LIMIT_POLL: Duration = Duration::from_secs(300);

/// The wait before the claude command line is asked again after a failed sign in, which a sign in
/// outside furb ends.
pub(super) const SIGN_POLL: Duration = Duration::from_secs(60);

/// The time after the end of a limit that the provider waits, so the call is not made before the
/// limit lifts.
pub(super) const LIMIT_SLACK: Duration = Duration::from_secs(1);

/// The moment a usage limit ends, from the message of the claude command line: `resets 5:10pm
/// (Europe/Paris)`, with a day before the time, as `resets Oct 28, 3pm`, when it ends in a later
/// day, a time with no zone being in the zone of the machine. A time with no day is the next one
/// after now.
pub(super) fn resets(said: &str, now: &Zoned) -> Option<Zoned> {
  let rest = said.split_once("resets")?.1.trim();
  let (head, zone) = match rest.split_once('(') {
    Some((head, zone)) => (head, TimeZone::get(zone.split_once(')')?.0.trim()).ok()?),
    None => (rest, now.time_zone().clone()),
  };
  let now = now.with_time_zone(zone.clone());
  let words: Vec<_> =
    head.split([' ', ',']).filter(|one| !one.is_empty() && *one != "at").collect();
  let (time, day) = words.split_last()?;
  // The clock as jiff reads it, `5:10pm` or `3pm`, whose hour is of 1 to 12.
  let time = ["%I:%M%p", "%I%p"]
    .iter()
    .find_map(|format| strtime::parse(format, time).ok())?
    .to_time()
    .ok()?;
  let at = |date: civil::Date| date.to_datetime(time).to_zoned(zone.clone()).ok();
  match day {
    [] => {
      let today = at(now.date())?;
      if today > now { Some(today) } else { at(now.date().tomorrow().ok()?) }
    }
    [month, day] => {
      // The month, whole or short, and the day, as jiff reads them.
      let date = format!("{month} {day}");
      let named =
        ["%B %d", "%b %d"].iter().find_map(|format| strtime::parse(format, &date).ok())?;
      let (month, day) = (named.month()?, named.day()?);
      let this = at(civil::Date::new(now.year(), month, day).ok()?)?;
      if this > now { Some(this) } else { at(civil::Date::new(now.year() + 1, month, day).ok()?) }
    }
    _ => None,
  }
}
