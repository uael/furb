/** One picture of an animation: four bytes for each pixel, and how long it stays, in hundredths of a second. */
export interface Still {
  pixels: Uint8Array;
  delay: number;
}

/** The bits of an index, the colors of the palette, and the index that marks a pixel as the same as the picture
 * before it. The palette takes all 255 colors that a GIF allows beside that index, so that the edges of the text keep
 * their shades. */
const bits = 8,
  size = 1 << bits,
  unchanged = size - 1;

/** An animated GIF that plays the pictures in a loop. The pictures share one palette of the colors they show most,
 * and each picture after the first keeps only the rectangle where it differs from the one before it, with the pixels
 * that did not change left clear, so that a terminal that changes a few cells costs a few bytes. */
export function gif(stills: Still[], width: number, height: number, kept: number[] = []): Uint8Array {
  const palette = colors(stills, width, height, kept);
  const index = mapper(palette);
  const out: number[] = [];
  const word = (value: number) => out.push(value & 0xff, (value >> 8) & 0xff);
  out.push(...Buffer.from("GIF89a"));
  word(width);
  word(height);
  // A global table of the colors, eight bits of color resolution, and the first color behind.
  out.push(0xf0 | (bits - 1), 0, 0);
  for (let slot = 0; slot < size; slot++) {
    const color = palette[slot] ?? 0;
    out.push((color >> 16) & 0xff, (color >> 8) & 0xff, color & 0xff);
  }
  // The extension that makes the animation loop with no end.
  out.push(0x21, 0xff, 0x0b, ...Buffer.from("NETSCAPE2.0"), 0x03, 0x01, 0x00, 0x00, 0x00);
  let before: Uint8Array | undefined;
  for (const [position, still] of stills.entries()) {
    const now = new Uint8Array(width * height);
    for (let pixel = 0; pixel < now.length; pixel++) now[pixel] = index(color(still.pixels, pixel));
    // The rectangle where this picture differs from the one before it, and the whole picture for the first.
    let left = 0,
      top = 0,
      right = width - 1,
      bottom = height - 1;
    if (before) {
      left = width;
      top = height;
      right = -1;
      bottom = -1;
      for (let y = 0; y < height; y++)
        for (let x = 0; x < width; x++)
          if (now[y * width + x] !== before[y * width + x]) {
            left = Math.min(left, x);
            right = Math.max(right, x);
            top = Math.min(top, y);
            bottom = Math.max(bottom, y);
          }
      // A picture that changes nothing still takes its time, as one clear pixel.
      if (right < 0) left = right = top = bottom = 0;
    }
    const w = right - left + 1,
      h = bottom - top + 1;
    const cells = new Uint8Array(w * h);
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const at = (top + y) * width + left + x;
        cells[y * w + x] = before && now[at] === before[at] ? unchanged : (now[at] ?? 0);
      }
    // The control of the picture: it stays in place under the next one, for its time, with the clear index.
    out.push(0x21, 0xf9, 0x04, (1 << 2) | (position ? 1 : 0));
    word(still.delay);
    out.push(unchanged, 0);
    out.push(0x2c);
    word(left);
    word(top);
    word(w);
    word(h);
    out.push(0);
    out.push(bits);
    const data = lzw(cells);
    for (let at = 0; at < data.length; at += 255) {
      const block = data.subarray(at, at + 255);
      out.push(block.length, ...block);
    }
    out.push(0);
    before = now;
  }
  out.push(0x3b);
  return new Uint8Array(out);
}

/** The color of a pixel as one number: its red, green, and blue. */
function color(pixels: Uint8Array, pixel: number): number {
  return (
    ((pixels[pixel * 4] ?? 0) << 16) | ((pixels[pixel * 4 + 1] ?? 0) << 8) | (pixels[pixel * 4 + 2] ?? 0)
  );
}

/** The colors that the pictures show most, counted over the first picture and what each later one changes, after
 * the colors that are kept whatever their count. */
function colors(stills: Still[], width: number, height: number, kept: number[]): number[] {
  const counts = new Map<number, number>();
  let before: Uint8Array | undefined;
  for (const { pixels } of stills) {
    for (let pixel = 0; pixel < width * height; pixel++) {
      const value = color(pixels, pixel);
      if (before && color(before, pixel) === value) continue;
      counts.set(value, (counts.get(value) ?? 0) + 1);
    }
    before = pixels;
  }
  const common = [...counts]
    .sort((one, other) => other[1] - one[1])
    .map(([value]) => value)
    .filter((value) => !kept.includes(value));
  return [...kept, ...common].slice(0, unchanged);
}

/** The index in the palette of the color nearest each color, which it reads once for each color. */
function mapper(palette: number[]): (value: number) => number {
  const known = new Map<number, number>();
  return (value) => {
    const found = known.get(value);
    if (found !== undefined) return found;
    let best = 0,
      distance = Number.POSITIVE_INFINITY;
    for (const [slot, candidate] of palette.entries()) {
      const red = ((value >> 16) & 0xff) - ((candidate >> 16) & 0xff),
        green = ((value >> 8) & 0xff) - ((candidate >> 8) & 0xff),
        blue = (value & 0xff) - (candidate & 0xff);
      const far = 3 * red * red + 4 * green * green + 2 * blue * blue;
      if (far < distance) {
        best = slot;
        distance = far;
      }
    }
    known.set(value, best);
    return best;
  };
}

/** The indices of a picture compressed as the LZW of GIF, with a code a bit longer than an index at least. */
function lzw(indices: Uint8Array): Uint8Array {
  const clear = size,
    end = size + 1;
  const out: number[] = [];
  let width = bits + 1,
    next = end + 1,
    pending = 0,
    count = 0;
  const emit = (code: number) => {
    pending |= code << count;
    count += width;
    while (count >= 8) {
      out.push(pending & 0xff);
      pending >>>= 8;
      count -= 8;
    }
  };
  let table = new Map<number, number>();
  emit(clear);
  let prefix = indices[0] ?? 0;
  for (let at = 1; at < indices.length; at++) {
    const symbol = indices[at] ?? 0;
    const key = (prefix << bits) | symbol;
    const code = table.get(key);
    if (code !== undefined) {
      prefix = code;
      continue;
    }
    emit(prefix);
    // A full table starts again from a clear code, and a table that outgrows its codes widens them by a bit.
    if (next === 4096) {
      emit(clear);
      table = new Map();
      next = end + 1;
      width = bits + 1;
    } else {
      if (next >= 1 << width) width++;
      table.set(key, next++);
    }
    prefix = symbol;
  }
  emit(prefix);
  emit(end);
  if (count > 0) out.push(pending & 0xff);
  return new Uint8Array(out);
}
