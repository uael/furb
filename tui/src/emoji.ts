import { gemoji } from "gemoji";

/** An emoji of GitHub, under the shortcode that a filter matched. */
export interface Emoji {
  emoji: string;
  name: string;
  description: string;
}

/** The emoji of each shortcode of GitHub. */
const named = new Map(gemoji.flatMap(({ emoji, names }) => names.map((name) => [name, emoji] as const)));

/** The emoji that a whole shortcode names, if one does. */
export function emojiNamed(name: string): string | undefined {
  return named.get(name.toLowerCase());
}

/** The emojis with a shortcode that holds the text, each once: the one whose shortcode is the text first, then those
 * with a shortcode that starts with it, then the others, each in the order of GitHub. */
export function emojis(text: string): Emoji[] {
  const wanted = text.toLowerCase();
  const ranked: [number, Emoji][] = [];
  for (const { emoji, names, description } of gemoji) {
    const name =
      names.find((one) => one === wanted) ??
      names.find((one) => one.startsWith(wanted)) ??
      names.find((one) => one.includes(wanted));
    if (name === undefined) continue;
    const rank = name === wanted ? 0 : name.startsWith(wanted) ? 1 : 2;
    ranked.push([rank, { emoji, name, description }]);
  }
  return ranked.sort(([one], [other]) => one - other).map(([, found]) => found);
}
