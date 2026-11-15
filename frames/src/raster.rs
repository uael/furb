//! The image of a frame: each cell drawn on its ground with the face `JetBrains Mono`, at the size
//! where a cell is 10 pixels wide. A glyph that the face lacks is named, and drawn from a face of
//! the system that holds it, as a terminal falls back to one, or as a box when none does.

use std::{collections::HashMap, fs, sync::LazyLock};

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

/// The faces of macOS that hold each glyph of the rivals that `JetBrains Mono` lacks, a face of a
/// terminal first: the frame then shows each mark that a terminal shows, in a face near to its own.
const SYSTEM: [&str; 3] = [
  "/System/Library/Fonts/Menlo.ttc",
  "/System/Library/Fonts/Apple Symbols.ttf",
  "/System/Library/Fonts/Supplemental/STIXTwoMath.otf",
];

/// Each face of `SYSTEM` that stands and parses, read once. A system with none of them draws each
/// glyph that the face lacks as a box, which says so.
static FALLBACKS: LazyLock<Vec<Font>> = LazyLock::new(|| {
  let settings = FontSettings { scale: PX, ..FontSettings::default() };
  SYSTEM
    .iter()
    .filter_map(|path| fs::read(path).ok())
    .filter_map(|bytes| Font::from_bytes(bytes, settings).ok())
    .collect()
});

/// The height of a cell and the row of its base line, from the face: a line of box drawing then
/// meets the line above and the line below.
#[expect(clippy::expect_used, reason = "the regular face that the program carries has its metrics")]
fn rows() -> (usize, usize) {
  let line = FACES[0].horizontal_line_metrics(PX).expect("the regular face has horizontal metrics");
  (to_pixels(line.ascent - line.descent), to_pixels(line.ascent))
}

/// The pixels of an image, three bytes each, row by row.
pub(crate) struct Image {
  pub(crate) width: usize,
  pub(crate) height: usize,
  pub(crate) pixels: Vec<u8>,
}

/// The image of a frame as a PNG, and each glyph that the face lacks, once, in its order.
pub(crate) fn png(frame: &Frame) -> Result<(Vec<u8>, Vec<char>), png::EncodingError> {
  let (image, missing) = image(frame);
  Ok((image.png()?, missing))
}

/// The image of a frame, and each glyph that the face lacks, once, in its order.
pub(crate) fn image(frame: &Frame) -> (Image, Vec<char>) {
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
      if face.is_none_or(|face| face >= FACES.len()) && !missing.contains(&glyph) {
        missing.push(glyph);
      }
      match face {
        Some(face) => {
          let rasterized =
            glyphs.entry((glyph, face)).or_insert_with(|| font(face).rasterize(glyph, PX));
          canvas.glyph(rasterized, cell, place);
        }
        None => canvas.outline(cell.fg, place),
      }
      canvas.lines(cell, place);
    }
  }
  let image = Image { width: canvas.width, height: frame.rows * height, pixels: canvas.pixels };
  (image, missing)
}

/// The face of a glyph in a look: bold, italic or regular, the regular one when the face of the
/// look lacks it, then the first of `FALLBACKS` that holds it, after the three faces. None when no
/// face holds it.
fn face(attributes: u8, glyph: char) -> Option<usize> {
  let wanted = if attributes & BOLD != 0 {
    1
  } else if attributes & ITALIC != 0 {
    2
  } else {
    0
  };
  let system = (0..FALLBACKS.len()).map(|fallback| FACES.len() + fallback);
  [wanted, 0].into_iter().chain(system).find(|&face| font(face).lookup_glyph_index(glyph) != 0)
}

/// A face by its place: the three faces, then `FALLBACKS`.
fn font(face: usize) -> &'static Font {
  FACES.get(face).unwrap_or_else(|| &FALLBACKS[face - FACES.len()])
}

/// A rectangle of pixels.
#[derive(Clone, Copy)]
struct Area {
  x: usize,
  y: usize,
  width: usize,
  height: usize,
}

