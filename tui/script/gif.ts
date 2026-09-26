import { GIFEncoder } from "gifenc";

/** One picture of an animation: its pixels packed with deflate, four bytes for each, and how long it stays, in
 * hundredths of a second. A picture stays packed until the encoder reads it, so that a long animation fits in
 * memory. */
export interface Still {
  packed: Uint8Array<ArrayBuffer>;
  delay: number;
}
/** A picture of an animation, packed. */
export function pack(pixels: Uint8Array<ArrayBuffer>, delay: number): Still {
  return { packed: Bun.deflateSync(pixels, { level: 1 }), delay };
}
function unpack(still: Still): Uint8Array {
  return Bun.inflateSync(still.packed);
}

/** The index that marks a pixel as the same as the picture before it. The palette takes all 255 colors that a GIF
 * allows beside that index, so that the edges of the text keep their shades. */
const unchanged = 255;

/** An animated GIF that plays the pictures in a loop. The pictures share one palette of the colors they show most,
 * and each picture after the first leaves the pixels that did not change clear, so that a terminal that changes a few
 * cells costs a few bytes. */
export function gif(stills: Still[], width: number, height: number, kept: number[] = []): Uint8Array {
  const palette = colors(stills, width, height, kept);
  const index = mapper(palette);
  const encoder = GIFEncoder();
  let before: Uint8Array | undefined;
  for (const still of stills) {
    const pixels = unpack(still);
    const now = new Uint8Array(width * height);
    for (let pixel = 0; pixel < now.length; pixel++) now[pixel] = index(color(pixels, pixel));
    const last = before;
    encoder.writeFrame(
      last ? now.map((value, at) => (value === last[at] ? unchanged : value)) : now,
      width,
      height,
      {
        // The first picture carries the palette of all of them.
        palette: last
          ? undefined
          : Array.from({ length: unchanged + 1 }, (_, slot) => {
              const value = palette[slot] ?? 0;
              return [(value >> 16) & 0xff, (value >> 8) & 0xff, value & 0xff];
            }),
        delay: still.delay * 10,
        transparent: Boolean(last),
        transparentIndex: unchanged,
        // Each picture stays in place under the next one.
        dispose: 1,
      },
    );
    before = now;
  }
  encoder.finish();
  return encoder.bytes();
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
  for (const still of stills) {
    const pixels = unpack(still);
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
