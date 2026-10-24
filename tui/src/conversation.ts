import { bound, opens, type Paragraph, paragraphs, plain, type Turn } from "furb";
import { type ActRow, failed } from "./session.ts";

/** One thing the conversation shows, read off the python of the turns of a chain.
 *
 * - `python`: a word, which an assistant turn holds, or which the open of a rung its caller wrote binds as rungN_word,
 *   with the rung it is the word of.
 * - `prompt`: the open of a prompt, which tells its message.
 * - `result`: the close of a prompt, and whether other prompts closed in the same turn.
 * - `act`: the open or the end of any other act, which the conversation shows once.
 * - `note`: any other paragraph: what an act told of itself after its open, or a query of a run, by its header.
 */
export type Item =
  | { type: "python"; key: string; code: string; rung?: ActRow }
  | { type: "prompt"; key: string; act: ActRow }
  | { type: "result"; key: string; act: ActRow; parallel: boolean }
  | { type: "act"; key: string; act: ActRow }
  | { type: "note"; key: string; label: string; detail: string; body: string; act?: ActRow };

/** The headers that tell how an act ended, which the card of the act shows as its state. */
const ENDS = ["closed", "exited", "cancelled"];

/** What the conversation of a chain shows, in the order of its turns, with each act by its name. A read of the turns
 * after a turn starts from the place of that turn and the rung that the turns before it advanced on, and says the rung
 * that its own turns end on in from. */
export function conversation(
  turns: readonly Turn[],
  actOf: (id: string) => ActRow | undefined,
  from: { at?: number; rung?: ActRow } = {},
): Item[] {
  const rows = { get: actOf };
  const items: Item[] = [];
  const seen = new Set<string>();
  let rung = from.rung;
  for (const [place, [role, python]] of turns.entries()) {
    const index = (from.at ?? 0) + place;
    if (role === "assistant") {
      if (python) items.push({ type: "python", key: `turn-${index}`, code: python, rung });
      if (rung) seen.add(rung.id);
      continue;
    }
    const told = paragraphs(python);
    const closes = told.filter(
      (paragraph) => rows.get(paragraph.name)?.kind === "prompt" && paragraph.words.startsWith("closed"),
    ).length;
    for (const [part, paragraph] of told.entries()) {
      const key = `turn-${index}-${part}`;
      const act = rows.get(paragraph.name);
      const [word = "", ...rest] = paragraph.words.split(" ");
      const shown = () => items.push(note(key, paragraph, act, word, rest.join(" ")));
      if (word === "ledger") continue;
      if (act?.kind === "rung") {
        if (writes(act, paragraph)) {
          rung = act;
          seen.add(act.id);
          items.push({
            type: "python",
            key,
            code: String(act.words[0]),
            rung: act,
          });
        } else if (word === "advance") rung = act;
        else if (word === "closed") {
          if (failed(act) && !seen.has(act.id)) items.push({ type: "act", key: act.id, act });
          seen.add(act.id);
        } else if (!(["raised", "refused"].includes(word) && failed(act) && seen.has(act.id))) shown();
      } else if (act?.kind === "prompt") {
        if (opens(paragraph)) items.push({ type: "prompt", key, act });
        else if (word === "closed") items.push({ type: "result", key, act, parallel: closes > 1 });
        else shown();
      } else if (act && ["chain", "grant"].includes(act.kind)) {
        // The inspector shows what the chain stands on, so its standing, headed by its roster, is no card of the
        // conversation.
        if (!opens(paragraph) && !ENDS.includes(word) && word !== "standing") shown();
      } else if (act && (opens(paragraph) || ENDS.includes(word))) {
        if (!seen.has(act.id)) items.push({ type: "act", key: act.id, act });
        seen.add(act.id);
      } else shown();
    }
  }
  from.rung = rung;
  return items;
}

/** Whether a paragraph is the open of a rung its caller wrote, which binds that word as rungN_word under a header
 * that names its maker alone. */
function writes(act: ActRow, paragraph: Paragraph): boolean {
  return (
    Boolean(act.words[0]) &&
    paragraph.words === `by ${act.by}` &&
    bound(paragraph.lines, `${act.id}_word`) === act.words[0]
  );
}

/** A paragraph as a note: the word of its header after the name of an act, or the kind of that act when its header
 * says nothing more, as its label. */
function note(key: string, paragraph: Paragraph, act: ActRow | undefined, word: string, rest: string): Item {
  return act
    ? { type: "note", key, label: word || act.kind, detail: rest, body: plain(paragraph.lines), act }
    : {
        type: "note",
        key,
        label: paragraph.name,
        detail: paragraph.words,
        body: plain(paragraph.lines),
      };
}
