//! The compare of a scenario: the frame of the window beside the two winner frames of the judge of
//! the other programs, the scores of the winners and of the window on each aspect, and the
//! findings of the probes. The frames and the judgment of the other programs stand outside the
//! repository, in the folder that `FURB_RIVALS` names, with the judgment in `BEST.md`; with no such
//! folder, the compare shows the window alone.

use std::{
  fmt::Write as _,
  fs,
  path::{Path, PathBuf},
};

use unicode_width::UnicodeWidthStr;

use crate::{
  error::Error,
  frame::Frame,
  raster,
  replay::put,
  scenario::{Has, SIZES, Scenario, Size},
};

/// The other programs, as `BEST.md` names them, and the folder of the frames of each.
const PROGRAMS: [(&str, &str); 4] =
  [("Claude Code", "claude"), ("Codex", "codex"), ("OpenCode", "opencode"), ("Pi", "pi")];

/// A winner of a scenario: its folder, its rank, its step, and its name in `BEST.md`.
struct Winner {
  program: &'static str,
  folder: &'static str,
  rank: usize,
  step: String,
}

/// A frame in a column of the compare: its head, its rows and its width.
struct Column {
  head: String,
  lines: Vec<String>,
  width: usize,
}

impl Column {
  fn of(head: String, text: &str) -> Self {
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let width = lines.iter().map(|line| line.width()).max().unwrap_or(0);
    Self { head, lines, width }
  }
}

/// The compare of a scenario as text, from the captures in `out` and the folder of the other
/// programs, when there is one. The images of the winners are drawn in `<out>/rivals/`.
pub(crate) fn compare(
  scenario: &Scenario,
  out: &Path,
  rivals: Option<&Path>,
) -> Result<String, Error> {
  let mut text = format!("# {}\n\n", scenario.name);
  let best = rivals.and_then(|folder| fs::read_to_string(folder.join("BEST.md")).ok());
  let winners = best.as_deref().map(|best| winners(best, scenario.name)).unwrap_or_default();
  if let Has::None(gap) = &scenario.has {
    write!(text, "furb has none: the gap {gap:?} of BEST.md\n\n")?;
  }
  let mut theirs = Vec::new();
  for winner in &winners {
    if let Some(drawn) =
      rivals.map(|folder| drawn(folder, out, scenario.name, winner)).transpose()?.flatten()
    {
      theirs.push(drawn);
    }
  }
  let window = window(scenario, &winners);
  for (size, name, step) in &window {
    let path = |extension: &str| out.join(size.name()).join(format!("{name}.{step}.{extension}"));
    let Ok(frame) = fs::read_to_string(path("txt")) else {
      write!(
        text,
        "no capture of {name} {step} at {}: run `furb-frames capture {name}`\n\n",
        size.name()
      )?;
      continue;
    };
    let ours = Column {
      width: size.cols,
      ..Column::of(format!("furb {name} {step} {}", size.name()), &frame)
    };
    let columns: Vec<&Column> =
      [&ours].into_iter().chain(theirs.iter().map(|(column, _)| column)).collect();
    text += &beside(&columns);
    write!(text, "\nimages: {}\n", path("png").display())?;
  }
  if window.is_empty() {
    let columns: Vec<&Column> = theirs.iter().map(|(column, _)| column).collect();
    text += &beside(&columns);
  }
  for (_, image) in &theirs {
    writeln!(text, "        {}", image.display())?;
  }
  text += &match &best {
    Some(best) => scores(best, scenario.name, &winners),
    None => {
      "\nno frames of the other programs: FURB_RIVALS names no folder with BEST.md\n".to_owned()
    }
  };
  let probes =
    fs::read_to_string(out.join(format!("{}.probes.txt", scenario.name))).unwrap_or_default();
  write!(text, "\nprobes:\n{}", if probes.is_empty() { "none\n" } else { &probes })?;
  Ok(text)
}

/// The frames of the window that a scenario compares, each by its size, its scenario and its step:
/// the step of the first winner, else the first step of the walk, or the steps of other scenarios
/// that it names at one size.
fn window(scenario: &Scenario, winners: &[Winner]) -> Vec<(Size, String, String)> {
  match &scenario.has {
    Has::Walk(walk) => {
      let wanted = winners.first().and_then(|winner| winner.step.split('.').next());
      let step = walk.steps.iter().find(|step| Some(step.name) == wanted).or(walk.steps.first());
      step
        .map(|step| (SIZES[0], scenario.name.to_owned(), step.name.to_owned()))
        .into_iter()
        .collect()
    }
    Has::Sized(size, steps) => {
      steps.iter().map(|(name, step)| (*size, (*name).to_owned(), (*step).to_owned())).collect()
    }
    Has::None(_) => Vec::new(),
  }
}

