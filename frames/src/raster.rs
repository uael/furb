//! The image of a frame: each cell drawn on its ground with the face `JetBrains Mono`, at the size
//! where a cell is 10 pixels wide. A glyph that the font lacks is drawn as a box, and named.

use std::{collections::HashMap, sync::LazyLock};

use fontdue::{Font, FontSettings, Metrics};

use crate::frame::{BOLD, Cell, Frame, ITALIC, Rgb, STRIKE, UNDERLINE};

/// The size of the font, at which its advance is 10 pixels: the face advances 0.6 of it.
const PX: f32 = 10.0 / 0.6;
const WIDTH: usize = 10;

/// The three faces, which the program carries, under the license that `fonts/OFL.txt` holds.
#[expect(clippy::expect_used, reason = "the faces that the program carries parse")]
static FACES: LazyLock<[Font; 3]> = LazyLock::new(|| {
  let face = |bytes: &[u8]| {
    Font::from_bytes(bytes, FontSettings { scale: PX, ..FontSettings::default() })
      .expect("the faces that the program carries parse")
  };
  [
    face(include_bytes!("../fonts/JetBrainsMono-Regular.ttf")),
    face(include_bytes!("../fonts/JetBrainsMono-Bold.ttf")),
    face(include_bytes!("../fonts/JetBrainsMono-Italic.ttf")),
  ]
});

/// The height of a cell and the row of its base line, from the face: a line of box drawing then
/// meets the line above and the line below.
#[expect(clippy::expect_used, reason = "the regular face that the program carries has its metrics")]
fn rows() -> (usize, usize) {
  let line = FACES[0].horizontal_line_metrics(PX).expect("the regular face has horizontal metrics");
  (to_pixels(line.ascent - line.descent), to_pixels(line.ascent))
}

/// The image of a frame as a PNG, and each glyph that no face holds, once, in its order.
pub(crate) fn png(frame: &Frame) -> Result<(Vec<u8>, Vec<char>), png::EncodingError> {
  let (height, base) = rows();
  let mut canvas = Canvas {
    width: frame.cols * WIDTH,
    pixels: vec![0; frame.cols * WIDTH * frame.rows * height * 3],
  };
  let mut glyphs: HashMap<(char, usize), (Metrics, Vec<u8>)> = HashMap::new();
  let mut missing = Vec::new();
  for (row, cells) in frame.cells().iter().enumerate() {
    for (col, cell) in cells.iter().enumerate() {
      let (x, y) = (col * WIDTH, row * height);
      canvas.fill(Area { x, y, width: WIDTH, height }, cell.bg);
      let wide = cells.get(col + 1).is_some_and(|next| next.glyph.is_empty());
      let place =
        Place { area: Area { x, y, width: if wide { 2 * WIDTH } else { WIDTH }, height }, base };
      let Some(glyph) = cell.glyph.chars().next().filter(|glyph| !glyph.is_whitespace()) else {
        canvas.lines(cell, place);
        continue;
      };
      if canvas.shape(glyph, cell.fg, place) {
        canvas.lines(cell, place);
        continue;
      }
      let face = face(cell.attributes, glyph);
      if face.is_none() && !missing.contains(&glyph) {
        missing.push(glyph);
      }
      match face {
        Some(face) => {
          let rasterized =
            glyphs.entry((glyph, face)).or_insert_with(|| FACES[face].rasterize(glyph, PX));
          canvas.glyph(rasterized, cell, place);
        }
        None => canvas.outline(cell.fg, place),
      }
      canvas.lines(cell, place);
    }
  }
  Ok((canvas.encoded(frame.rows * height)?, missing))
}

/// The face of a glyph in a look: bold, italic or regular, or the regular one when the face of the
/// look lacks it. None when no face holds it.
fn face(attributes: u8, glyph: char) -> Option<usize> {
  let wanted = if attributes & BOLD != 0 {
    1
  } else if attributes & ITALIC != 0 {
    2
  } else {
    0
  };
  [wanted, 0].into_iter().find(|&face| FACES[face].lookup_glyph_index(glyph) != 0)
}

/// A rectangle of pixels.
#[derive(Clone, Copy)]
struct Area {
  x: usize,
  y: usize,
  width: usize,
  height: usize,
}

/// The pixels of a cell, or of the two cells of a wide glyph, and the row of its base line in it.
#[derive(Clone, Copy)]
struct Place {
  area: Area,
  base: usize,
}

/// The pixels of an image, three bytes each.
struct Canvas {
  width: usize,
  pixels: Vec<u8>,
}

impl Canvas {
  fn fill(&mut self, area: Area, color: Rgb) {
    for y in area.y..area.y + area.height {
      for x in area.x..area.x + area.width {
        self.blend((x, y), color, 255);
      }
    }
  }

