//! The image of a frame: each cell drawn on its ground with the face `JetBrains Mono`, at the size
//! where a cell is 10 pixels wide, a symbol that it lacks with a face of symbols, and an emoji with
//! the face `Noto Emoji`. Each machine draws a frame the same way, from these faces alone. A glyph
//! that the faces lack is named, and drawn as a box.

use std::{collections::HashMap, sync::LazyLock};

use fontdue::{Font, FontSettings, Metrics};

use crate::frame::{BOLD, Cell, Frame, ITALIC, Rgb, STRIKE, UNDERLINE};

/// The width of a cell in pixels at a density of one, and the size of the font at which its advance
/// is that width: the face advances 0.6 of it. An image at a density of two takes twice each.
const WIDTH: usize = 10;
const PX: f32 = 10.0 / 0.6;

/// The face of symbols and the face of an emoji among `FACES`.
const SYMBOLS: usize = 3;
const EMOJI: usize = 4;

/// The faces that the program carries, each under the license beside it in `fonts`:
/// - the three of `JetBrains Mono`, under `OFL.txt`;
/// - the face of symbols, under `OFL-FramesSymbols.txt`: `JuliaMono-Regular.ttf` of the release
///   v0.63.2 of cormullion/juliamono, whose `JuliaMono-ttf.tar.gz` has the sha256
///   be6517295198ec5c92bdbaad42f4f6f8d83f921d80512b79f54fe036add95c0c, cut to the symbols from
///   U+2000 to U+2BFF by `pyftsubset --unicodes='U+2000-2BFF' --layout-features=''
///   --no-hinting --name-IDs='*'`, and named `Frames Symbols` in its table of names, since its
///   license keeps the name `JuliaMono` for the face as its authors made it;
/// - the face of an emoji, under `OFL-NotoEmoji.txt`: `NotoEmoji[wght].ttf` of google/fonts at
///   8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5, set to its weight 400 by `fonttools
///   varLib.instancer`, then cut the same way with `--unicodes='*'`. It has no color, which no
///   face of this rasterizer draws, and a joined emoji or a flag shows as its first part.
///
/// No hint and no rule of layout is read by this rasterizer, so each cut draws as its face.
#[expect(clippy::expect_used, reason = "the faces that the program carries parse")]
static FACES: LazyLock<[Font; 5]> = LazyLock::new(|| {
  let face = |bytes: &[u8]| {
    Font::from_bytes(bytes, FontSettings { scale: PX, ..FontSettings::default() })
      .expect("the faces that the program carries parse")
  };
  [
    face(include_bytes!("../fonts/JetBrainsMono-Regular.ttf")),
    face(include_bytes!("../fonts/JetBrainsMono-Bold.ttf")),
    face(include_bytes!("../fonts/JetBrainsMono-Italic.ttf")),
    face(include_bytes!("../fonts/FramesSymbols-Regular.ttf")),
    face(include_bytes!("../fonts/NotoEmoji-Regular.ttf")),
  ]
});

/// The height of a cell and the row of its base line, from the face: a line of box drawing then
/// meets the line above and the line below.
#[expect(clippy::expect_used, reason = "the regular face that the program carries has its metrics")]
fn rows(density: usize) -> (usize, usize) {
  let line = FACES.first().and_then(|one| one.horizontal_line_metrics(px(density)));
  let line = line.expect("the regular face has horizontal metrics");
  (to_pixels(line.ascent - line.descent), to_pixels(line.ascent))
}

/// The size of the font at a density.
fn px(density: usize) -> f32 {
  #[expect(clippy::cast_precision_loss, reason = "a density is one or two")]
  let density = density as f32;
  PX * density
}

/// The pixels of an image, three bytes each, row by row, and how many pixels of it stand for one
/// pixel of an image at a density of one.
pub(crate) struct Image {
  pub(crate) width: usize,
  pub(crate) height: usize,
  pub(crate) pixels: Vec<u8>,
  pub(crate) density: usize,
}

/// The image of a frame as a PNG, and each glyph that the face lacks, once, in its order.
pub(crate) fn png(frame: &Frame) -> Result<(Vec<u8>, Vec<char>), png::EncodingError> {
  let (image, missing) = image(frame, 1);
  Ok((image.png()?, missing))
}

