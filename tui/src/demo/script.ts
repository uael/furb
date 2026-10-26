/** What the models of the demo write: a word for each step of a prompt, found by what the prompt says. A word that
 * ends with no close ends its step, and the chain asks the model again, so a prompt with four words is four steps of a
 * model that reads, fails, mends and answers. The words run for real, on the project of the demo. */
export interface Step {
  /** What the model thinks before it writes, which a watched life streams. */
  thinking: string;
  /** The word, as Python. */
  word: string;
}

export interface Script {
  /** A part of the message of the prompt that this script answers. */
  cue: string;
  steps: Step[];
  /** The tokens that a step of this script asks, which grow with the chain, and what an answer costs in dollars. */
  weight?: { input: number; output: number; cost: number };
}

/** The text that a transcript holds: the user messages that the engine gave the model, joined. */
export function transcriptOf(messages: readonly unknown[]): string {
  const texts: string[] = [];
  for (const message of messages as { role?: string; content?: unknown }[]) {
    if (message.role !== "user") continue;
    if (typeof message.content === "string") texts.push(message.content);
    else if (Array.isArray(message.content))
      for (const part of message.content as { type?: string; text?: string }[])
        if (part.type === "text" && typeof part.text === "string") texts.push(part.text);
  }
  return texts.join("\n");
}

/** The message of a prompt, as the transcript binds it: in quotes when it is one line, in a quote when it is more. */
export function messageOf(transcript: string, prompt: string): string | undefined {
  // Python writes a string in the quotes that need the fewest escapes, so a message with an apostrophe comes in double ones.
  for (const quote of ["'", '"']) {
    const other = quote === "'" ? "\\'" : '\\"';
    const line = new RegExp(`${prompt}_message = ${quote}((?:[^${quote}\\\\]|\\\\.)*)${quote}`).exec(
      transcript,
    );
    if (line?.[1] !== undefined) return line[1].replaceAll(other, quote).replaceAll("\\n", "\n");
  }
  return new RegExp(`<s:${prompt}_message>\\n([\\s\\S]*?)</s:${prompt}_message>`).exec(transcript)?.[1];
}

/** The script that answers the step that the transcript asks, and the number of that step, from 0: each advance on a
 * prompt in the transcript is a step, so a word that the gate refused takes a step too. */
export function scripted(
  scripts: readonly Script[],
  messages: readonly unknown[],
): { script: Script; step: number } | undefined {
  const transcript = transcriptOf(messages);
  const advances = [...transcript.matchAll(/#rung\d+ advance on (prompt\d+)/g)];
  const prompt = advances.at(-1)?.[1];
  if (!prompt) return undefined;
  const message = messageOf(transcript, prompt);
  const script = message === undefined ? undefined : scripts.find((one) => message.includes(one.cue));
  if (!script) return undefined;
  const step = advances.filter((one) => one[1] === prompt).length - 1;
  return { script, step };
}
