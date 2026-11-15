//! The box of the context of the open thread: its bar, in the colors of the parts of what the last
//! answer read.

#![cfg(test)]

use opentui::style::Color;

use crate::{
  cards::shows,
  daemon::READS,
  question::opened,
  thread::{LEFT, at, clicked},
};

#[test]
fn the_bar_fills_its_row_and_a_part_that_holds_tokens_takes_a_cell() {
  let (app, _daemon, ..) = opened(&format!("read the trace of the open\n{READS}"), "each chain.");
  shows(&app, "12%");
  clicked(&app, "Context", LEFT, 0);
  shows(&app, "Engine");
  let bar = "━".repeat(35);
  let (x, y) = at(&app, &bar, LEFT);
  app.click(x, y);
  shows(&app, "Written to the cache   8.4k    1%");
  // The parts read 98k, 8.4k and 17k of 1M: the bar gives each a run of its color, the 8.4k too,
  // which holds less than a cell, and the free room the rest.
  let frame = app.render();
  let cells = &frame.cells[usize::from(y) * frame.width as usize..][usize::from(x)..][..35];
  let mut runs: Vec<(Color, usize)> = Vec::new();
  for cell in cells {
    match runs.last_mut() {
      Some((color, count)) if *color == cell.fg => *count += 1,
      _ => runs.push((cell.fg, 1)),
    }
  }
  let counts: Vec<usize> = runs.iter().map(|one| one.1).collect();
  assert_eq!(counts.len(), 4, "{runs:?}");
  assert!(counts[1] == 1 && counts.iter().sum::<usize>() == 35, "{runs:?}");
}