  fn blend(&mut self, (x, y): (usize, usize), color: Rgb, alpha: u8) {
    let at = (y * self.width + x) * 3;
    let Some(pixel) = self.pixels.get_mut(at..at + 3) else { return };
    let alpha = u16::from(alpha);
    for (channel, value) in pixel.iter_mut().zip(color.0) {
      let mixed = (u16::from(*channel) * (255 - alpha) + u16::from(value) * alpha) / 255;
      *channel = u8::try_from(mixed).unwrap_or(u8::MAX);
    }
  }

  /// A glyph on its base line, cut to its cell and to the cell after it for a wide one.
  fn glyph(&mut self, (metrics, coverage): &(Metrics, Vec<u8>), cell: &Cell, place: Place) {
    let Place { area, base } = place;
    let top = i64::try_from(area.y + base).unwrap_or(i64::MAX)
      - i64::try_from(metrics.height).unwrap_or_default()
      - i64::from(metrics.ymin);
    let left = i64::try_from(area.x).unwrap_or_default() + i64::from(metrics.xmin);
    for (at, alpha) in coverage.iter().enumerate().filter(|(_, alpha)| **alpha > 0) {
      let (x, y) = (left + to_i64(at % metrics.width), top + to_i64(at / metrics.width));
      let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else { continue };
      if (area.x..area.x + area.width).contains(&x) && (area.y..area.y + area.height).contains(&y) {
        self.blend((x, y), cell.fg, *alpha);
      }
    }
  }

  /// A glyph of a loader that the face lacks, drawn as its shape: a pattern of braille as its dots,
  /// and a circle with one half filled. False for each other glyph.
  fn shape(&mut self, glyph: char, color: Rgb, place: Place) -> bool {
    let Area { x, y, height, .. } = place.area;
    if let Some(dots) = u32::from(glyph).checked_sub(0x2800).filter(|dots| *dots < 0x100) {
      // The bits of a pattern, by the column and the row of their dot.
      let places = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (0, 3), (1, 3)];
      let lit = places.into_iter().enumerate().filter(|(bit, _)| dots >> bit & 1 == 1);
      for (col, row) in lit.map(|(_, at)| at) {
        let top = y + height * (row + 1) / 5 - 1;
        self.fill(Area { x: x + 2 + col * 4, y: top, width: 2, height: 2 }, color);
      }
      return true;
    }
    let Some(half) = "◐◓◑◒".chars().position(|one| one == glyph) else { return false };
    let (middle, center) = (to_i64(x) * 2 + 9, to_i64(y) * 2 + to_i64(height));
    for row in y..y + height {
      for col in x..x + 10 {
        let (dx, dy) = (to_i64(col) * 2 + 1 - middle, to_i64(row) * 2 + 1 - center);
        let inside = dx * dx + dy * dy <= 64;
        let filled = [dx < 0, dy < 0, dx > 0, dy > 0][half];
        if inside && (filled || dx * dx + dy * dy >= 36) {
          self.blend((col, row), color, 255);
        }
      }
    }
    true
  }

  /// The box that stands for a glyph that no face holds.
  fn outline(&mut self, color: Rgb, place: Place) {
    let Area { x, y, width, height } = place.area;
    let (across, down) = (width - 2, height - 6);
    self.fill(Area { x: x + 1, y: y + 3, width: across, height: 1 }, color);
    self.fill(Area { x: x + 1, y: y + height - 4, width: across, height: 1 }, color);
    self.fill(Area { x: x + 1, y: y + 3, width: 1, height: down }, color);
    self.fill(Area { x: x + width - 2, y: y + 3, width: 1, height: down }, color);
  }

  /// The line under a text, or through it.
  fn lines(&mut self, cell: &Cell, place: Place) {
    let Place { area, base } = place;
    let line = |row: usize| Area { y: area.y + row, height: 1, ..area };
    if cell.attributes & UNDERLINE != 0 {
      self.fill(line(base + 2), cell.fg);
    }
    if cell.attributes & STRIKE != 0 {
      self.fill(line(base - 5), cell.fg);
    }
  }

  fn encoded(&self, height: usize) -> Result<Vec<u8>, png::EncodingError> {
    let mut bytes = Vec::new();
    let size = |pixels: usize| u32::try_from(pixels).unwrap_or(u32::MAX);
    let mut encoder = png::Encoder::new(&mut bytes, size(self.width), size(height));
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&self.pixels)?;
    writer.finish()?;
    Ok(bytes)
  }
}

fn to_pixels(length: f32) -> usize {
  // A length of a face at this size is a few dozen pixels.
  #[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a small positive length"
  )]
  let pixels = length.ceil() as usize;
  pixels
}

fn to_i64(value: usize) -> i64 {
  i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[path = "raster.test.rs"]
mod test;