/// The blocks of quarters of a cell, each with its quarters lit: the upper left, the upper right,
/// the lower left and the lower right, from the lowest bit.
const BLOCKS: [(char, u8); 15] = [
  ('▘', 0b0001),
  ('▝', 0b0010),
  ('▀', 0b0011),
  ('▖', 0b0100),
  ('▌', 0b0101),
  ('▞', 0b0110),
  ('▛', 0b0111),
  ('▗', 0b1000),
  ('▚', 0b1001),
  ('▐', 0b1010),
  ('▜', 0b1011),
  ('▄', 0b1100),
  ('▙', 0b1101),
  ('▟', 0b1110),
  ('█', 0b1111),
];

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

  /// A glyph drawn as its shape and not from the face: a block of quarters of the cell, as a
  /// terminal draws it from the cell and not from its font, so a block meets the block of the cell
  /// beside it, above it and under it with no seam; and a glyph of a loader that the face lacks, a
  /// pattern of braille as its dots and a circle with one half filled. False for each other glyph.
  fn shape(&mut self, glyph: char, color: Rgb, place: Place) -> bool {
    let Area { x, y, height, .. } = place.area;
    if let Some(quarters) = BLOCKS.iter().find(|one| one.0 == glyph).map(|one| one.1) {
      let (left, top) = (WIDTH / 2, height / 2);
      let parts = [(0, 0, left, top), (left, 0, WIDTH - left, top)];
      let parts = parts.into_iter().chain([(0, top, left, height - top)]);
      let parts = parts.chain([(left, top, WIDTH - left, height - top)]);
      for (at, (dx, dy, width, height)) in parts.enumerate() {
        if quarters >> at & 1 == 1 {
          self.fill(Area { x: x + dx, y: y + dy, width, height }, color);
        }
      }
      return true;
    }
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
}

impl Image {
  pub(crate) fn png(&self) -> Result<Vec<u8>, png::EncodingError> {
    let mut bytes = Vec::new();
    let size = |pixels: usize| u32::try_from(pixels).unwrap_or(u32::MAX);
    let mut encoder = png::Encoder::new(&mut bytes, size(self.width), size(self.height));
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&self.pixels)?;
    writer.finish()?;
    Ok(bytes)
  }
}

/// An image whose pixels each hold an alpha after their color, as the window in its frame on a
/// ground that shows through.
pub(crate) struct Framed {
  pub(crate) width: usize,
  pub(crate) height: usize,
  pub(crate) pixels: Vec<u8>,
}

/// The margin around the frame of a window, the height of its bar of title, the radius of its
/// corners, and the colors of its bar, of its dots and of its edge, in the greys of the window.
const MARGIN: usize = 24;
const BAR: usize = 30;
const RADIUS: usize = 10;
const TITLE: Rgb = Rgb::hex(0x16_1616);
const DOT: Rgb = Rgb::hex(0x3a_3a3a);
const EDGE: Rgb = Rgb::hex(0x2a_2a2a);

impl Image {
  /// The image as a terminal window shows it: in a frame with round corners and a bar of title
  /// with three dots, on a ground that shows through, as a README shows a program.
  pub(crate) fn framed(&self) -> Framed {
    let (inner, tall) = (self.width + 2, self.height + BAR + 1);
    let (width, height) = (inner + 2 * MARGIN, tall + 2 * MARGIN);
    let mut pixels = vec![0; width * height * 4];
    for y in 0..tall {
      for x in 0..inner {
        let Some(color) = self.framed_at(x, y) else { continue };
        let at = ((y + MARGIN) * width + x + MARGIN) * 4;
        pixels[at..at + 4].copy_from_slice(&[color.0[0], color.0[1], color.0[2], 255]);
      }
    }
    Framed { width, height, pixels }
  }

  /// The color of a pixel of the frame, none outside its round corners: its edge, its bar with its
  /// dots, or the image under the bar.
  fn framed_at(&self, x: usize, y: usize) -> Option<Rgb> {
    let (inner, tall) = (self.width + 2, self.height + BAR + 1);
    let away = |at: usize, size: usize| RADIUS.saturating_sub(at.min(size - 1 - at));
    let (dx, dy) = (away(x, inner), away(y, tall));
    if dx * dx + dy * dy > RADIUS * RADIUS {
      return None;
    }
    let edge = x == 0 || y == 0 || x == inner - 1 || y == tall - 1;
    let ring = (dx + 1) * (dx + 1) + (dy + 1) * (dy + 1) > RADIUS * RADIUS && dx > 0 && dy > 0;
    if edge || ring {
      return Some(EDGE);
    }
    if y < BAR {
      let dot = (0..3).any(|n| {
        let (cx, cy) = (18 + 20 * n, BAR / 2);
        let (ex, ey) = (x.abs_diff(cx), y.abs_diff(cy));
        ex * ex + ey * ey <= 36
      });
      return Some(if dot { DOT } else { TITLE });
    }
    let (ix, iy) = (x - 1, y - BAR);
    let at = (iy.min(self.height - 1) * self.width + ix.min(self.width - 1)) * 3;
    Some(Rgb([self.pixels[at], self.pixels[at + 1], self.pixels[at + 2]]))
  }
}

impl Framed {
  pub(crate) fn png(&self) -> Result<Vec<u8>, png::EncodingError> {
    let mut bytes = Vec::new();
    let size = |pixels: usize| u32::try_from(pixels).unwrap_or(u32::MAX);
    let mut encoder = png::Encoder::new(&mut bytes, size(self.width), size(self.height));
    encoder.set_color(png::ColorType::Rgba);
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
