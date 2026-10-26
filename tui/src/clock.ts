/** The time that the view shows, in milliseconds since the epoch. FURB_STILL_CLOCK holds it at the time that it names,
 * so that two captures of the same code draw the same frame: no spinner turns, and no time passes for what the view
 * measures, as a step that runs or an answer that took a while. The work of a life keeps the time of the machine. */
const still = Number(process.env.FURB_STILL_CLOCK) || undefined;

export function clock(): number {
  return still ?? Date.now();
}