/// The image of a frame at a density, and each glyph that the face lacks, once, in its order.
pub(crate) fn image(frame: &Frame, density: usize) -> (Image, Vec<char>) {
  let (height, base) = rows(density);
  let side = WIDTH * density;
  let mut canvas = Canvas {
    width: frame.cols * side,
    pixels: vec![0; frame.cols * side * frame.rows * height * 3],
    density,
  };
  let mut glyphs: HashMap<(char, usize), (Metrics, Vec<u8>)> = HashMap::new();
  let mut missing = Vec::new();
  for (row, cells) in frame.cells().iter().enumerate() {
    for (col, cell) in cells.iter().enumerate() {
      // The second cell of a wide glyph is drawn with the first, so its ground covers no part of
      // the glyph.
      if col > 0 && cell.glyph.is_empty() {
        continue;
      }
      let (x, y, width) = (col * side, row * height, side);
      let wide = cells.get(col + 1).is_some_and(|next| next.glyph.is_empty());
      let place =
        Place { area: Area { x, y, width: if wide { 2 * width } else { width }, height }, base };
      canvas.fill(place.area, cell.bg);
      let Some(glyph) = cell.glyph.chars().next().filter(|glyph| !glyph.is_whitespace()) else {
        canvas.lines(cell, place);
        continue;
      };
      if canvas.shape(glyph, cell.fg, place) {
        canvas.lines(cell, place);
        continue;
      }
      let face = face(cell.attributes, glyph, wide);
      if face.is_none() && !missing.contains(&glyph) {
        missing.push(glyph);
      }
      match face.and_then(|face| Some((face, FACES.get(face)?))) {
        Some((face, font)) => {
          let rasterized =
            glyphs.entry((glyph, face)).or_insert_with(|| font.rasterize(glyph, px(density)));
          canvas.glyph(rasterized, cell, place);
        }
        None => canvas.outline(cell.fg, place),
      }
      canvas.lines(cell, place);
    }
  }
  let image =
    Image { width: canvas.width, height: frame.rows * height, pixels: canvas.pixels, density };
  (image, missing)
}

