import { expect, test } from "bun:test";
import { contrast, palettes, placeholderContrast, terminalPalette } from "../src/theme.ts";

const grounds = ["background", "panel", "raised"] as const;

test("a placeholder reads on each ground of an input, in each theme", () => {
  for (const [name, palette] of Object.entries(palettes)) {
    for (const ground of grounds) {
      expect(contrast(palette.faint, palette[ground]), `${name} on ${ground}`).toBeGreaterThanOrEqual(
        placeholderContrast,
      );
    }
  }
});

test("a placeholder reads on each ground of an input, on the colors of a terminal", () => {
  const terminals = [
    ["#ffffff", "#000000"],
    ["#c5c8c6", "#1d1f21"],
    ["#657b83", "#fdf6e3"],
    ["#000000", "#ffffff"],
    ["#839496", "#002b36"],
  ] as const;
  for (const [foreground, background] of terminals) {
    const palette = terminalPalette(foreground, background);
    expect(palette).toBeDefined();
    for (const ground of grounds) {
      expect(
        contrast(palette?.faint ?? "", palette?.[ground] ?? ""),
        `${foreground} on ${background}, ${ground}`,
      ).toBeGreaterThanOrEqual(placeholderContrast);
    }
  }
});
