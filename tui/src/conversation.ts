import { opens, type Paragraph, paragraphs, type Turn, uncommented } from "@furb/engine";
import { Parts } from "./parts.ts";
import { type ActRow, failed } from "./session.ts";

/** One thing the conversation shows, read off the python of the turns of a chain.
 *
 * - `python`: a word, which an assistant turn holds, or which the open of a rung its caller wrote tells, with the
 *   rung it is the word of.
 * - `prompt`: the open of a prompt, which tells its message.
 * - `result`: the close of a prompt, and whether other prompts closed in the same turn.
 * - `act`: the open or the end of any other act, which the conversation shows once.
 * - `note`: any other paragraph: what an act told of itself after its open, or a query of a run, by its header.
 *   The rungs that the World played stand as one note, `extensions`, which names the extensions the life plays,
 *   and which is `played`.
 */
export type Item =
  | { type: "python"; key: string; code: string; rung?: ActRow }
  | { type: "prompt"; key: string; act: ActRow }
  | { type: "result"; key: string; act: ActRow; parallel: boolean }
  | { type: "act"; key: string; act: ActRow }
  | {
      type: "note";
      key: string;
      label: string;
      detail: string;
      body: string;
      act?: ActRow;
      played?: boolean;
    };

/** What the conversation of a chain shows, in the order of its turns, with the parts of the extensions, which say
 * how the acts of their kinds end, which acts no card shows, and which notes are quiet, and the names of the
 * extensions whose words the World played. */
export function conversation(
  turns: readonly Turn[],
  acts: readonly ActRow[],
  parts = new Parts(),
  played: readonly string[] = [],
): Item[] {
  const rows = new Map(acts.map((act) => [act.id, act]));
  const items: Item[] = [];
  const seen = new Set<string>();
  let rung: ActRow | undefined;
  let told = false;
  for (const [index, [role, python]] of turns.entries()) {
    if (role === "assistant") {
      if (python) items.push({ type: "python", key: `turn-${index}`, code: python, rung });
      if (rung) seen.add(rung.id);
      continue;
    }
    const said = paragraphs(python);
    const closes = said.filter(
      (paragraph) => rows.get(paragraph.name)?.kind === "prompt" && paragraph.words.startsWith("closed"),
    ).length;
    for (const [part, paragraph] of said.entries()) {
      const key = `turn-${index}-${part}`;
      const act = rows.get(paragraph.name);
      const [word = "", ...rest] = paragraph.words.split(" ");
      const shown = () => items.push(note(key, paragraph, act, word, rest.join(" ")));
      if (parts.quiet.has(word)) continue;
      if (act?.kind === "rung" && act.by === "world") {
        // The words of the extensions are the World's, and one line names them all, the first time they are told.
        if (!told)
          items.push({
            type: "note",
            key,
            label: "extensions",
            detail: played.join(", "),
            body: "",
            played: true,
          });
        told = true;
        seen.add(act.id);
      } else if (act?.kind === "rung") {
        if (word === "advance") rung = act;
        else if (!paragraph.words) {
          // The open of a rung its caller wrote: its header, and then that word.
          rung = act;
          seen.add(act.id);
          items.push({ type: "python", key, code: paragraph.lines.join("\n"), rung: act });
        } else if (word === "closed") {
          if (failed(act) && !seen.has(act.id)) items.push({ type: "act", key: act.id, act });
          seen.add(act.id);
        } else if (!(["raised", "refused"].includes(word) && failed(act) && seen.has(act.id))) shown();
      } else if (act?.kind === "prompt") {
        if (opens(paragraph)) items.push({ type: "prompt", key, act });
        else if (word === "closed") items.push({ type: "result", key, act, parallel: closes > 1 });
        else shown();
      } else if (act && parts.hidden(act)) {
        // The inspector shows what the chain stands on, so its standing, headed by its roster, is no card of the
        // conversation.
        if (!opens(paragraph) && !parts.ends(act.kind).includes(word) && word !== "roster") shown();
      } else if (act && (opens(paragraph) || parts.ends(act.kind).includes(word))) {
        if (!seen.has(act.id)) items.push({ type: "act", key: act.id, act });
        seen.add(act.id);
      } else shown();
    }
  }
  return items;
}

/** A paragraph as a note: the word of its header after the name of an act, or the kind of its query, as its label. */
function note(key: string, paragraph: Paragraph, act: ActRow | undefined, word: string, rest: string): Item {
  return act
    ? { type: "note", key, label: word, detail: rest, body: uncommented(paragraph.lines), act }
    : {
        type: "note",
        key,
        label: paragraph.name,
        detail: paragraph.words,
        body: uncommented(paragraph.lines),
      };
}

/** The findings that refused the word of a rung, one for each comment of the paragraph its chain told of it. */
export function refusal(turns: readonly Turn[], rung: string): string[] {
  const told = turns
    .flatMap(([role, python]) => (role === "user" ? paragraphs(python) : []))
    .find((paragraph) => paragraph.name === rung && paragraph.words === "refused");
  return told ? uncommented(told.lines).split("\n") : [];
}