/// The face of a glyph in a look: bold, italic or regular, the regular one when the face of the
/// look lacks it, then the face of symbols. A glyph of two cells, as a terminal draws an emoji,
/// takes the face of an emoji first. None when no face holds it.
fn face(attributes: u8, glyph: char, wide: bool) -> Option<usize> {
  let wanted = if attributes & BOLD != 0 {
    1
  } else if attributes & ITALIC != 0 {
    2
  } else {
    0
  };
  let emoji = wide.then_some(EMOJI);
  let faces = emoji.into_iter().chain([wanted, 0, SYMBOLS]);
  faces
    .into_iter()
    .find(|&face| FACES.get(face).is_some_and(|one| one.lookup_glyph_index(glyph) != 0))
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

/// The pixels of an image, three bytes each, at a density.
struct Canvas {
  width: usize,
  pixels: Vec<u8>,
  density: usize,
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
    let k = self.density;
    let cell = WIDTH * k;
    if let Some(quarters) = BLOCKS.iter().find(|one| one.0 == glyph).map(|one| one.1) {
      let (left, top) = (cell / 2, height / 2);
      let parts = [(0, 0, left, top), (left, 0, cell - left, top)];
      let parts = parts.into_iter().chain([(0, top, left, height - top)]);
      let parts = parts.chain([(left, top, cell - left, height - top)]);
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
        let top = y + height * (row + 1) / 5 - k;
        self.fill(Area { x: x + 2 * k + col * 4 * k, y: top, width: 2 * k, height: 2 * k }, color);
      }
      return true;
    }
    let Some(half) = "◐◓◑◒".chars().position(|one| one == glyph) else { return false };
    // In half pixels: a circle of a radius of 4 pixels of a density of one, and its ring of 1.
    let (middle, center) = (to_i64(x) * 2 + to_i64(cell) - 1, to_i64(y) * 2 + to_i64(height));
    let (outer, inner) = (to_i64(8 * k).pow(2), to_i64(6 * k).pow(2));
    for row in y..y + height {
      for col in x..x + cell {
        let (dx, dy) = (to_i64(col) * 2 + 1 - middle, to_i64(row) * 2 + 1 - center);
        let inside = dx * dx + dy * dy <= outer;
        let filled = match half {
          0 => dx < 0,
          1 => dy < 0,
          2 => dx > 0,
          _ => dy > 0,
        };
        if inside && (filled || dx * dx + dy * dy >= inner) {
          self.blend((col, row), color, 255);
        }
      }
    }
    true
  }

  /// The box that stands for a glyph that no face holds.
  fn outline(&mut self, color: Rgb, place: Place) {
    let Area { x, y, width, height } = place.area;
    let k = self.density;
    let (across, down) = (width - 2 * k, height - 6 * k);
    self.fill(Area { x: x + k, y: y + 3 * k, width: across, height: k }, color);
    self.fill(Area { x: x + k, y: y + height - 4 * k, width: across, height: k }, color);
    self.fill(Area { x: x + k, y: y + 3 * k, width: k, height: down }, color);
    self.fill(Area { x: x + width - 2 * k, y: y + 3 * k, width: k, height: down }, color);
  }

  /// The line under a text, or through it.
  fn lines(&mut self, cell: &Cell, place: Place) {
    let Place { area, base } = place;
    let k = self.density;
    let line = |row: usize| Area { y: area.y + row, height: k, ..area };
    if cell.attributes & UNDERLINE != 0 {
      self.fill(line(base + 2 * k), cell.fg);
    }
    if cell.attributes & STRIKE != 0 {
      self.fill(line(base - 5 * k), cell.fg);
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
/// corners, at a density of one, and the colors of its bar, of its dots and of its edge, in the
/// greys of the window.
const MARGIN: usize = 24;
const BAR: usize = 30;
const RADIUS: usize = 10;
const TITLE: Rgb = Rgb::hex(0x16_1616);
const DOT: Rgb = Rgb::hex(0x3a_3a3a);
const EDGE: Rgb = Rgb::hex(0x2a_2a2a);

impl Image {
  /// The image as a terminal window shows it: in a frame with round corners and a bar of title
  /// with three dots, on a ground that shows through, as a README shows a program, each part of
  /// the frame at the density of the image.
  pub(crate) fn framed(&self) -> Framed {
    let (k, margin) = (self.density, MARGIN * self.density);
    let (inner, tall) = (self.width + 2 * k, self.height + (BAR + 1) * k);
    let (width, height) = (inner + 2 * margin, tall + 2 * margin);
    let mut pixels = vec![0; width * height * 4];
    for y in 0..tall {
      for x in 0..inner {
        let Some(color) = self.framed_at(x, y) else { continue };
        let at = ((y + margin) * width + x + margin) * 4;
        let Rgb([red, green, blue]) = color;
        if let Some(pixel) = pixels.get_mut(at..at + 4) {
          pixel.copy_from_slice(&[red, green, blue, 255]);
        }
      }
    }
    Framed { width, height, pixels }
  }

  /// The color of a pixel of the frame, none outside its round corners: its edge, its bar with its
  /// dots, or the image under the bar.
  fn framed_at(&self, x: usize, y: usize) -> Option<Rgb> {
    let k = self.density;
    let (inner, tall) = (self.width + 2 * k, self.height + (BAR + 1) * k);
    let (radius, bar) = (RADIUS * k, BAR * k);
    let away = |at: usize, size: usize| radius.saturating_sub(at.min(size - 1 - at));
    let (dx, dy) = (away(x, inner), away(y, tall));
    if dx * dx + dy * dy > radius * radius {
      return None;
    }
    let edge = x < k || y < k || x >= inner - k || y >= tall - k;
    let ring = (dx + k) * (dx + k) + (dy + k) * (dy + k) > radius * radius && dx > 0 && dy > 0;
    if edge || ring {
      return Some(EDGE);
    }
    if y < bar {
      let dot = (0..3).any(|n| {
        let (cx, cy) = ((18 + 20 * n) * k, bar / 2);
        let (ex, ey) = (x.abs_diff(cx), y.abs_diff(cy));
        ex * ex + ey * ey <= 36 * k * k
      });
      return Some(if dot { DOT } else { TITLE });
    }
    let (ix, iy) = (x - k, y - bar);
    let at = (iy.min(self.height - 1) * self.width + ix.min(self.width - 1)) * 3;
    match self.pixels.get(at..at + 3) {
      Some(&[red, green, blue]) => Some(Rgb([red, green, blue])),
      _ => None,
    }
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