/// The frame of a winner as a column, and its image, which the rasterizer of the window draws from
/// its cells in `<out>/rivals/`. A step of motion takes its first frame. None when the folder holds
/// no such frame.
fn drawn(
  rivals: &Path,
  out: &Path,
  scenario: &str,
  winner: &Winner,
) -> Result<Option<(Column, PathBuf)>, Error> {
  let folder = rivals.join(winner.folder);
  let stem = [format!("{scenario}.{}", winner.step), format!("{scenario}.{}.1", winner.step)]
    .into_iter()
    .find(|stem| folder.join(format!("{stem}.txt")).is_file());
  let Some(stem) = stem else { return Ok(None) };
  let path = folder.join(format!("{stem}.txt"));
  let text = fs::read_to_string(&path).map_err(|source| Error::Read { path, source })?;
  let frame = Frame::read(&folder.join(format!("{stem}.json")))?;
  let images = out.join("rivals");
  fs::create_dir_all(&images).map_err(|source| Error::Write { path: images.clone(), source })?;
  let image =
    images.join(format!("{scenario}.{}-{}.{}.png", winner.rank, winner.folder, winner.step));
  put(&image, &raster::png(&frame)?.0)?;
  let head = format!("{} {} {}", winner.rank, winner.program, winner.step);
  Ok(Some((Column::of(head, &text), image)))
}

/// The two winners of a scenario, from its row of the table of the answer of `BEST.md`, as
/// `| 05 | prompt-box | OpenCode `d` | Claude Code `c` | ...`.
fn winners(best: &str, scenario: &str) -> Vec<Winner> {
  let (number, name) = scenario.split_once('-').unwrap_or((scenario, ""));
  let row = best.lines().find(|line| line.starts_with(&format!("| {number} | {name} |")));
  let cells: Vec<&str> = row.map(|row| row.split('|').map(str::trim).collect()).unwrap_or_default();
  cells
    .iter()
    .skip(3)
    .take(2)
    .enumerate()
    .filter_map(|(at, cell)| {
      let (program, step) = cell.split_once(" `")?;
      let &(program, folder) = PROGRAMS.iter().find(|one| one.0 == program)?;
      Some(Winner { program, folder, rank: at + 1, step: step.trim_end_matches('`').to_owned() })
    })
    .collect()
}

/// The score of each aspect of the winners and of the window, from the table of the part of the
/// scenario in `BEST.md`.
fn scores(best: &str, scenario: &str, winners: &[Winner]) -> String {
  let part =
    best.split(&format!("\n## {}\n", scenario.replacen('-', " ", 1))).nth(1).unwrap_or_default();
  let mut rows = part
    .lines()
    .skip_while(|line| !line.starts_with("| Aspect"))
    .take_while(|line| line.starts_with('|'));
  let head: Vec<&str> =
    rows.next().map(|head| head.split('|').map(str::trim).collect()).unwrap_or_default();
  let wanted: Vec<(String, usize)> = winners
    .iter()
    .map(|winner| winner.program)
    .chain(["furb"])
    .filter_map(|program| {
      Some((program.to_owned(), head.iter().position(|cell| *cell == program)?))
    })
    .collect();
  let mut text = vec![String::from("\nscores:")];
  for row in rows.skip(1) {
    let cells: Vec<&str> = row.split('|').map(str::trim).collect();
    let each: Vec<String> = wanted
      .iter()
      .map(|(program, at)| format!("{program} {}", cells.get(*at).unwrap_or(&"")))
      .collect();
    text.push(format!("- {}: {}", cells.get(1).unwrap_or(&""), each.join(" | ")));
  }
  text.join("\n") + "\n"
}

/// Frames side by side, each in its own width, under its head.
fn beside(columns: &[&Column]) -> String {
  let rows = columns.iter().map(|column| column.lines.len()).max().unwrap_or(0);
  let pad =
    |text: &str, width: usize| format!("{text}{}", " ".repeat(width.saturating_sub(text.width())));
  let line = |cells: Vec<String>| cells.join(" │ ").trim_end().to_owned() + "\n";
  let mut text = line(columns.iter().map(|column| pad(&column.head, column.width)).collect());
  for row in 0..rows {
    text += &line(
      columns
        .iter()
        .map(|column| pad(column.lines.get(row).map_or("", String::as_str), column.width))
        .collect(),
    );
  }
  text
}

#[cfg(test)]
#[path = "compare.test.rs"]
mod test;
