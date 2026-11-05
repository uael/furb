import type { Turn } from "furb";
import { scripts } from "./index.ts";
import { scripted, transcriptOf } from "./script.ts";

/** What a model of the demo is asked: its actor, its chain and its messages, as the daemon gives each request. */
export interface Asked {
  actor: string;
  chain: string;
  messages: unknown[];
}

/** What a model of the demo writes as it writes it. */
export type Write = (part: { text?: string; thinking?: string }) => void;

/** What the demo thinks and answers on a later turn, by what the turn says. */
const replies: [cue: string, thinking: string, answer: string][] = [
  [
    "refused",
    "The gate refused the fence, so the word is Python alone.",
    "Here it is again as plain Python, with no fence around it.",
  ],
  [
    "show live progress",
    "The index is ready, so the answer says what it holds.",
    "The index is built, and three notes are ready to search. Each word streamed into the feed as it ran.",
  ],
  [
    "keyboard navigation",
    "The keys work, so the answer lists them.",
    "Keyboard navigation works: Ctrl+K opens the search, and the arrows move between notes.",
  ],
  [
    "layout",
    "The image shows the welcome, so the answer reads its layout.",
    "The layout reads well. The logo leads, the starters sit under it, and the keys close the column.",
  ],
  [
    " done",
    "The command is done, so the answer says what it found.",
    "The checks finished and all three passed, so the project is ready for the search shortcut.",
  ],
];
/** What each model of the demo answers when a word asks every model what it would change first, by its name. */
const views: [model: string, answer: string][] = [
  ["haiku", "Add a test for a search that finds nothing."],
  ["sonnet", "Save the notes to a file, so they stay after a restart."],
  ["fable", "Give search a shortcut, Ctrl+K, so it is one key away."],
];
function reply(turn: string, actor: string, chain: string): [thinking: string, answer: string] {
  // Three rungs of one chain answer the word, so each reads the question in the turns of its chain.
  const view = chain.includes("change first") && views.find(([name]) => actor.includes(name));
  if (view) return ["One change comes first, so the answer names it.", view[1]];
  const found = replies.find(([cue]) => turn.includes(cue));
  return found
    ? [found[1], found[2]]
    : [
        "The change is small, so the answer says where it is.",
        "Done. The change is small, its checks pass, and the result is in the feed.",
      ];
}

/** The turns that think until a cancel, each with what writes its stream. */
const held = new Map<(turn: Turn) => void, Write>();

/** The thought that each turn that thinks until a cancel writes, as a model streams one. */
export function think(thinking: string): void {
  for (const write of held.values()) write({ thinking });
}

/** The models of the demo, which ask no model and answer each request from the script of the demo. A demo that the
 * operator watches streams each turn, as a model writes it, and each of its answers costs nothing. */
export function demoModels(watch = false): (asked: Asked, write: Write) => Promise<Turn> {
  return async ({ actor, messages }, write) => {
    const pause = (milliseconds: number) => new Promise<void>((resolve) => setTimeout(resolve, milliseconds));
    const last = JSON.stringify(messages.at(-1));
    // A turn whose last message asks to think until a cancel never answers, so its work stays in progress until the
    // operator cancels it, however slow the machine that shows it.
    const message = [...last.matchAll(/prompt\d+_message = '([^']*)'/g)].at(-1)?.[1];
    if (message === "Think until I cancel.")
      return await new Promise<Turn>((resolve) => held.set(resolve, write));
    // Only the turn that asks for live progress is slow, and not every later turn of its chain: it thinks, then
    // writes its word a few words at a time, as a model streams it. FURB_DEMO_STREAM=1 makes every turn so, for a
    // recording of the demo.
    const life = scripted(scripts, messages as Parameters<typeof scripted>[1]);
    // A step that the script does not hold closes the prompt, so that a word that failed never loops.
    const step = life
      ? (life.script.steps[life.step] ?? {
          thinking: "I report what I did.",
          word: 'close("The work is done, and the project holds it.")',
        })
      : undefined;
    const slow = watch || process.env.FURB_DEMO_STREAM === "1" || last.includes("show live progress");
    if (slow) await pause(300);
    // The first answer of the model is the turn of the first advance: a turn that its caller wrote, as the read of
    // the doctrine that the World plays on each chain, is an assistant message that the model did not write.
    const first =
      (transcriptOf(messages as Parameters<typeof transcriptOf>[0]).match(/#rung\d+ advance on /g) ?? [])
        .length <= 1;
    const [thinking, answer] = first
      ? ["I read the README and run the checks before I answer.", ""]
      : reply(last, actor, JSON.stringify(messages));
    // A message that asks for an answer in a fence gets one, as a model sometimes writes it, which the gate refuses,
    // and the model answers again once it reads the refusal.
    const fenced = !first && last.includes("an answer in a fence") && !last.includes("refused");
    // A message that says to ask first gets a question to the operator and no close, so the model answers again
    // once the operator has answered.
    const asks = !first && last.includes("Ask me first");
    // A message that hires workers makes a life as a conductor makes it: a prompt to a worker on each of two or eight
    // chains of its own, each of which thinks until a cancel, under a wait that holds the thread of the message open.
    const hires = /Hire (two|eight) workers/.exec(last)?.[1];
    // A message that adds the search shortcut writes the README, so a rewind to a point before it can give the file
    // back as it was.
    const shortcut = !first && last.includes("Add the search shortcut");
    const fallback = hires
      ? hires === "two"
        ? 'for label in ("w/one", "w/two"):\n  prompt(str, "Think until I cancel.", on=chain(label))\nawait wait(3600, __name__)'
        : 'for n in range(8):\n  prompt(str, "Think until I cancel.", on=chain(f"w/{n}"))\nawait wait(3600, __name__)'
      : shortcut
        ? 'notes = read("README.md")\nwrite(notes.replace("- Find it when it matters", "- Find it when it matters, with Ctrl+K"))\nclose("Ctrl+K opens the search now, and the README says so.")'
        : fenced
          ? '```python\nclose("Here is the answer, in a fence.")\n```'
          : asks
            ? 'await prompt(str, "Which effort should the next step take?", to=OPERATOR)'
            : first
              ? 'notes = read("README.md")\ncheck = await bash("printf \'✓ capture\\n✓ search\\n✓ local storage\\n\'")\nclose("## A clear starting point\\nFieldnotes keeps ideas close. The project has three small parts: capture, search, and local storage.\\n\\nAll three checks passed. A useful next step is to add a **search shortcut**, then cover it with a focused test.")'
              : `close(${JSON.stringify(answer)})`;
    const code = step?.word ?? fallback;
    if (slow) {
      write({ thinking: step?.thinking ?? thinking });
      await pause(200);
      // FURB_DEMO_PACE sets the time between two pieces, in milliseconds, which a capture of work in progress
      // lengthens.
      const pace = Number(process.env.FURB_DEMO_PACE) || 30;
      for (const piece of code.match(/.{1,10}/gs) ?? []) {
        write({ text: piece });
        await pause(pace);
      }
      await pause(200);
    }
    // A scripted prompt asks more tokens at each step, as the chain that it reads grows.
    const weight = life?.script.weight;
    const usage: [number, number, number, number, number] = weight
      ? [weight.input + 1400 * (life?.step ?? 0), weight.output, 2800, 0, watch ? 0 : weight.cost]
      : [3240, 184, 2800, 0, watch ? 0 : 0.0024];
    return ["assistant", code, usage, null];
  };
}
